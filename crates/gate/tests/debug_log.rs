#![allow(clippy::panic, clippy::unwrap_used, clippy::never_loop)]

use pystral_core::history::HistoryManager;
use pystral_gate::render::utils::{EntityExt, RenderResultExt};
use pystral_gate::WorkerInput;
use futures::channel::mpsc;
use pystral_runtime::demo::generate_demo_log;

#[test]
fn debug_ui_log_errors() {
    let (tx, mut rx) = mpsc::unbounded::<WorkerInput>();

    let (atlas_json, spritesheet_rgba, width) = pystral_gate::load_test_assets();
    let mut history = HistoryManager::new();
    generate_demo_log(&mut history, &atlas_json, &spritesheet_rgba, width);
    
    let steps = history.log.len();
    
    println!("Total Log Length: {} steps", steps);
    
    for i in 0..=steps {
        history.jump_to(i);
        let state = &history.current_state;
        
        println!("Index: {}, Entities: {}", i, state.entities.len());
        for entity in &state.entities {
            if entity.id == 0 {
                let _ = entity.get_hex_map().log_fallback(&tx);
                let _ = entity.get_lighting().log_fallback(&tx);
            } else if entity.kind == "camera" || entity.kind == "camera_anchor" {
                let _ = entity.get_float("angle", 0.0).log_fallback(&tx);
                let _ = entity.get_float("distance", 0.0).log_fallback(&tx);
                let _ = entity.get_float("height", 0.0).log_fallback(&tx);
            } else {
                let _ = entity.get_float("scale", 1.0).log_fallback(&tx);
                let _ = entity.get_float("z", 0.0).log_fallback(&tx);
                let _ = entity.get_material(&state.materials).log_fallback(&tx);
                
                if entity.kind == "arrow" {
                    let _ = entity.get_float("rotation_z", 0.0).log_fallback(&tx);
                }
            }
        }
        
        while let Ok(msg) = rx.try_recv() {
            if let WorkerInput::LogError(msg) = msg {
                println!("Errors at index {}:", i);
                println!("  - {}", msg);
                panic!("UI Log error: {}", msg);
            }
        }
    }
}
