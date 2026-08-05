use glam::{Mat4, Vec3};
use pystral_core::log::{WorldState, EntityState, PropertyValue};
use pystral_core::domain::{Shape3D, Joint};
use web_sys::WebGlRenderingContext as GL;
use hexx::ColumnMeshBuilder;
use crate::render::context::RenderContext;
use crate::render::mesh::Mesh;
use crate::render::draw_utils::{set_model_matrix, set_material};
use crate::render::utils::{EntityExt, RenderResultExt};

pub fn draw_scene(ctx: &mut RenderContext, state: WorldState, cam_right: Vec3, cam_up: Vec3, cam_forward: Vec3, debug_mode: bool, now: f64, _is_playing_anims: bool) {
    apply_lighting(ctx, &state);

    let layout = get_layout(&state);

    if ctx.unit_hex_mesh_cache.is_none() {
        let unit_hex_info = ColumnMeshBuilder::new(&layout, 1.0).center_aligned().without_bottom_face().build();
        ctx.unit_hex_mesh_cache = Some(Mesh::from_mesh_info(&ctx.gl, &unit_hex_info));
    }
    let unit_hex_mesh = ctx.unit_hex_mesh_cache.as_ref().unwrap();

    let mut current_map = None;
    if let Some(world) = state.entities.iter().find(|e| e.id == 0) {
        current_map = Some(world.get_hex_map().log_fallback());
    }

    if let Some(ref map) = current_map {
        for tile in &map.tiles {
            let world_pos = layout.hex_to_world_pos(tile.hex);
            let hex_model = Mat4::from_translation(Vec3::new(world_pos.x, tile.bottom, world_pos.y))
                * Mat4::from_scale(Vec3::new(1.0, tile.height, 1.0));

            set_model_matrix(&ctx.gl, &ctx.uniforms, &hex_model);

            let mut hex_color = [0.5, 0.5, 0.5];
            let (mut roughness, mut metalness, mut emissive) = (0.8, 0.0, 0.0);

            if let Some(m) = state.materials.get(&tile.material) {
                hex_color = m.color;
                roughness = m.roughness;
                metalness = m.metalness;
                emissive = m.emissive;
            }

            set_material(&ctx.gl, &ctx.uniforms, hex_color, roughness, metalness, emissive);
            ctx.gl.uniform1i(ctx.uniforms.u_use_tex.as_ref(), 0);
            unit_hex_mesh.draw(&ctx.gl, ctx.attribs.pos, ctx.attribs.norm, ctx.attribs.uv);

            if debug_mode {
                set_material(&ctx.gl, &ctx.uniforms, [0.0, 1.0, 1.0], 0.0, 0.0, 1.0);
                ctx.gl.disable(GL::DEPTH_TEST);
                unit_hex_mesh.draw_wireframe(&ctx.gl, ctx.attribs.pos);
                ctx.gl.enable(GL::DEPTH_TEST);
            }
        }
    }

    ctx.movement_tweens.retain(|_, tween| (now - tween.start_time_ms) < tween.duration_ms);

    for entity in &state.entities {
        if entity.id == 0 || entity.kind == "camera" { continue; }
        
        let mut current_hex_pos = layout.hex_to_world_pos(entity.hex);
        let mut current_hex = entity.hex;

        if let Some(tween) = ctx.movement_tweens.get(&entity.id) {
            let t = ((now - tween.start_time_ms) / tween.duration_ms).clamp(0.0, 1.0) as f32;
            let start_pos = layout.hex_to_world_pos(tween.from_hex);
            let end_pos = layout.hex_to_world_pos(tween.to_hex);
            current_hex_pos = start_pos + (end_pos - start_pos) * t;
            current_hex = if t < 0.5 { tween.from_hex } else { tween.to_hex };
        }

        // Allow overriding position with absolute world_x, world_y properties
        if let Some(PropertyValue::Float(abs_x)) = entity.properties.get("world_x") {
            current_hex_pos.x = *abs_x;
        }
        if let Some(PropertyValue::Float(abs_y)) = entity.properties.get("world_y") {
            current_hex_pos.y = *abs_y;
        }

        let mut top_height = 0.0f32;
        if let Some(map) = &current_map {
            for tile in &map.tiles {
                if tile.hex == current_hex {
                    top_height = top_height.max(tile.bottom + tile.height);
                }
            }
        }

        let entity_scale = entity.get_float("scale", 1.0).log_fallback();
        let z_pos = entity.get_float("z", top_height).log_fallback();
        let rotation_z = -entity.get_float("rotation_z", 0.0).log_fallback();
        
        let cam_rel_offset = Vec3::new(
            entity.get_float("cam_offset_x", 0.0).log_fallback(),
            entity.get_float("cam_offset_y", 0.0).log_fallback(),
            entity.get_float("cam_offset_z", 0.0).log_fallback()
        );
        let world_offset = cam_right * cam_rel_offset.x + cam_up * cam_rel_offset.y + cam_forward * cam_rel_offset.z;

        let mat = entity.get_material(&state.materials).log_fallback();
        let (entity_color, roughness, metalness, emissive) = (mat.color, mat.roughness, mat.metalness, mat.emissive);

        let entity_pos = Vec3::new(current_hex_pos.x, z_pos, current_hex_pos.y);
        let sprite_pos = entity_pos + world_offset;
        
        let billboard_up = Vec3::Y;
        let billboard_right = cam_right;
        let billboard_forward = billboard_right.cross(billboard_up).normalize();
        let billboard_rot = Mat4::from_cols(billboard_right.extend(0.0), billboard_up.extend(0.0), billboard_forward.extend(0.0), glam::Vec4::W);
        
        let side = if cam_forward.z < 0.0 { crate::render::painter::ViewSide::Front } else { crate::render::painter::ViewSide::Mirrored };

        let mut render_rotation_z = rotation_z;
        if side == crate::render::painter::ViewSide::Mirrored {
            render_rotation_z = -rotation_z;
        }

        let sprite_model = Mat4::from_translation(sprite_pos + billboard_up * (entity_scale * 0.5)) * billboard_rot * Mat4::from_scale(Vec3::splat(entity_scale)) * Mat4::from_rotation_z(render_rotation_z);
        
        set_model_matrix(&ctx.gl, &ctx.uniforms, &sprite_model);
        set_material(&ctx.gl, &ctx.uniforms, entity_color, roughness, metalness, emissive);

        ctx.gl.disable(GL::CULL_FACE);

        // Draw Parts
        draw_parts(ctx, entity, &state, sprite_pos, entity_pos, billboard_rot, entity_scale, cam_right, billboard_up, cam_forward, side, render_rotation_z);

        // Draw Skeleton
        if let Some(skeleton) = entity.get_skeleton().log_fallback() {
            draw_skeleton(ctx, entity, sprite_pos, entity_scale, cam_right, billboard_up, cam_forward, &skeleton, debug_mode, side, render_rotation_z);
        }

        // Draw Spritestack
        draw_spritestack(ctx, entity, &state, entity_pos, billboard_rot, entity_scale, cam_forward, render_rotation_z);

        if debug_mode {
            if let Some(shape) = entity.get_collision().log_fallback() {
                draw_collision(ctx, sprite_pos, rotation_z, &shape);
            }
        }

        ctx.gl.enable(GL::CULL_FACE);
    }
}

