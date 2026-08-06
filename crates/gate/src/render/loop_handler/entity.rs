use glam::{Mat4, Vec3, Vec2};
use pystral_core::log::{EntityState, PropertyValue, WorldState};
use pystral_core::domain::{Shape3D, Joint};
use web_sys::WebGlRenderingContext as GL;
use crate::render::context::RenderContext;
use crate::render::draw_utils::{set_model_matrix, set_material};
use crate::render::utils::{EntityExt, RenderResultExt};

#[allow(clippy::too_many_lines)]
pub fn draw_parts(ctx: &mut RenderContext, worker_tx: &futures::channel::mpsc::UnboundedSender<crate::WorkerInput>, entity: &EntityState, state: &WorldState, sprite_pos: Vec3, entity_pos: Vec3, billboard_rot: Mat4, entity_scale: f32, cam_right: Vec3, billboard_up: Vec3, cam_forward: Vec3, side: crate::render::painter::ViewSide, render_rotation_z: f32, rotation_y: f32) {
    let parts = entity.get_sprite_parts().log_fallback(worker_tx);
    let cos_z = render_rotation_z.cos();
    let sin_z = render_rotation_z.sin();

    for (i, part) in parts.iter().enumerate() {
        let jx = entity.get_float(&part.x_prop, 0.0).log_fallback(worker_tx);
        let jy = entity.get_float(&part.y_prop, 0.0).log_fallback(worker_tx);
        let jz = entity.get_float(&part.z_prop, 0.0).log_fallback(worker_tx);
        
        let rx = jx * cos_z - jy * sin_z;
        let ry = jx * sin_z + jy * cos_z;
        let rz = jz;

        let prot = if let Some(rot_prop) = &part.rotation_prop {
            -entity.get_float(rot_prop, 0.0).log_fallback(worker_tx)
        } else {
            0.0
        };
        
        let mut render_prot = prot;
        if side == crate::render::painter::ViewSide::Mirrored {
            render_prot = -prot;
        }
        
        let engine_offset = Vec3::new(rx, rz, ry);
        let bx = engine_offset.dot(cam_right);
        let by = engine_offset.dot(billboard_up);
        let bz = engine_offset.dot(cam_forward);

        let part_pos = sprite_pos + cam_right * (bx * entity_scale) + billboard_up * (by * entity_scale) + cam_forward * (bz * entity_scale - i as f32 * 0.001);
        let part_model = Mat4::from_translation(part_pos + billboard_up * (part.scale * entity_scale * 0.5)) * billboard_rot * Mat4::from_rotation_z(render_rotation_z + render_prot) * Mat4::from_scale(Vec3::splat(part.scale * entity_scale));
        set_model_matrix(&ctx.gl, &ctx.uniforms, &part_model);
        
        let mut use_tex = false;
        if !part.painter_commands.is_empty() {
            let cache_key = (entity.id, i);
            let needs_update = if let Some((old_cmds, _)) = ctx.sprite_part_textures.get(&cache_key) {
                old_cmds != &part.painter_commands
            } else {
                true
            };
            
            if needs_update {
                let front_tex = crate::render::painter::render_commands_to_texture(&ctx.gl, &part.painter_commands, 256, 256, crate::render::painter::ViewSide::Front);
                let mirrored_tex = crate::render::painter::render_commands_to_texture(&ctx.gl, &part.painter_commands, 256, 256, crate::render::painter::ViewSide::Mirrored);
                
                if let (Some(f), Some(m)) = (front_tex, mirrored_tex)
                    && let Some((_, old_set)) = ctx.sprite_part_textures.insert(cache_key, (part.painter_commands.clone(), crate::render::context::TextureSet { front: f, mirrored: m })) {
                    ctx.gl.delete_texture(Some(&old_set.front));
                    ctx.gl.delete_texture(Some(&old_set.mirrored));
                }
            }
            
            if let Some((_, tex_set)) = ctx.sprite_part_textures.get(&cache_key) {
                let tex = if side == crate::render::painter::ViewSide::Front { &tex_set.front } else { &tex_set.mirrored };
                ctx.gl.active_texture(GL::TEXTURE0);
                ctx.gl.bind_texture(GL::TEXTURE_2D, Some(tex));
                ctx.gl.uniform1i(ctx.uniforms.texture.as_ref(), 0);
                use_tex = true;
            }
        }

        ctx.gl.uniform1i(ctx.uniforms.use_tex.as_ref(), use_tex as i32);
        ctx.gl.uniform3f(ctx.uniforms.obj_color.as_ref(), part.color[0], part.color[1], part.color[2]);
        
        if let Some((collection_name, asset_name)) = &part.spritestack {
            let cache_key = format!("{}:{}", collection_name, asset_name);
            
            if !ctx.asset_collection_cache.contains_key(collection_name) && let Some(data) = state.asset_collections.get(collection_name) {
                let collection = pystral_compiler::assets::AssetCollection::from_binary(data);
                ctx.asset_collection_cache.insert(collection_name.clone(), collection);
            }

            if !ctx.spritestack_assets.contains_key(&cache_key)
                && let Some(collection) = ctx.asset_collection_cache.get(collection_name)
                && let Some(stack) = collection.spritestacks.get(asset_name) {
                let mut color_textures = Vec::new();
                let mut normal_textures = Vec::new();
                for slice in &stack.slices {
                    color_textures.push(create_texture(&ctx.gl, stack.width, stack.height, &slice.color_data));
                    normal_textures.push(create_texture(&ctx.gl, stack.width, stack.height, &slice.normal_data));
                }
                ctx.spritestack_assets.insert(cache_key.clone(), crate::render::context::SpritestackTextures {
                    color_textures,
                    normal_textures,
                    width: stack.width,
                    height: stack.height,
                    spacing: stack.spacing,
                });
            }

            if let Some(textures) = ctx.spritestack_assets.get(&cache_key) {
                let spacing = textures.spacing;
                let quad_scale = (textures.width as f32 - 0.5) * spacing * part.scale * entity_scale;
                
                for (i, (color_tex, normal_tex)) in textures.color_textures.iter().zip(textures.normal_textures.iter()).enumerate() {
                    let z_offset = (i as f32 - textures.color_textures.len() as f32 * 0.5) * spacing * entity_scale;
                    
                    let engine_world_offset = Vec3::new(rx, rz, ry) * entity_scale;
                    let spritestack_part_pos = entity_pos + engine_world_offset + Vec3::Y * z_offset;

                    let slice_model = Mat4::from_translation(spritestack_part_pos) 
                        * Mat4::from_rotation_y(rotation_y + render_prot)
                        * Mat4::from_rotation_x(-std::f32::consts::FRAC_PI_2)
                        * Mat4::from_scale(Vec3::splat(quad_scale));
                    set_model_matrix(&ctx.gl, &ctx.uniforms, &slice_model);
                    
                    ctx.gl.active_texture(GL::TEXTURE0);
                    ctx.gl.bind_texture(GL::TEXTURE_2D, Some(color_tex));
                    ctx.gl.uniform1i(ctx.uniforms.texture.as_ref(), 0);
                    ctx.gl.active_texture(GL::TEXTURE1);
                    ctx.gl.bind_texture(GL::TEXTURE_2D, Some(normal_tex));
                    ctx.gl.uniform1i(ctx.uniforms.normal_map.as_ref(), 1);
                    ctx.gl.uniform1i(ctx.uniforms.use_tex.as_ref(), 1);
                    ctx.gl.uniform1i(ctx.uniforms.use_normal_map.as_ref(), 1);
                    ctx.sprite_mesh.draw(&ctx.gl, ctx.attribs.pos, ctx.attribs.norm, ctx.attribs.uv);
                }
                ctx.gl.uniform1i(ctx.uniforms.use_normal_map.as_ref(), 0);
            }
        } else {
            ctx.sprite_mesh.draw(&ctx.gl, ctx.attribs.pos, ctx.attribs.norm, ctx.attribs.uv);
        }
    }
}

