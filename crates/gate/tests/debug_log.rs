use pystral_core::history::HistoryManager;
use pystral_compiler::demo::generate_demo_log;
use pystral_gate::ui_log::get_log_messages;
use pystral_gate::render::utils::{EntityExt, RenderResultExt};
use pystral_core::communication::WorkerBus;

#[test]
fn debug_ui_log_errors() {
    let mut bus_data = vec![0u8; 1024 * 1024];
    let bus = unsafe { WorkerBus::from_ptr(bus_data.as_mut_ptr(), bus_data.len()) };

    let mut history = HistoryManager::new();
    generate_demo_log(&mut history);
    
    let steps = history.log.len();
    
    println!("Total Log Length: {} steps", steps);
    
    for i in 0..=steps {
        history.jump_to(i);
        let state = &history.current_state;
        
        println!("Index: {}, Entities: {}", i, state.entities.len());
        for entity in &state.entities {
            if entity.id == 0 {
                let _ = entity.get_hex_map().log_fallback(&bus);
                let _ = entity.get_lighting().log_fallback(&bus);
            } else if entity.kind == "camera" || entity.kind == "camera_anchor" {
                let _ = entity.get_float("angle", 0.0).log_fallback(&bus);
                let _ = entity.get_float("distance", 0.0).log_fallback(&bus);
                let _ = entity.get_float("height", 0.0).log_fallback(&bus);
            } else {
                let _ = entity.get_float("scale", 1.0).log_fallback(&bus);
                let _ = entity.get_float("z", 0.0).log_fallback(&bus);
                let _ = entity.get_material(&state.materials).log_fallback(&bus);
                let _ = entity.get_sprite_parts().log_fallback(&bus);
                
                if entity.kind == "arrow" {
                    let _ = entity.get_float("rotation_z", 0.0).log_fallback(&bus);
                }
            }
        }
        
        let errors = get_log_messages(&bus);
        if !errors.is_empty() {
            println!("Errors at index {}:", i);
            for err in errors {
                println!("  - {}", err);
            }
        }
    }
}