fn apply_lighting(ctx: &RenderContext, state: &WorldState) {
    let mut lighting = pystral_core::domain::LightingConfig::default();
    if let Some(world) = state.entities.iter().find(|e| e.id == 0) {
        lighting = world.get_lighting().log_fallback();
    }

    ctx.gl.uniform3f(ctx.uniforms.u_ambient_color.as_ref(), lighting.ambient_color[0], lighting.ambient_color[1], lighting.ambient_color[2]);
    ctx.gl.uniform1f(ctx.uniforms.u_ambient_intensity.as_ref(), lighting.ambient_intensity);

    for i in 0..4 {
        if i < lighting.lights.len() {
            let light = &lighting.lights[i];
            ctx.gl.uniform3f(ctx.uniforms.u_lights_dir[i].as_ref(), light.direction[0], light.direction[1], light.direction[2]);
            ctx.gl.uniform3f(ctx.uniforms.u_lights_color[i].as_ref(), light.color[0], light.color[1], light.color[2]);
            ctx.gl.uniform1f(ctx.uniforms.u_lights_intensity[i].as_ref(), light.intensity);
        } else {
            ctx.gl.uniform3f(ctx.uniforms.u_lights_dir[i].as_ref(), 1.0, 1.0, 1.0);
            ctx.gl.uniform3f(ctx.uniforms.u_lights_color[i].as_ref(), 0.0, 0.0, 0.0);
            ctx.gl.uniform1f(ctx.uniforms.u_lights_intensity[i].as_ref(), 0.0);
        }
    }
}