pub fn draw_skeleton(ctx: &mut RenderContext, worker_tx: &futures::channel::mpsc::UnboundedSender<crate::WorkerInput>, entity: &EntityState, sprite_pos: Vec3, entity_scale: f32, cam_right: Vec3, billboard_up: Vec3, cam_forward: Vec3, skeleton: &pystral_core::domain::Skeleton, debug_mode: bool, side: crate::render::painter::ViewSide, render_rotation_z: f32, _rotation_y: f32) {
    set_material(&ctx.gl, &ctx.uniforms, [1.0, 1.0, 1.0], 0.0, 0.0, 1.0);
    
    if debug_mode {
        ctx.gl.disable(GL::DEPTH_TEST);
    }

    let cos_z = render_rotation_z.cos();
    let sin_z = render_rotation_z.sin();

    let get_joint_billboard_pos = |joint: &Joint| -> (f32, f32, f32) {
        let (jx, jy, jz) = match joint {
            Joint::Constant(x, y, z) => (*x, *y, *z),
            Joint::Property(prop) => {
                let px = format!("{}_x", prop);
                let py = format!("{}_y", prop);
                let pz = format!("{}_z", prop);
                (
                    entity.get_float(&px, 0.0).log_fallback(worker_tx),
                    entity.get_float(&py, 0.0).log_fallback(worker_tx),
                    entity.get_float(&pz, 0.0).log_fallback(worker_tx),
                )
            }
        };
        
        let rx = jx * cos_z - jy * sin_z;
        let ry = jx * sin_z + jy * cos_z;
        let rz = jz;

        let engine_offset = Vec3::new(rx, rz, ry);
        (
            engine_offset.dot(cam_right),
            engine_offset.dot(billboard_up),
            engine_offset.dot(cam_forward),
        )
    };

    for (i, bone) in skeleton.bones.iter().enumerate() {
        let (sx, sy, sz) = get_joint_billboard_pos(&bone.start);
        let (ex, ey, ez) = get_joint_billboard_pos(&bone.end);
        let start = sprite_pos + cam_right * (sx * entity_scale) + billboard_up * (sy * entity_scale) + cam_forward * (sz * entity_scale);
        let end = sprite_pos + cam_right * (ex * entity_scale) + billboard_up * (ey * entity_scale) + cam_forward * (ez * entity_scale);
        let dir = end - start;
        let len = dir.length();
        
        if !bone.painter_commands.is_empty() && len > 0.001 {
            let center = (start + end) * 0.5;
            let up = dir.normalize();
            let forward = -cam_forward;
            let right = up.cross(forward).normalize();
            
            let bone_rot = Mat4::from_cols(right.extend(0.0), up.extend(0.0), forward.extend(0.0), glam::Vec4::W);
            
            let cache_key = (entity.id, i);
            let needs_update = if let Some((old_cmds, _)) = ctx.bone_textures.get(&cache_key) {
                old_cmds != &bone.painter_commands
            } else {
                true
            };
            
            if needs_update {
                let front_tex = crate::render::painter::render_commands_to_texture(&ctx.gl, &bone.painter_commands, 256, 256, crate::render::painter::ViewSide::Front);
                let mirrored_tex = crate::render::painter::render_commands_to_texture(&ctx.gl, &bone.painter_commands, 256, 256, crate::render::painter::ViewSide::Mirrored);
                
                if let (Some(f), Some(m)) = (front_tex, mirrored_tex)
                    && let Some((_, old_set)) = ctx.bone_textures.insert(cache_key, (bone.painter_commands.clone(), crate::render::context::TextureSet { front: f, mirrored: m })) {
                    ctx.gl.delete_texture(Some(&old_set.front));
                    ctx.gl.delete_texture(Some(&old_set.mirrored));
                }
            }
            
            if let Some((_, tex_set)) = ctx.bone_textures.get(&cache_key) {
                let tex = if side == crate::render::painter::ViewSide::Front { &tex_set.front } else { &tex_set.mirrored };
                ctx.gl.active_texture(GL::TEXTURE0);
                ctx.gl.bind_texture(GL::TEXTURE_2D, Some(tex));
                ctx.gl.uniform1i(ctx.uniforms.texture.as_ref(), 0);
                ctx.gl.uniform1i(ctx.uniforms.use_tex.as_ref(), 1);
                
                let offset_center = center + cam_forward * 0.01;
                let bone_width = 0.05 * entity_scale;
                let bone_model_offset = Mat4::from_translation(offset_center) * bone_rot * Mat4::from_scale(Vec3::new(bone_width, len, 1.0));
                set_model_matrix(&ctx.gl, &ctx.uniforms, &bone_model_offset);
                
                ctx.sprite_mesh.draw(&ctx.gl, ctx.attribs.pos, ctx.attribs.norm, ctx.attribs.uv);
            }
        } else if debug_mode && len > 0.001 {
            let rotation = glam::Quat::from_rotation_arc(Vec3::Z, dir / len);
            let bone_model = Mat4::from_translation(start + dir * 0.5) * Mat4::from_quat(rotation) * Mat4::from_scale(Vec3::new(0.02 * entity_scale, 0.02 * entity_scale, len));
            set_model_matrix(&ctx.gl, &ctx.uniforms, &bone_model);
            ctx.gl.uniform1i(ctx.uniforms.use_tex.as_ref(), 0);
            ctx.cylinder_mesh.draw_wireframe(&ctx.gl, ctx.attribs.pos);
        }
    }
    
    if debug_mode {
        ctx.gl.enable(GL::DEPTH_TEST);
    }
}

