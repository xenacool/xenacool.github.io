use futures::SinkExt;
use futures::StreamExt;
use js_sys::Uint8Array;
use pystral_core::history::HistoryManager;
use pystral_gate::render::{
    FRAGMENT_SHADER, VERTEX_SHADER, compile_shader, create_sprite_mesh, link_program,
    start_render_loop,
};
use pystral_gate::simulation_worker::{SimulationInput, SimulationOutput, SimulationWorker};
use pystral_gate::worker::UnifiedWorker;
use pystral_gate::{
    AppCommand, Envelope, ReliableInput, ReliableOutput, WorkerInput, WorkerOutput,
};
use pystral_runtime::pg_rpg::{
    AssetManifest, NamedBinaryAsset, NamedTextAsset, ScenarioBundle, VirtualRhaiWorkspace,
};
use pystral_runtime::{Runtime, RuntimeRequest, RuntimeResponse};
use std::sync::mpsc::{Sender, channel};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::WebGlRenderingContext as GL;

fn simulation_request_label(request: &RuntimeRequest) -> &'static str {
    match request {
        RuntimeRequest::StartPgRpgSimulation { .. } => "StartPgRpgSimulation",
        RuntimeRequest::StepPgRpgSimulation => "StepPgRpgSimulation",
        RuntimeRequest::RequestMctsDecision { .. } => "RequestMctsDecision",
        RuntimeRequest::MctsDecisionReady { .. } => "MctsDecisionReady",
        RuntimeRequest::AcknowledgeAnimation { .. } => "AcknowledgeAnimation",
        RuntimeRequest::CommitWait { .. } => "CommitWait",
        RuntimeRequest::CommitMove { .. } => "CommitMove",
        RuntimeRequest::CommitDecision { .. } => "CommitDecision",
        RuntimeRequest::OpenMovePreview { .. } => "OpenMovePreview",
        RuntimeRequest::OpenAbilityTargets { .. } => "OpenAbilityTargets",
        RuntimeRequest::ActionInput { .. } => "ActionInput",
        RuntimeRequest::TestOccupyDestination { .. } => "TestOccupyDestination",
        RuntimeRequest::ResumeBoundary => "ResumeBoundary",
        RuntimeRequest::ResumeRejected { .. } => "ResumeRejected",
        RuntimeRequest::SolveIk(_) | RuntimeRequest::GeneratePgRpgLog { .. } => "Other",
        RuntimeRequest::RunRhaiCase { .. } => "RunRhaiCase",
    }
}