fn get_layout(state: &WorldState) -> hexx::HexLayout {
    if let Some(world) = state.entities.iter().find(|e| e.id == 0) {
        world.get_hex_map().log_fallback().layout()
    } else {
        hexx::HexLayout::default()
    }
}

fn draw_parts(ctx: &mut RenderContext, entity: &EntityState, state: &WorldState, sprite_pos: Vec3, entity_pos: Vec3, billboard_rot: Mat4, entity_scale: f32, cam_right: Vec3, billboard_up: Vec3, cam_forward: Vec3, side: crate::render::painter::ViewSide, render_rotation_z: f32) {
    let parts = entity.get_sprite_parts().log_fallback();
    let cos_z = render_rotation_z.cos();
    let sin_z = render_rotation_z.sin();

    for (i, part) in parts.iter().enumerate() {
        let jx = entity.get_float(&part.x_prop, 0.0).log_fallback();
        let jy = entity.get_float(&part.y_prop, 0.0).log_fallback();
        let jz = entity.get_float(&part.z_prop, 0.0).log_fallback();
        
        let rx = jx * cos_z - jy * sin_z;
        let ry = jx * sin_z + jy * cos_z;
        let rz = jz;

        let prot = if let Some(rot_prop) = &part.rotation_prop {
            -entity.get_float(rot_prop, 0.0).log_fallback()
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
                
                if let (Some(f), Some(m)) = (front_tex, mirrored_tex) {
                    if let Some((_, old_set)) = ctx.sprite_part_textures.insert(cache_key, (part.painter_commands.clone(), crate::render::context::TextureSet { front: f, mirrored: m })) {
                        ctx.gl.delete_texture(Some(&old_set.front));
                        ctx.gl.delete_texture(Some(&old_set.mirrored));
                    }
                }
            }
            
            if let Some((_, tex_set)) = ctx.sprite_part_textures.get(&cache_key) {
                let tex = if side == crate::render::painter::ViewSide::Front { &tex_set.front } else { &tex_set.mirrored };
                ctx.gl.active_texture(GL::TEXTURE0);
                ctx.gl.bind_texture(GL::TEXTURE_2D, Some(tex));
                ctx.gl.uniform1i(ctx.uniforms.u_texture.as_ref(), 0);
                use_tex = true;
            }
        }

        ctx.gl.uniform1i(ctx.uniforms.u_use_tex.as_ref(), use_tex as i32);
        ctx.gl.uniform3f(ctx.uniforms.u_obj_color.as_ref(), part.color[0], part.color[1], part.color[2]);
        
        if let Some((collection_name, asset_name)) = &part.spritestack {
            let cache_key = format!("{}:{}", collection_name, asset_name);
            
            // Check if collection is in cache, otherwise load it
            if !ctx.asset_collection_cache.contains_key(collection_name) {
                if let Some(data) = state.asset_collections.get(collection_name) {
                    let collection = pystral_compiler::assets::AssetCollection::from_binary(data);
                    ctx.asset_collection_cache.insert(collection_name.clone(), collection);
                }
            }

            if !ctx.spritestack_assets.contains_key(&cache_key) {
                if let Some(collection) = ctx.asset_collection_cache.get(collection_name) {
                    if let Some(stack) = collection.spritestacks.get(asset_name) {
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
                }
            }

            if let Some(textures) = ctx.spritestack_assets.get(&cache_key) {
                let spacing = textures.spacing;
                let quad_scale = (textures.width as f32 - 0.5) * spacing * part.scale * entity_scale;
                
                for (i, (color_tex, normal_tex)) in textures.color_textures.iter().zip(textures.normal_textures.iter()).enumerate() {
                    let z_offset = (i as f32 - textures.color_textures.len() as f32 * 0.5) * spacing * entity_scale;
                    
                    let engine_world_offset = Vec3::new(rx, rz, ry) * entity_scale;
                    let spritestack_part_pos = entity_pos + engine_world_offset + Vec3::Y * z_offset;

                    let slice_model = Mat4::from_translation(spritestack_part_pos) 
                        * Mat4::from_rotation_y(render_rotation_z + render_prot)
                        * Mat4::from_rotation_x(-std::f32::consts::FRAC_PI_2)
                        * Mat4::from_scale(Vec3::splat(quad_scale));
                    set_model_matrix(&ctx.gl, &ctx.uniforms, &slice_model);
                    
                    ctx.gl.active_texture(GL::TEXTURE0);
                    ctx.gl.bind_texture(GL::TEXTURE_2D, Some(color_tex));
                    ctx.gl.uniform1i(ctx.uniforms.u_texture.as_ref(), 0);
                    ctx.gl.active_texture(GL::TEXTURE1);
                    ctx.gl.bind_texture(GL::TEXTURE_2D, Some(normal_tex));
                    ctx.gl.uniform1i(ctx.uniforms.u_normal_map.as_ref(), 1);
                    ctx.gl.uniform1i(ctx.uniforms.u_use_tex.as_ref(), 1);
                    ctx.gl.uniform1i(ctx.uniforms.u_use_normal_map.as_ref(), 1);
                    ctx.sprite_mesh.draw(&ctx.gl, ctx.attribs.pos, ctx.attribs.norm, ctx.attribs.uv);
                }
                ctx.gl.uniform1i(ctx.uniforms.u_use_normal_map.as_ref(), 0);
            }
        } else {
            ctx.sprite_mesh.draw(&ctx.gl, ctx.attribs.pos, ctx.attribs.norm, ctx.attribs.uv);
        }
    }
}