pub fn draw_spritestack(ctx: &mut RenderContext, entity: &EntityState, state: &WorldState, sprite_pos: Vec3, _billboard_rot: Mat4, entity_scale: f32, _cam_forward: Vec3, yaw: f32) {
    if let Some(PropertyValue::AssetRef(collection_name)) = entity.properties.get("spritestack_collection") {
        let Some(PropertyValue::String(asset_name)) = entity.properties.get("spritestack_name") else {
            return;
        };

        let cache_key = format!("{}:{}", collection_name, asset_name);
        
        if !ctx.asset_collection_cache.contains_key(collection_name) && let Some(data) = state.asset_collections.get(collection_name) {
            let collection = pystral_compiler::assets::AssetCollection::from_binary(data);
            ctx.asset_collection_cache.insert(collection_name.clone(), collection);
        }

        if !ctx.spritestack_assets.contains_key(&cache_key)
            && let Some(collection) = ctx.asset_collection_cache.get(collection_name)
            && let Some(stack) = collection.spritestacks.get(asset_name) {
            let mut color_textures = Vec::new();
            let mut normal_textures = Vec::new();

            for slice in &stack.slices {
                color_textures.push(create_texture(&ctx.gl, stack.width, stack.height, &slice.color_data));
                normal_textures.push(create_texture(&ctx.gl, stack.width, stack.height, &slice.normal_data));
            }

            ctx.spritestack_assets.insert(cache_key.clone(), crate::render::context::SpritestackTextures {
                color_textures,
                normal_textures,
                width: stack.width,
                height: stack.height,
                spacing: stack.spacing,
            });
        }

        if let Some(textures) = ctx.spritestack_assets.get(&cache_key) {
            let spacing = textures.spacing;
            let quad_scale = (textures.width as f32 - 0.5) * spacing * entity_scale;
            
            for (i, (color_tex, normal_tex)) in textures.color_textures.iter().zip(textures.normal_textures.iter()).enumerate() {
                let z_offset = (i as f32 - textures.color_textures.len() as f32 * 0.5) * spacing * entity_scale;
                let slice_pos = sprite_pos + Vec3::Y * z_offset;
                
                let slice_model = Mat4::from_translation(slice_pos) 
                    * Mat4::from_rotation_y(yaw)
                    * Mat4::from_rotation_x(-std::f32::consts::FRAC_PI_2)
                    * Mat4::from_scale(Vec3::splat(quad_scale));
                
                set_model_matrix(&ctx.gl, &ctx.uniforms, &slice_model);
                
                ctx.gl.active_texture(GL::TEXTURE0);
                ctx.gl.bind_texture(GL::TEXTURE_2D, Some(color_tex));
                ctx.gl.uniform1i(ctx.uniforms.texture.as_ref(), 0);
                
                ctx.gl.active_texture(GL::TEXTURE1);
                ctx.gl.bind_texture(GL::TEXTURE_2D, Some(normal_tex));
                ctx.gl.uniform1i(ctx.uniforms.normal_map.as_ref(), 1);
                
                ctx.gl.uniform1i(ctx.uniforms.use_tex.as_ref(), 1);
                ctx.gl.uniform1i(ctx.uniforms.use_normal_map.as_ref(), 1);
                
                ctx.sprite_mesh.draw(&ctx.gl, ctx.attribs.pos, ctx.attribs.norm, ctx.attribs.uv);
            }
            ctx.gl.uniform1i(ctx.uniforms.use_normal_map.as_ref(), 0);
        }
    }
}

