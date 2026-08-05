use glam::{Mat4, Vec3};
use pystral_core::log::WorldState;
use wasm_bindgen::JsCast;
use web_sys::HtmlCanvasElement;
use crate::render::context::RenderContext;
use crate::render::utils::{EntityExt, RenderResultExt};

pub fn update_canvas_size(ctx: &RenderContext) -> (u32, u32) {
    let window = web_sys::window().unwrap();
    let canvas = ctx.gl.canvas().unwrap().dyn_into::<HtmlCanvasElement>().unwrap();
    let width = window.inner_width().unwrap().as_f64().unwrap() as u32;
    let height = window.inner_height().unwrap().as_f64().unwrap() as u32;
    
    if canvas.width() != width || canvas.height() != height {
        canvas.set_width(width);
        canvas.set_height(height);
        ctx.gl.viewport(0, 0, width as i32, height as i32);
    }
    (width, height)
}

pub fn setup_camera(ctx: &mut RenderContext, worker_tx: &futures::channel::mpsc::UnboundedSender<crate::WorkerInput>, state: &WorldState, width: u32, height: u32) -> (Mat4, Mat4, Vec3, Vec3, Vec3) {
    let mut cam_dist = 20.0;
    let mut angle = 0.0;
    let mut height_val = 12.0;
    let mut target = Vec3::ZERO;

    if let Some(cam) = state.entities.iter().find(|e| e.kind == "camera") {
        angle = cam.get_float("angle", 0.0).log_fallback(worker_tx);
        cam_dist = cam.get_float("distance", 20.0).log_fallback(worker_tx);
        height_val = cam.get_float("height", 12.0).log_fallback(worker_tx);
        target = Vec3::new(
            cam.get_float("target_x", 0.0).log_fallback(worker_tx),
            cam.get_float("target_y", 0.0).log_fallback(worker_tx),
            cam.get_float("target_z", 0.0).log_fallback(worker_tx),
        );
    }

    let cam_x = cam_dist * angle.cos();
    let cam_z = cam_dist * angle.sin();
    let view = Mat4::look_at_rh(Vec3::new(cam_x + target.x, height_val + target.y, cam_z + target.z), target, Vec3::Y);
    
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

    ctx.gl.uniform_matrix4fv_with_f32_array(ctx.uniforms.u_view.as_ref(), false, &view.to_cols_array());
    ctx.gl.uniform_matrix4fv_with_f32_array(ctx.uniforms.u_proj.as_ref(), false, &proj.to_cols_array());

    (view, proj, cam_right, cam_up, cam_forward)
}