fn draw_skeleton(ctx: &mut RenderContext, entity: &EntityState, sprite_pos: Vec3, entity_scale: f32, cam_right: Vec3, billboard_up: Vec3, cam_forward: Vec3, skeleton: &pystral_core::domain::Skeleton, debug_mode: bool, side: crate::render::painter::ViewSide, render_rotation_z: f32) {
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
                    entity.get_float(&px, 0.0).log_fallback(),
                    entity.get_float(&py, 0.0).log_fallback(),
                    entity.get_float(&pz, 0.0).log_fallback(),
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
                
                if let (Some(f), Some(m)) = (front_tex, mirrored_tex) {
                    if let Some((_, old_set)) = ctx.bone_textures.insert(cache_key, (bone.painter_commands.clone(), crate::render::context::TextureSet { front: f, mirrored: m })) {
                        ctx.gl.delete_texture(Some(&old_set.front));
                        ctx.gl.delete_texture(Some(&old_set.mirrored));
                    }
                }
            }
            
            if let Some((_, tex_set)) = ctx.bone_textures.get(&cache_key) {
                let tex = if side == crate::render::painter::ViewSide::Front { &tex_set.front } else { &tex_set.mirrored };
                ctx.gl.active_texture(GL::TEXTURE0);
                ctx.gl.bind_texture(GL::TEXTURE_2D, Some(tex));
                ctx.gl.uniform1i(ctx.uniforms.u_texture.as_ref(), 0);
                ctx.gl.uniform1i(ctx.uniforms.u_use_tex.as_ref(), 1);
                
                // Draw bones slightly behind parts
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
            ctx.gl.uniform1i(ctx.uniforms.u_use_tex.as_ref(), 0);
            ctx.cylinder_mesh.draw_wireframe(&ctx.gl, ctx.attribs.pos);
        }
    }
    
    if debug_mode {
        ctx.gl.enable(GL::DEPTH_TEST);
    }
}