#[wasm_bindgen]
pub fn run_rhai_case(workspace_json: &str, case_name: &str, seed: u64) -> String {
    let result = (|| -> Result<serde_json::Value, String> {
        let workspace: VirtualRhaiWorkspace = serde_json::from_str(workspace_json)
            .map_err(|error| format!("Invalid workspace JSON: {error}"))?;
        let request = RuntimeRequest::RunRhaiCase {
            workspace,
            case_name: case_name.to_string(),
            seed,
        };
        let (response, logs) = Runtime::new().process_request(request);
        match response {
            RuntimeResponse::RhaiCaseResult {
                case_name,
                seed,
                replay_header,
                details,
            } => Ok(serde_json::json!({
                "status": "passed",
                "case_name": case_name,
                "seed": seed,
                "replay_header": replay_header,
                "details": serde_json::from_str::<serde_json::Value>(&details)
                    .map_err(|error| format!("Invalid case details: {error}"))?,
            })),
            RuntimeResponse::Error(error) => Err(error),
            other => Err(format!(
                "Unexpected Rhai response: {other:?}; logs={logs:?}"
            )),
        }
    })();
    match result {
        Ok(value) => serde_json::to_string(&value).unwrap_or_else(|error| {
            format!(r#"{{"status":"error","error":"serialization failed: {error}"}}"#)
        }),
        Err(error) => serde_json::json!({ "status": "error", "error": error }).to_string(),
    }
}

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

    pub fn set_history_step_ms(&self, value: f64) {
        let _ = self.sender.send(AppCommand::SetHistoryStepMs(value));
    }

    pub fn set_broken_mode(&self, enabled: bool) {
        pystral_core::render::ERROR_MODE_ENABLED
            .store(enabled, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn update_history(&self, json: &str) {
        if let Ok(history) = serde_json::from_str::<HistoryManager>(json) {
            let _ = self
                .sender
                .send(AppCommand::UpdateHistory(Box::new(history)));
        }
    }

    pub fn camera_nav(&self, direction: String) {
        let _ = self.sender.send(AppCommand::CameraNav(direction));
    }

    pub fn action_nav(&self, direction: String) {
        let _ = self.sender.send(AppCommand::ActionNav(direction));
    }
}

fn initialize_renderer(
    gl: &GL,
) -> Result<
    (
        web_sys::WebGlProgram,
        pystral_gate::render::Mesh,
        HistoryManager,
    ),
    JsValue,
> {
    let vert_shader = compile_shader(gl, GL::VERTEX_SHADER, VERTEX_SHADER)?;
    let frag_shader = compile_shader(gl, GL::FRAGMENT_SHADER, FRAGMENT_SHADER)?;
    let program = link_program(gl, &vert_shader, &frag_shader)?;
    gl.use_program(Some(&program));

    let mut history = HistoryManager::new();
    history.jump_to(0);
    pystral_gate::render::set_ui_slider_max(history.log.len() as u32);

    gl.enable(GL::DEPTH_TEST);
    gl.enable(GL::CULL_FACE);
    gl.enable(GL::BLEND);
    gl.blend_func(GL::SRC_ALPHA, GL::ONE_MINUS_SRC_ALPHA);
    gl.clear_color(0.1, 0.1, 0.1, 1.0);
    Ok((program, create_sprite_mesh(gl), history))
}

fn request_initial_pg_rpg_log(worker_tx: futures::channel::mpsc::UnboundedSender<WorkerInput>) {
    wasm_bindgen_futures::spawn_local(async move {
        match fetch_assets().await {
            Ok((bundle, atlas_json, spritesheet_rgba, width)) => {
                let _ = worker_tx.unbounded_send(WorkerInput::RuntimeRequest(
                    RuntimeRequest::GeneratePgRpgLog {
                        bundle,
                        atlas_json,
                        spritesheet_rgba,
                        spritesheet_width: width,
                    },
                ));
            }
            Err(error) => {
                web_sys::console::error_1(&format!("Failed to fetch assets: {error:?}").into());
            }
        }
    });
}

fn start_heartbeat_probe(worker_tx: futures::channel::mpsc::UnboundedSender<WorkerInput>) {
    wasm_bindgen_futures::spawn_local(async move {
        loop {
            gloo_timers::future::TimeoutFuture::new(500).await;
            if worker_tx
                .unbounded_send(WorkerInput::HeartbeatProbe)
                .is_err()
            {
                break;
            }
        }
    });
}

#[wasm_bindgen]
pub fn run_app() -> Result<AppHandle, JsValue> {
    let document = web_sys::window()
        .expect("No global window found")
        .document()
        .expect("No document found");
    let canvas = document
        .get_element_by_id("canvas")
        .expect("No element with id 'canvas' found")
        .dyn_into::<web_sys::HtmlCanvasElement>()?;

    let gl: GL = canvas
        .get_context("webgl")?
        .expect("Could not get WebGL context")
        .dyn_into()?;

    let (program, sprite_mesh, history) = initialize_renderer(&gl)?;

    let (app_tx, app_rx) = channel();
    let (worker_tx, mut worker_rx) = futures::channel::mpsc::unbounded::<WorkerInput>();

    request_initial_pg_rpg_log(worker_tx.clone());

    let bridge = UnifiedWorker::spawn();
    let (mut bridge_sender, mut bridge_listener) = bridge.split();
    let simulation_bridge = SimulationWorker::spawn();
    let (mut simulation_sender, mut simulation_listener) = simulation_bridge.split();

    let unified_input_tx = worker_tx.clone();
    wasm_bindgen_futures::spawn_local(async move {
        use futures::StreamExt;
        while let Some(output) = simulation_listener.next().await {
            match output {
                SimulationOutput::Response(response) => {
                    record_debug_trace(format!(
                        "simulation bridge received response request seq {} continuation {:?}",
                        response.msg.request_seq, response.msg.continuation
                    ));
                    if unified_input_tx
                        .unbounded_send(WorkerInput::SimulationResponse(Box::new(response)))
                        .is_err()
                    {
                        record_debug_trace(
                            "simulation bridge failed to forward response to unified worker"
                                .to_string(),
                        );
                    }
                }
                SimulationOutput::Heartbeat(heartbeat) => {
                    record_debug_trace(format!(
                        "simulation worker heartbeat input seq {} output seq {} status {:?}",
                        heartbeat.latest_input_seq, heartbeat.latest_output_seq, heartbeat.status
                    ));
                }
                SimulationOutput::Watermark(seq) => {
                    record_debug_trace(format!(
                        "simulation bridge received watermark input seq {seq}"
                    ));
                }
            }
        }
        record_debug_trace("simulation bridge listener ended".to_string());
        let _ = unified_input_tx.unbounded_send(WorkerInput::SimulationBridgeFailure {
            request_seq: None,
            reason: "simulation worker listener ended".to_string(),
        });
    });

    let app_tx_clone = app_tx.clone();
    let simulation_failure_tx = worker_tx.clone();
    wasm_bindgen_futures::spawn_local(async move {
        use futures::StreamExt;
        while let Some(output) = bridge_listener.next().await {
            match output {
                ReliableOutput::Heartbeat(heartbeat) => {
                    update_worker_heartbeat(
                        heartbeat.latest_output_seq,
                        heartbeat.latest_input_seq,
                        format!(
                            "{:?}{} · simulation request {:?} · progress {}",
                            heartbeat.status,
                            heartbeat
                                .pending_barrier
                                .map_or_else(String::new, |id| format!(" · pending barrier {id}")),
                            heartbeat.active_request_seq,
                            heartbeat.last_progress_seq,
                        ),
                    );
                }
                ReliableOutput::Msg(envelope) => match envelope.msg {
                    WorkerOutput::SimulationRequest(request) => {
                        let request_seq = request.seq;
                        record_debug_trace(format!(
                            "simulation bridge send request seq {} kind {}",
                            request.seq,
                            simulation_request_label(&request.msg)
                        ));
                        if simulation_sender
                            .send(SimulationInput::Request(request))
                            .await
                            .is_err()
                        {
                            record_debug_trace(
                                "simulation bridge failed to send request to simulation worker"
                                    .to_string(),
                            );
                            let _ = simulation_failure_tx.unbounded_send(
                                WorkerInput::SimulationBridgeFailure {
                                    request_seq: Some(request_seq),
                                    reason: "simulation worker channel closed".to_string(),
                                },
                            );
                        }
                    }
                    WorkerOutput::LogUpdate {
                        messages,
                        total_errors,
                        total_info,
                    } => {
                        update_log_ui(messages, total_errors, total_info);
                    }
                    WorkerOutput::RuntimeResponse(res) => match *res {
                        RuntimeResponse::PgRpgLogGenerated(history) => {
                            let _ = app_tx_clone.send(AppCommand::UpdateHistory(Box::new(history)));
                        }
                        RuntimeResponse::PgRpgSimulationStarted(history) => {
                            let _ = app_tx_clone.send(AppCommand::UpdateHistory(Box::new(history)));
                        }
                        RuntimeResponse::PgRpgSimulationStepped(history) => {
                            let _ = app_tx_clone.send(AppCommand::AppendHistory(Box::new(history)));
                        }
                        RuntimeResponse::GameCompleted { history, .. } => {
                            record_game_completed_response();
                            let _ = app_tx_clone.send(AppCommand::AppendHistory(Box::new(history)));
                        }
                        RuntimeResponse::ActionCommitted {
                            action, history, ..
                        } => {
                            update_action_feedback(&format!("{} committed", action));
                            let _ = app_tx_clone.send(AppCommand::AppendHistory(Box::new(history)));
                        }
                        RuntimeResponse::ActionRejected { reason, .. } => {
                            // Keep feedback tied to the authoritative runtime
                            // response as well as the compatibility side-channel.
                            // This prevents a later transient-state update from
                            // hiding a rejection before it reaches the user.
                            update_action_feedback(&format!("Move rejected: {:?}", reason));
                        }
                        _ => {}
                    },
                    WorkerOutput::TransientState(state) => {
                        record_debug_trace(format!(
                            "main thread applying transient active_unit={:?} actions={}",
                            state.active_unit_id,
                            state.available_actions.is_some()
                        ));
                        if let Ok(json) = serde_json::to_string(&*state) {
                            update_action_menu(json);
                        }
                        let _ = app_tx_clone.send(AppCommand::UpdateTransientState(state));
                    }
                    WorkerOutput::DebugTrace { message } => {
                        record_debug_trace(message);
                    }
                    WorkerOutput::ActionRejected { request_id, reason } => {
                        update_action_feedback(&format!("Move rejected: {:?}", reason));
                        let _ =
                            app_tx_clone.send(AppCommand::ActionRejected { request_id, reason });
                    }
                },
                ReliableOutput::Watermark(_) => {}
            }
        }
        record_debug_trace("unified worker listener ended".to_string());
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

    // Liveness probes are control traffic, not gameplay history.
    start_heartbeat_probe(worker_tx.clone());

    start_render_loop(gl, program, sprite_mesh, history, app_rx, worker_tx);

    Ok(AppHandle { sender: app_tx })
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = window)]
    fn update_log_ui(messages: Vec<String>, total_errors: u32, total_info: u32);

    #[wasm_bindgen(js_namespace = window)]
    fn update_action_menu(json: String);

    #[wasm_bindgen(js_namespace = window)]
    fn update_action_feedback(message: &str);

    #[wasm_bindgen(js_namespace = window)]
    fn record_debug_trace(message: String);

    #[wasm_bindgen(js_namespace = window)]
    fn record_game_completed_response();

    #[wasm_bindgen(js_namespace = window)]
    fn update_worker_heartbeat(latest_seq: u64, latest_input_seq: u64, status: String);
}

async fn fetch_assets() -> Result<(ScenarioBundle, String, Vec<u8>, u32), JsValue> {
    let window = web_sys::window().expect("No global window found");

    let resp_atlas =
        wasm_bindgen_futures::JsFuture::from(window.fetch_with_str("web/atlas.json")).await?;
    let resp_atlas: web_sys::Response = resp_atlas.dyn_into()?;
    if !resp_atlas.ok() {
        return Err(format!(
            "Failed to fetch atlas: {} {}",
            resp_atlas.status(),
            resp_atlas.status_text()
        )
        .into());
    }
    let atlas_json = wasm_bindgen_futures::JsFuture::from(resp_atlas.text()?)
        .await?
        .as_string()
        .unwrap();

    let resp_img =
        wasm_bindgen_futures::JsFuture::from(window.fetch_with_str("web/spritesheet.png")).await?;
    let resp_img: web_sys::Response = resp_img.dyn_into()?;
    if !resp_img.ok() {
        return Err(format!(
            "Failed to fetch spritesheet: {} {}",
            resp_img.status(),
            resp_img.status_text()
        )
        .into());
    }
    let blob = wasm_bindgen_futures::JsFuture::from(resp_img.blob()?).await?;
    let blob: web_sys::Blob = blob.dyn_into()?;

    let bitmap_promise = window.create_image_bitmap_with_blob(&blob)?;
    let bitmap = wasm_bindgen_futures::JsFuture::from(bitmap_promise).await?;
    let bitmap: web_sys::ImageBitmap = bitmap.dyn_into()?;

    let width = bitmap.width();
    let height = bitmap.height();

    let document = window.document().expect("No document found");
    let canvas = document
        .create_element("canvas")?
        .dyn_into::<web_sys::HtmlCanvasElement>()?;
    canvas.set_width(width);
    canvas.set_height(height);
    let ctx = canvas
        .get_context("2d")?
        .unwrap()
        .dyn_into::<web_sys::CanvasRenderingContext2d>()?;
    ctx.draw_image_with_image_bitmap(&bitmap, 0.0, 0.0)?;

    let image_data = ctx.get_image_data(0, 0, width as i32, height as i32)?;
    let pixels = image_data.data().to_vec();

    let script_manifest = fetch_manifest("web/scripts/manifest.json").await?;
    let material_manifest = fetch_manifest("web/assets/material/manifest.json").await?;
    let yarn_manifest = fetch_manifest("web/assets/yarnscript/manifest.json").await?;
    let mut bundle = ScenarioBundle::default();
    for path in script_manifest.files {
        // Fetch from the browser-facing web/scripts URL while retaining the
        // bundle's internal scripts/ namespace for Rhai include resolution.
        let web_path = format!("web/scripts/{path}");
        bundle.rhai_files.push(NamedTextAsset {
            path: format!("scripts/{path}"),
            contents: fetch_text(&web_path).await?,
        });
    }
    for path in material_manifest.files {
        let web_path = format!("web/assets/material/{path}");
        bundle.material_files.push(NamedBinaryAsset {
            path: format!("materials/{path}"),
            contents: fetch_bytes(&web_path).await?,
        });
    }
    for path in yarn_manifest.files {
        let web_path = format!("web/assets/yarnscript/{path}");
        bundle.yarn_files.push(NamedTextAsset {
            path: format!("yarn/{path}"),
            contents: fetch_text(&web_path).await?,
        });
    }
    Ok((bundle, atlas_json, pixels, width))
}

async fn fetch_manifest(path: &str) -> Result<AssetManifest, JsValue> {
    AssetManifest::parse(&fetch_text(path).await?).map_err(|error| JsValue::from_str(&error))
}

async fn fetch_text(path: &str) -> Result<String, JsValue> {
    let response =
        wasm_bindgen_futures::JsFuture::from(web_sys::window().unwrap().fetch_with_str(path))
            .await?;
    let response: web_sys::Response = response.dyn_into()?;
    if !response.ok() {
        return Err(format!(
            "Failed to fetch {path}: {} {}",
            response.status(),
            response.status_text()
        )
        .into());
    }
    wasm_bindgen_futures::JsFuture::from(response.text()?)
        .await?
        .as_string()
        .ok_or_else(|| JsValue::from_str("asset response was not text"))
}

async fn fetch_bytes(path: &str) -> Result<Vec<u8>, JsValue> {
    let response =
        wasm_bindgen_futures::JsFuture::from(web_sys::window().unwrap().fetch_with_str(path))
            .await?;
    let response: web_sys::Response = response.dyn_into()?;
    if !response.ok() {
        return Err(format!(
            "Failed to fetch {path}: {} {}",
            response.status(),
            response.status_text()
        )
        .into());
    }
    let buffer = wasm_bindgen_futures::JsFuture::from(response.array_buffer()?).await?;
    Ok(Uint8Array::new(&buffer).to_vec())
}

fn main() {}
