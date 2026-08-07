use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{WebGlRenderingContext as GL};
use std::sync::mpsc::{channel, Sender};
use pystral_runtime::{RuntimeRequest, RuntimeResponse};
use pystral_core::history::HistoryManager;
use pystral_gate::render::{compile_shader, link_program, create_sprite_mesh, start_render_loop, VERTEX_SHADER, FRAGMENT_SHADER};
use pystral_gate::{AppCommand, WorkerOutput, ReliableOutput, WorkerInput, ReliableInput, Envelope};
use futures::StreamExt;
use futures::SinkExt;
use pystral_gate::worker::UnifiedWorker;

#[wasm_bindgen]
pub struct AppHandle {
    sender: Sender<AppCommand>,
}

#[wasm_bindgen]
impl AppHandle {
    pub fn set_history_index(&self, index: u32) {
        let _ = self.sender.send(AppCommand::SetHistoryIndex(index));
    }

    pub fn toggle_play_log(&self) {
        let _ = self.sender.send(AppCommand::TogglePlayLog);
    }

    pub fn toggle_play_animations(&self) {
        let _ = self.sender.send(AppCommand::TogglePlayAnimations);
    }

    pub fn set_debug_mode(&self, enabled: bool) {
        let _ = self.sender.send(AppCommand::SetDebugMode(enabled));
    }

    pub fn set_broken_mode(&self, enabled: bool) {
        pystral_core::render::ERROR_MODE_ENABLED.store(enabled, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn update_history(&self, json: &str) {
        if let Ok(history) = serde_json::from_str::<HistoryManager>(json) {
            let _ = self.sender.send(AppCommand::UpdateHistory(Box::new(history)));
        }
    }

    pub fn camera_nav(&self, direction: String) {
        let _ = self.sender.send(AppCommand::CameraNav(direction));
    }
}

#[wasm_bindgen]
pub fn run_app() -> Result<AppHandle, JsValue> {
    let document = web_sys::window().expect("No global window found").document().expect("No document found");
    let canvas = document.get_element_by_id("canvas").expect("No element with id 'canvas' found")
        .dyn_into::<web_sys::HtmlCanvasElement>()?;

    let gl: GL = canvas.get_context("webgl")?
        .expect("Could not get WebGL context").dyn_into()?;

    let vert_shader = compile_shader(&gl, GL::VERTEX_SHADER, VERTEX_SHADER)?;
    let frag_shader = compile_shader(&gl, GL::FRAGMENT_SHADER, FRAGMENT_SHADER)?;
    let program = link_program(&gl, &vert_shader, &frag_shader)?;
    gl.use_program(Some(&program));

    // Initialize history with empty data
    let mut history = HistoryManager::new();
    history.jump_to(0);

    pystral_gate::render::set_ui_slider_max(history.log.len() as u32);

    let sprite_mesh = create_sprite_mesh(&gl);

    gl.enable(GL::DEPTH_TEST);
    gl.enable(GL::CULL_FACE);
    gl.enable(GL::BLEND);
    gl.blend_func(GL::SRC_ALPHA, GL::ONE_MINUS_SRC_ALPHA);
    gl.clear_color(0.1, 0.1, 0.1, 1.0);
    
    let (app_tx, app_rx) = channel();
    let (worker_tx, mut worker_rx) = futures::channel::mpsc::unbounded::<WorkerInput>();
    
    // Request initial demo log immediately
    let worker_tx_clone = worker_tx.clone();
    wasm_bindgen_futures::spawn_local(async move {
        gloo_timers::future::TimeoutFuture::new(50).await;
        let _ = worker_tx_clone.unbounded_send(WorkerInput::RuntimeRequest(RuntimeRequest::GenerateDemoLog));
    });

    let bridge = UnifiedWorker::spawn();
    let (mut bridge_sender, mut bridge_listener) = bridge.split();
    
    let app_tx_clone = app_tx.clone();
    wasm_bindgen_futures::spawn_local(async move {
        use futures::StreamExt;
        while let Some(output) = bridge_listener.next().await {
            if let ReliableOutput::Msg(envelope) = output {
                match envelope.msg {
                    WorkerOutput::LogUpdate { messages, total_errors } => {
                        update_log_ui(messages, total_errors);
                    }
                    WorkerOutput::RuntimeResponse(res) => {
                        if let RuntimeResponse::DemoLogGenerated(history) = *res {
                            let _ = app_tx_clone.send(AppCommand::UpdateHistory(Box::new(history)));
                        }
                    }
                }
            }
        }
    });

    wasm_bindgen_futures::spawn_local(async move {
        use futures::StreamExt;
        let mut next_seq = 1u64;
        while let Some(input) = worker_rx.next().await {
            let envelope = Envelope {
                seq: next_seq,
                msg: input,
            };
            next_seq += 1;
            let _ = bridge_sender.send(ReliableInput::Msg(envelope)).await;
        }
    });

    start_render_loop(gl, program, sprite_mesh, history, app_rx, worker_tx);

    Ok(AppHandle { sender: app_tx })
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = window)]
    fn update_log_ui(messages: Vec<String>, total_errors: u32);
}

fn main() {}
