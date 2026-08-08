use crate::render::context::RenderContext;
use crate::render::state::CameraTween;
use crate::render::utils::{EntityExt, RenderResultExt};
use glam::{Mat4, Vec3};
use pystral_core::log::WorldState;
use wasm_bindgen::JsCast;
use web_sys::HtmlCanvasElement;

pub fn update_canvas_size(ctx: &RenderContext) -> (u32, u32) {
    let window = web_sys::window().expect("No global window found");
    let canvas = ctx
        .gl
        .canvas()
        .expect("No canvas found")
        .dyn_into::<HtmlCanvasElement>()
        .expect("Canvas is not an HtmlCanvasElement");
    let width = window
        .inner_width()
        .expect("Could not get innerWidth")
        .as_f64()
        .expect("innerWidth is not a number") as u32;
    let height = window
        .inner_height()
        .expect("Could not get innerHeight")
        .as_f64()
        .expect("innerHeight is not a number") as u32;

    if canvas.width() != width || canvas.height() != height {
        canvas.set_width(width);
        canvas.set_height(height);
        #[allow(clippy::cast_possible_wrap)]
        ctx.gl.viewport(0, 0, width as i32, height as i32);
    }
    (width, height)
}

pub fn setup_camera(
    ctx: &mut RenderContext,
    worker_tx: &futures::channel::mpsc::UnboundedSender<crate::WorkerInput>,
    state: &WorldState,
    width: u32,
    height: u32,
    delta_ms: f64,
) -> (Mat4, Mat4, Vec3, Vec3, Vec3) {
    let mut cam_dist = 20.0;
    let mut angle = 0.0;
    let mut height_val = 12.0;
    let mut target = Vec3::ZERO;

    let cam = if let Some(id) = ctx.active_camera_id {
        let found = state
            .entities
            .iter()
            .find(|e| e.id == id && e.kind == "camera");
        if found.is_none() {
            let _ = worker_tx.unbounded_send(crate::WorkerInput::LogError(format!(
                "Active camera {} not found in state",
                id
            )));
        }
        found
    } else {
        state.entities.iter().find(|e| e.kind == "camera")
    };

    if let Some(cam) = cam {
        let target_values = [
            cam.get_float("angle", 0.0).log_fallback(worker_tx),
            cam.get_float("distance", 20.0).log_fallback(worker_tx),
            cam.get_float("height", 12.0).log_fallback(worker_tx),
            cam.get_float("target_x", 0.0).log_fallback(worker_tx),
            cam.get_float("target_y", 0.0).log_fallback(worker_tx),
            cam.get_float("target_z", 0.0).log_fallback(worker_tx),
        ];
        let config = state.transition_configs.get(&cam.id);
        if let Some((previous_id, previous_pose)) = ctx.camera_pose {
            if previous_id != cam.id {
                ctx.camera_tween = None;
                ctx.camera_pose = Some((cam.id, target_values));
            } else {
                let current = if let Some(tween) = &mut ctx.camera_tween {
                    tween.advance(delta_ms)
                } else {
                    previous_pose
                };
                if ctx.camera_tween.as_ref().is_some_and(CameraTween::finished) {
                    ctx.camera_tween = None;
                }
                let needs_new_tween = ctx
                    .camera_tween
                    .as_ref()
                    .is_none_or(|tween| tween.target != target_values);
                if needs_new_tween
                    && current != target_values
                    && let Some(config) = config
                {
                    ctx.camera_tween =
                        Some(CameraTween::new(cam.id, current, target_values, config));
                }
                ctx.camera_pose = Some((cam.id, current));
            }
        } else {
            ctx.camera_tween = None;
            ctx.camera_pose = Some((cam.id, target_values));
        }
        let values = ctx
            .camera_pose
            .expect("camera pose should be initialized")
            .1;
        angle = values[0];
        cam_dist = values[1];
        height_val = values[2];
        target = Vec3::new(values[3], values[4], values[5]);
    }

    let cam_x = cam_dist * angle.cos();
    let cam_z = cam_dist * angle.sin();
    let view = Mat4::look_at_rh(
        Vec3::new(cam_x + target.x, height_val + target.y, cam_z + target.z),
        target,
        Vec3::Y,
    );

    let aspect = width as f32 / height as f32;
    let ortho_size = cam_dist * 0.5;
    let proj = Mat4::orthographic_rh(
        -ortho_size * aspect,
        ortho_size * aspect,
        -ortho_size,
        ortho_size,
        -100.0,
        100.0,
    );

    let cam_right = Vec3::new(view.x_axis.x, view.y_axis.x, view.z_axis.x);
    let cam_up = Vec3::new(view.x_axis.y, view.y_axis.y, view.z_axis.y);
    let cam_backward = Vec3::new(view.x_axis.z, view.y_axis.z, view.z_axis.z);
    let cam_forward = -cam_backward;

    ctx.gl.uniform_matrix4fv_with_f32_array(
        ctx.uniforms.view.as_ref(),
        false,
        &view.to_cols_array(),
    );
    ctx.gl.uniform_matrix4fv_with_f32_array(
        ctx.uniforms.proj.as_ref(),
        false,
        &proj.to_cols_array(),
    );

    (view, proj, cam_right, cam_up, cam_forward)
}
