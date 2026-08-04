use web_sys::WebGlRenderingContext as GL;
use glam::Mat4;
use crate::render::context::UniformLocations;

pub fn set_model_matrix(gl: &GL, uniforms: &UniformLocations, matrix: &Mat4) {
    if let Some(loc) = uniforms.u_model.as_ref() {
        gl.uniform_matrix4fv_with_f32_array(Some(loc), false, &matrix.to_cols_array());
    }
}

pub fn set_material(gl: &GL, uniforms: &UniformLocations, color: [f32; 3], roughness: f32, metalness: f32, emissive: f32) {
    if let Some(loc) = uniforms.u_obj_color.as_ref() {
        gl.uniform3f(Some(loc), color[0], color[1], color[2]);
    }
    if let Some(loc) = uniforms.u_roughness.as_ref() {
        gl.uniform1f(Some(loc), roughness);
    }
    if let Some(loc) = uniforms.u_metalness.as_ref() {
        gl.uniform1f(Some(loc), metalness);
    }
    if let Some(loc) = uniforms.u_emissive.as_ref() {
        gl.uniform1f(Some(loc), emissive);
    }
}
