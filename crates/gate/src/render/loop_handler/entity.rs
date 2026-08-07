use glam::{Mat4, Vec3};
use pystral_core::log::{EntityState, WorldState};
use pystral_core::domain::Shape3D;
use web_sys::WebGlRenderingContext as GL;
use crate::render::context::RenderContext;
use crate::render::draw_utils::{set_model_matrix, set_material};


pub fn draw_spritestack(ctx: &mut RenderContext, worker_tx: &futures::channel::mpsc::UnboundedSender<crate::WorkerInput>, entity: &EntityState, state: &WorldState, sprite_pos: Vec3, _billboard_rot: glam::Mat4, entity_scale: f32, _cam_forward: Vec3, yaw: f32) {
    if let Some(pystral_core::log::PropertyValue::String(asset_name)) = entity.properties.get("asset") {
        let collection_name = "primitives".to_string();
        let cache_key = format!("{}:{}", collection_name, asset_name);
        attach_assets(ctx, worker_tx, state, &collection_name, asset_name, &cache_key);

        if let Some(textures) = ctx.spritestack_assets.get(&cache_key) {
            let quad_scale = textures.aabb.x * entity_scale;
            let total_height = textures.aabb.y * entity_scale;
            let num_layers = textures.color_textures.len() as f32;
            
            for (i, (color_tex, normal_tex)) in textures.color_textures.iter().zip(textures.normal_textures.iter()).enumerate() {
                let z_offset = if num_layers > 1.0 {
                    (i as f32 / (num_layers - 1.0) - 0.5) * total_height
                } else {
                    0.0
                };
                let slice_pos = sprite_pos + Vec3::Y * z_offset;
                
                let slice_model = glam::Mat4::from_translation(slice_pos) 
                    * glam::Mat4::from_rotation_y(yaw)
                    * glam::Mat4::from_rotation_x(-std::f32::consts::FRAC_PI_2)
                    * glam::Mat4::from_scale(Vec3::splat(quad_scale));
                
                set_model_matrix(&ctx.gl, &ctx.uniforms, &slice_model);
                
                ctx.gl.active_texture(web_sys::WebGlRenderingContext::TEXTURE0);
                ctx.gl.bind_texture(web_sys::WebGlRenderingContext::TEXTURE_2D, Some(color_tex));
                ctx.gl.uniform1i(ctx.uniforms.texture.as_ref(), 0);
                
                ctx.gl.active_texture(web_sys::WebGlRenderingContext::TEXTURE1);
                ctx.gl.bind_texture(web_sys::WebGlRenderingContext::TEXTURE_2D, Some(normal_tex));
                ctx.gl.uniform1i(ctx.uniforms.normal_map.as_ref(), 1);
                
                ctx.gl.uniform1i(ctx.uniforms.use_tex.as_ref(), 1);
                ctx.gl.uniform1i(ctx.uniforms.use_normal_map.as_ref(), 1);
                
                ctx.sprite_mesh.draw(&ctx.gl, ctx.attribs.pos, ctx.attribs.norm, ctx.attribs.uv);
            }
            ctx.gl.uniform1i(ctx.uniforms.use_normal_map.as_ref(), 0);
        }
    }
}

fn attach_assets(ctx: &mut RenderContext, worker_tx: &futures::channel::mpsc::UnboundedSender<crate::WorkerInput>, state: &WorldState, collection_name: &String, asset_name: &String, cache_key: &str) {
    if !ctx.asset_collection_cache.contains_key(collection_name) {
        if let Some(data) = state.asset_collections.get(collection_name) {
            let collection = pystral_compiler::assets::AssetCollection::from_binary(data);
            ctx.asset_collection_cache.insert(collection_name.clone(), collection);
        } else {
            let _ = worker_tx.unbounded_send(crate::WorkerInput::LogError(format!("Asset collection {} not found in state", collection_name)));
        }
    }

    if !ctx.spritestack_assets.contains_key(cache_key) {
        if let Some(collection) = ctx.asset_collection_cache.get(collection_name) {
            if let Some(stack) = collection.spritestacks.get(asset_name) {
                let mut color_textures = Vec::new();
                let mut normal_textures = Vec::new();
                for slice in &stack.slices {
                    color_textures.push(create_texture(&ctx.gl, stack.width, stack.height, &slice.color_data));
                    normal_textures.push(create_texture(&ctx.gl, stack.width, stack.height, &slice.normal_data));
                }
                ctx.spritestack_assets.insert(cache_key.to_string(), crate::render::context::SpritestackTextures {
                    color_textures,
                    normal_textures,
                    width: stack.width,
                    height: stack.height,
                    spacing: stack.spacing,
                    aabb: stack.aabb,
                });
            } else {
                let _ = worker_tx.unbounded_send(crate::WorkerInput::LogError(format!("Asset {} not found in collection {}", asset_name, collection_name)));
            }
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