pub fn create_texture(gl: &GL, width: u32, height: u32, data: &[u8]) -> web_sys::WebGlTexture {
    let texture = gl.create_texture().expect("Failed to create texture");
    gl.bind_texture(GL::TEXTURE_2D, Some(&texture));
    #[allow(clippy::cast_possible_wrap)]
    gl.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
        GL::TEXTURE_2D, 0, GL::RGBA as i32, width as i32, height as i32, 0, GL::RGBA, GL::UNSIGNED_BYTE, Some(data)
    ).expect("Failed to upload texture data");
    #[allow(clippy::cast_possible_wrap)]
    gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_MIN_FILTER, GL::NEAREST as i32);
    #[allow(clippy::cast_possible_wrap)]
    gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_MAG_FILTER, GL::NEAREST as i32);
    #[allow(clippy::cast_possible_wrap)]
    gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_WRAP_S, GL::CLAMP_TO_EDGE as i32);
    #[allow(clippy::cast_possible_wrap)]
    gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_WRAP_T, GL::CLAMP_TO_EDGE as i32);
    texture
}

pub fn draw_collision(ctx: &RenderContext, sprite_pos: Vec3, rotation_z: f32, shape: &Shape3D) {
    set_material(&ctx.gl, &ctx.uniforms, [1.0, 1.0, 0.0], 0.0, 0.0, 1.0);
    ctx.gl.disable(GL::CULL_FACE);
    ctx.gl.disable(GL::DEPTH_TEST);

    let model_rot = Mat4::from_rotation_z(rotation_z);

    match shape {
        Shape3D::Capsule(capsule) => {
            let cyl_model = Mat4::from_translation(sprite_pos) * model_rot * Mat4::from_scale(Vec3::new(capsule.radius, capsule.radius, capsule.height));
            set_model_matrix(&ctx.gl, &ctx.uniforms, &cyl_model);
            ctx.cylinder_mesh.draw_wireframe(&ctx.gl, ctx.attribs.pos);
            
            let sphere_model_bottom = Mat4::from_translation(sprite_pos) * Mat4::from_scale(Vec3::splat(capsule.radius));
            set_model_matrix(&ctx.gl, &ctx.uniforms, &sphere_model_bottom);
            ctx.sphere_mesh.draw_wireframe(&ctx.gl, ctx.attribs.pos);
            
            let sphere_model_top = Mat4::from_translation(sprite_pos + model_rot.transform_vector3(Vec3::new(0.0, 0.0, capsule.height))) * Mat4::from_scale(Vec3::splat(capsule.radius));
            set_model_matrix(&ctx.gl, &ctx.uniforms, &sphere_model_top);
            ctx.sphere_mesh.draw_wireframe(&ctx.gl, ctx.attribs.pos);
        }
        Shape3D::Cube(size) => {
            let cube_model = Mat4::from_translation(sprite_pos) * model_rot * Mat4::from_scale(Vec3::splat(*size));
            set_model_matrix(&ctx.gl, &ctx.uniforms, &cube_model);
            ctx.sprite_mesh.draw_wireframe(&ctx.gl, ctx.attribs.pos);
        }
    }
    ctx.gl.enable(GL::DEPTH_TEST);
    ctx.gl.enable(GL::CULL_FACE);
}
