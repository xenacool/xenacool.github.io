use pystral_core::history::HistoryManager;
use pystral_compiler::demo::generate_demo_log;
use pystral_gate::ui_log::{get_log_messages, reset_log};
use pystral_gate::render::utils::{EntityExt, RenderResultExt};

#[test]
fn debug_ui_log_errors() {
    reset_log();
    let mut history = HistoryManager::new();
    generate_demo_log(&mut history);
    
    let total_duration = history.total_duration;
    let steps = 1000;
    
    println!("Total Duration: {}ms", total_duration);
    
    for i in 0..=steps {
        let time = (i as f64) * (total_duration / steps as f64);
        
        history.seek_to_time(time);
        let state = history.get_state_at(time);
        
        println!("Time: {}ms, Entities: {}", time, state.entities.len());
        for entity in &state.entities {
            println!("  Entity {}: kind={}, hex={:?}, props={:?}", entity.id, entity.kind, entity.hex, entity.properties.keys());
            if entity.id == 0 {
                let _ = entity.get_hex_map().log_fallback();
                let _ = entity.get_lighting().log_fallback();
            } else if entity.kind == "camera" || entity.kind == "camera_anchor" {
                let _ = entity.get_float("angle", 0.0).log_fallback();
                let _ = entity.get_float("distance", 0.0).log_fallback();
                let _ = entity.get_float("height", 0.0).log_fallback();
            } else {
                let _ = entity.get_float("scale", 1.0).log_fallback();
                let _ = entity.get_float("z", 0.0).log_fallback();
                let _ = entity.get_material(&state.materials).log_fallback();
                let _ = entity.get_sprite_parts().log_fallback();
                
                if entity.kind == "arrow" {
                    let _ = entity.get_float("rotation_z", 0.0).log_fallback();
                }
            }
        }
        
        let errors = get_log_messages();
        if !errors.is_empty() {
            println!("Errors at time {}ms (index {}/{}):", time, i, steps);
            for err in errors {
                println!("  - {}", err);
            }
            reset_log();
        }
    }
}
