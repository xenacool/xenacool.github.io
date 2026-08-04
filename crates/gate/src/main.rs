use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{WebGlRenderingContext as GL};
use std::sync::{Mutex, Arc};
use pystral_core::history::HistoryManager;
use pystral_gate::render::{compile_shader, link_program, create_sprite_mesh, start_render_loop, VERTEX_SHADER, FRAGMENT_SHADER, PlaybackState};
use pystral_compiler::demo::generate_demo_log;

lazy_static::lazy_static! {
    static ref HISTORY: Arc<Mutex<Option<HistoryManager>>> = Arc::new(Mutex::new(None));
    static ref PLAYBACK: Arc<Mutex<PlaybackState>> = Arc::new(Mutex::new(PlaybackState::default()));
}

#[wasm_bindgen]
pub fn set_history_index(index: u32) {
    if let Ok(mut history) = HISTORY.lock() {
        if let Some(h) = history.as_mut() {
            h.jump_to(index as usize);
        }
    }
}

#[wasm_bindgen]
pub fn get_log_length() -> u32 {
    if let Ok(history) = HISTORY.lock() {
        if let Some(h) = history.as_ref() {
            return h.log.len() as u32;
        }
    }
    0
}

#[wasm_bindgen]
pub fn toggle_play_log() {
    if let Ok(mut pb) = PLAYBACK.lock() {
        pb.playing_log = !pb.playing_log;
    }
}

#[wasm_bindgen]
pub fn toggle_play_animations() {
    if let Ok(mut pb) = PLAYBACK.lock() {
        pb.playing_animations = !pb.playing_animations;
    }
}

#[wasm_bindgen]
pub fn set_debug_mode(enabled: bool) {
    if let Ok(mut pb) = PLAYBACK.lock() {
        pb.debug_mode = enabled;
    }
}

#[wasm_bindgen]
pub fn set_broken_mode(enabled: bool) {
    pystral_core::render::ERROR_MODE_ENABLED.store(enabled, std::sync::atomic::Ordering::Relaxed);
}

#[wasm_bindgen(start)]
pub fn run() -> Result<(), JsValue> {
    let document = web_sys::window().unwrap().document().unwrap();
    let canvas = document.get_element_by_id("canvas").unwrap()
        .dyn_into::<web_sys::HtmlCanvasElement>()?;

    let gl: GL = canvas.get_context("webgl")?
        .unwrap().dyn_into()?;

    let vert_shader = compile_shader(&gl, GL::VERTEX_SHADER, VERTEX_SHADER)?;
    let frag_shader = compile_shader(&gl, GL::FRAGMENT_SHADER, FRAGMENT_SHADER)?;
    let program = link_program(&gl, &vert_shader, &frag_shader)?;
    gl.use_program(Some(&program));

    // Initialize history with some data
    let mut history = HistoryManager::new();
    generate_demo_log(&mut history);
    history.jump_to(0);

    if let Ok(mut h) = HISTORY.lock() {
        *h = Some(history);
    }

    let sprite_mesh = create_sprite_mesh(&gl);

    gl.enable(GL::DEPTH_TEST);
    gl.enable(GL::CULL_FACE);
    gl.clear_color(0.1, 0.1, 0.1, 1.0);
    
    start_render_loop(gl, program, sprite_mesh, HISTORY.clone(), PLAYBACK.clone());

    Ok(())
}

fn main() {}