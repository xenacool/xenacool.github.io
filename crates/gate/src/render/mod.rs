mod shaders;
mod mesh;
mod state;
mod context;
pub mod utils;
mod draw_utils;
pub mod loop_handler;
pub mod painter;

pub use crate::render::shaders::{VERTEX_SHADER, FRAGMENT_SHADER};
pub use crate::render::mesh::{Mesh, create_sprite_mesh, create_sphere_mesh, create_cylinder_mesh};
pub use crate::render::state::PlaybackState;

use web_sys::{WebGlRenderingContext as GL, WebGlProgram, WebGlShader};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use std::cell::RefCell;
use std::rc::Rc;
use pystral_core::history::HistoryManager;

use crate::render::context::RenderContext;
use crate::render::loop_handler::LoopHandler;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = window)]
    pub fn update_ui_slider(index: u32);
    
    #[wasm_bindgen(js_namespace = window)]
    pub fn set_ui_slider_max(max: u32);

    #[wasm_bindgen(js_namespace = window)]
    pub fn update_nav_buttons(up: bool, down: bool, left: bool, right: bool);

    #[wasm_bindgen(js_namespace = window)]
    pub fn update_action_buttons(visible: bool, up: bool, down: bool, left: bool, right: bool, confirm: bool, ret: bool);

    #[wasm_bindgen(js_namespace = window)]
    pub fn update_entity_viewer(json: &str);

    #[wasm_bindgen(js_namespace = window)]
    pub fn update_history_log(json: &str);
}


pub fn compile_shader(gl: &GL, shader_type: u32, source: &str) -> Result<WebGlShader, String> {
    let shader = gl.create_shader(shader_type)
        .ok_or("Unable to create shader")?;
    gl.shader_source(&shader, source);
    gl.compile_shader(&shader);

    if gl.get_shader_parameter(&shader, GL::COMPILE_STATUS).as_bool().unwrap_or(false) {
        Ok(shader)
    } else {
        Err(gl.get_shader_info_log(&shader).unwrap_or_default())
    }
}

pub fn link_program(gl: &GL, vert: &WebGlShader, frag: &WebGlShader) -> Result<WebGlProgram, String> {
    let program = gl.create_program().ok_or("Unable to create program")?;
    gl.attach_shader(&program, vert);
    gl.attach_shader(&program, frag);
    gl.link_program(&program);

    if gl.get_program_parameter(&program, GL::LINK_STATUS).as_bool().unwrap_or(false) {
        Ok(program)
    } else {
        Err(gl.get_program_info_log(&program).unwrap_or_default())
    }
}

fn request_animation_frame(f: &Closure<dyn FnMut()>) {
    web_sys::window()
        .expect("No global window found")
        .request_animation_frame(f.as_ref().unchecked_ref())
        .expect("should register `requestAnimationFrame` OK");
}

pub fn start_render_loop(
    gl: GL, 
    program: WebGlProgram, 
    sprite_mesh: Mesh, 
    history_manager: HistoryManager,
    app_rx: std::sync::mpsc::Receiver<crate::AppCommand>,
    worker_tx: futures::channel::mpsc::UnboundedSender<crate::WorkerInput>,
) {
    let sphere_mesh = create_sphere_mesh(&gl, 16, 16);
    let cylinder_mesh = create_cylinder_mesh(&gl, 16);
    
    let ctx = RenderContext::new(gl, program, sprite_mesh, sphere_mesh, cylinder_mesh);
    let handler = Rc::new(RefCell::new(LoopHandler::new(ctx, history_manager, app_rx, worker_tx)));

    let f = Rc::new(RefCell::new(None));
    let g = f.clone();

    *g.borrow_mut() = Some(Closure::wrap(Box::new(move || {
        handler.borrow_mut().tick();
        request_animation_frame(f.borrow().as_ref().expect("Closure should be initialized"));
    }) as Box<dyn FnMut()>));

    request_animation_frame(g.borrow().as_ref().expect("Closure should be initialized"));
}
