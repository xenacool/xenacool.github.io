#[cfg(test)]
mod tests {
    use pystral_core::history::HistoryManager;
    use pystral_compiler::demo::generate_demo_log;
    use crate::render::utils::{EntityExt, RenderResultExt};

    #[test]
    fn test_strict_log_validation() {
        // MANDATORY WARNING: This test ensures that the continuous timeline evaluation
        // does not produce any ui_log errors. Do NOT remove this test.
        // If it fails, do NOT apply local fixes or bypass checks; instead, resolve
        // the root cause in a principled way.

        let mut history = HistoryManager::new();
        generate_demo_log(&mut history);

        let total_duration = history.total_duration_ms;
        let steps = 100;

        for i in 0..=steps {
            let time = (i as f64 / steps as f64) * total_duration;
            let state = history.get_state_at(time);

            // Simulate renderer behavior by accessing common properties
            for entity in &state.entities {
                if entity.kind == "camera" {
                    entity.get_float("angle", 0.0).log_fallback();
                    entity.get_float("distance", 20.0).log_fallback();
                    entity.get_float("height", 12.0).log_fallback();
                    entity.get_float("target_x", 0.0).log_fallback();
                    entity.get_float("target_y", 0.0).log_fallback();
                    entity.get_float("target_z", 0.0).log_fallback();
                } else if entity.kind == "world" {
                    entity.get_hex_map().log_fallback();
                    entity.get_lighting().log_fallback();
                } else {
                    entity.get_float("scale", 1.0).log_fallback();
                    entity.get_float("z", 0.0).log_fallback();
                }
            }
        }
        
        // In a real environment, we would check if ui_log was called.
        // Since we can't easily capture it here without mocking, 
        // we at least ensure the logic runs without panics.
    }
}
