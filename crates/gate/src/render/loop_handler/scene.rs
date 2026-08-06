use glam::{Mat4, Vec3, Vec2};
use pystral_core::log::{WorldState, EntityState, PropertyValue};
use pystral_core::domain::{Shape3D, Joint};
use web_sys::WebGlRenderingContext as GL;
use hexx::ColumnMeshBuilder;
use crate::render::context::RenderContext;
use crate::render::mesh::Mesh;
use crate::render::draw_utils::{set_model_matrix, set_material};
use crate::render::utils::{EntityExt, RenderResultExt};
use super::entity::{draw_parts, draw_skeleton, draw_spritestack, draw_collision};

#[allow(clippy::too_many_lines)]
pub fn draw_scene(ctx: &mut RenderContext, worker_tx: &futures::channel::mpsc::UnboundedSender<crate::WorkerInput>, state: &WorldState, cam_right: Vec3, cam_up: Vec3, cam_forward: Vec3, debug_mode: bool, now: f64, _is_playing_anims: bool) {
    apply_lighting(ctx, worker_tx, state);

    let layout = get_layout(state, worker_tx);

    if ctx.unit_hex_mesh_cache.is_none() || ctx.cached_hex_orientation.as_ref() != Some(&layout.orientation) {
        if let Some(old_mesh) = ctx.unit_hex_mesh_cache.take() {
            old_mesh.destroy(&ctx.gl);
        }
        let mut unit_layout = layout.clone();
        unit_layout.scale = Vec2::ONE;
        let unit_hex_info = ColumnMeshBuilder::new(&unit_layout, 1.0).center_aligned().without_bottom_face().build();
        ctx.unit_hex_mesh_cache = Some(Mesh::from_mesh_info(&ctx.gl, &unit_hex_info));
        ctx.cached_hex_orientation = Some(layout.orientation);
    }
    let unit_hex_mesh = ctx.unit_hex_mesh_cache.as_ref().expect("Hex mesh cache should be initialized");

    let mut current_map = None;
    if let Some(world) = state.entities.iter().find(|e| e.id == 0) {
        current_map = Some(world.get_hex_map().log_fallback(worker_tx));
    }

    if let Some(ref map) = current_map {
        for tile in &map.tiles {
            let world_pos = layout.hex_to_world_pos(tile.hex);
            let hex_model = Mat4::from_translation(Vec3::new(world_pos.x, tile.bottom, world_pos.y))
                * Mat4::from_scale(Vec3::new(layout.scale.x, tile.height, layout.scale.y));

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
            ctx.gl.uniform1i(ctx.uniforms.use_tex.as_ref(), 0);
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

        let entity_scale = entity.get_float("scale", 1.0).log_fallback(worker_tx);
        let z_pos = entity.get_float("z", top_height).log_fallback(worker_tx);
        let rotation_z = -entity.get_float("rotation_z", 0.0).log_fallback(worker_tx);
        let rotation_y = entity.get_float("rotation_y", 0.0).log_fallback(worker_tx);
        
        let cam_rel_offset = Vec3::new(
            entity.get_float("cam_offset_x", 0.0).log_fallback(worker_tx),
            entity.get_float("cam_offset_y", 0.0).log_fallback(worker_tx),
            entity.get_float("cam_offset_z", 0.0).log_fallback(worker_tx)
        );
        let world_offset = cam_right * cam_rel_offset.x + cam_up * cam_rel_offset.y + cam_forward * cam_rel_offset.z;

        let mat = entity.get_material(&state.materials).log_fallback(worker_tx);
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
        draw_parts(ctx, worker_tx, entity, state, sprite_pos, entity_pos, billboard_rot, entity_scale, cam_right, billboard_up, cam_forward, side, render_rotation_z, rotation_y);

        // Draw Skeleton
        if let Some(skeleton) = entity.get_skeleton().log_fallback(worker_tx) {
            draw_skeleton(ctx, worker_tx, entity, state, sprite_pos, entity_scale, cam_right, billboard_up, cam_forward, &skeleton, debug_mode, side, render_rotation_z, rotation_y);
        }

        // Draw Spritestack
        draw_spritestack(ctx, entity, state, entity_pos, billboard_rot, entity_scale, cam_forward, rotation_y);

        if debug_mode && let Some(shape) = entity.get_collision().log_fallback(worker_tx) {
            draw_collision(ctx, sprite_pos, rotation_z, &shape);
        }

        ctx.gl.enable(GL::CULL_FACE);
    }
}

fn apply_lighting(ctx: &mut RenderContext, worker_tx: &futures::channel::mpsc::UnboundedSender<crate::WorkerInput>, state: &WorldState) {
    let mut lighting = pystral_core::domain::LightingConfig::default();
    if let Some(world) = state.entities.iter().find(|e| e.id == 0) {
        lighting = world.get_lighting().log_fallback(worker_tx);
    }

    ctx.gl.uniform3f(ctx.uniforms.ambient_color.as_ref(), lighting.ambient_color[0], lighting.ambient_color[1], lighting.ambient_color[2]);
    ctx.gl.uniform1f(ctx.uniforms.ambient_intensity.as_ref(), lighting.ambient_intensity);

    for i in 0..4 {
        if i < lighting.lights.len() {
            let light = &lighting.lights[i];
            ctx.gl.uniform3f(ctx.uniforms.lights_dir[i].as_ref(), light.direction[0], light.direction[1], light.direction[2]);
            ctx.gl.uniform3f(ctx.uniforms.lights_color[i].as_ref(), light.color[0], light.color[1], light.color[2]);
            ctx.gl.uniform1f(ctx.uniforms.lights_intensity[i].as_ref(), light.intensity);
        } else {
            ctx.gl.uniform3f(ctx.uniforms.lights_dir[i].as_ref(), 1.0, 1.0, 1.0);
            ctx.gl.uniform3f(ctx.uniforms.lights_color[i].as_ref(), 0.0, 0.0, 0.0);
            ctx.gl.uniform1f(ctx.uniforms.lights_intensity[i].as_ref(), 0.0);
        }
    }
}

fn get_layout(state: &WorldState, worker_tx: &futures::channel::mpsc::UnboundedSender<crate::WorkerInput>) -> hexx::HexLayout {
    if let Some(world) = state.entities.iter().find(|e| e.id == 0) {
        world.get_hex_map().log_fallback(worker_tx).layout()
    } else {
        hexx::HexLayout::default()
    }
}