fn draw_spritestack(ctx: &mut RenderContext, entity: &EntityState, state: &WorldState, sprite_pos: Vec3, _billboard_rot: Mat4, entity_scale: f32, _cam_forward: Vec3, rotation_z: f32) {
    if let Some(PropertyValue::AssetRef(collection_name)) = entity.properties.get("spritestack_collection") {
        let asset_name = if let Some(PropertyValue::String(name)) = entity.properties.get("spritestack_name") {
            name
        } else {
            return;
        };

        let cache_key = format!("{}:{}", collection_name, asset_name);
        
        // Check if collection is in cache, otherwise load it
        if !ctx.asset_collection_cache.contains_key(collection_name) {
            if let Some(data) = state.asset_collections.get(collection_name) {
                let collection = pystral_compiler::assets::AssetCollection::from_binary(data);
                ctx.asset_collection_cache.insert(collection_name.clone(), collection);
            }
        }

        if !ctx.spritestack_assets.contains_key(&cache_key) {
            if let Some(collection) = ctx.asset_collection_cache.get(collection_name) {
                if let Some(stack) = collection.spritestacks.get(asset_name) {
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
            }
        }

        if let Some(textures) = ctx.spritestack_assets.get(&cache_key) {
            let spacing = textures.spacing;
            let quad_scale = (textures.width as f32 - 0.5) * spacing * entity_scale;
            
            for (i, (color_tex, normal_tex)) in textures.color_textures.iter().zip(textures.normal_textures.iter()).enumerate() {
                let z_offset = (i as f32 - textures.color_textures.len() as f32 * 0.5) * spacing * entity_scale;
                let slice_pos = sprite_pos + Vec3::Y * z_offset; // Spritestacks are stacked vertically in world space
                
                let slice_model = Mat4::from_translation(slice_pos) 
                    * Mat4::from_rotation_y(rotation_z)
                    * Mat4::from_rotation_x(-std::f32::consts::FRAC_PI_2) // Rotate to lay flat on XZ plane
                    * Mat4::from_scale(Vec3::splat(quad_scale));
                
                set_model_matrix(&ctx.gl, &ctx.uniforms, &slice_model);
                
                ctx.gl.active_texture(GL::TEXTURE0);
                ctx.gl.bind_texture(GL::TEXTURE_2D, Some(color_tex));
                ctx.gl.uniform1i(ctx.uniforms.u_texture.as_ref(), 0);
                
                ctx.gl.active_texture(GL::TEXTURE1);
                ctx.gl.bind_texture(GL::TEXTURE_2D, Some(normal_tex));
                ctx.gl.uniform1i(ctx.uniforms.u_normal_map.as_ref(), 1);
                
                ctx.gl.uniform1i(ctx.uniforms.u_use_tex.as_ref(), 1);
                ctx.gl.uniform1i(ctx.uniforms.u_use_normal_map.as_ref(), 1);
                
                ctx.sprite_mesh.draw(&ctx.gl, ctx.attribs.pos, ctx.attribs.norm, ctx.attribs.uv);
            }
            
            // Reset normal map usage
            ctx.gl.uniform1i(ctx.uniforms.u_use_normal_map.as_ref(), 0);
        }
    }
}

fn create_texture(gl: &GL, width: u32, height: u32, data: &[u8]) -> web_sys::WebGlTexture {
    let texture = gl.create_texture().unwrap();
    gl.bind_texture(GL::TEXTURE_2D, Some(&texture));
    gl.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
        GL::TEXTURE_2D, 0, GL::RGBA as i32, width as i32, height as i32, 0, GL::RGBA, GL::UNSIGNED_BYTE, Some(data)
    ).unwrap();
    gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_MIN_FILTER, GL::NEAREST as i32);
    gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_MAG_FILTER, GL::NEAREST as i32);
    gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_WRAP_S, GL::CLAMP_TO_EDGE as i32);
    gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_WRAP_T, GL::CLAMP_TO_EDGE as i32);
    texture
}

fn draw_collision(ctx: &RenderContext, sprite_pos: Vec3, rotation_z: f32, shape: &Shape3D) {
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
