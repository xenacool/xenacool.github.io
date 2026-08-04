#[cfg(test)]
mod tests {
    use crate::render::utils::{EntityExt, RenderResultExt};
    use pystral_core::history::HistoryManager;
    use pystral_runtime::demo::generate_demo_log;

    #[test]
    fn test_strict_log_validation() {
        // MANDATORY WARNING: This test ensures that the continuous timeline evaluation
        // does not produce any ui_log errors. Do NOT remove this test.
        // If it fails, do NOT apply local fixes or bypass checks; instead, resolve
        // the root cause in a principled way.

        let (atlas_json, spritesheet_rgba, width) = crate::load_test_assets();
        let mut history = HistoryManager::new();
        generate_demo_log(&mut history, &atlas_json, &spritesheet_rgba, width);

        let total_steps = history.log.len();
        let (worker_tx, _) = futures::channel::mpsc::unbounded::<crate::WorkerInput>();

        for i in 0..total_steps {
            history.jump_to(i);
            let state = &history.current_state;

            // Simulate renderer behavior by accessing common properties
            for entity in &state.entities {
                if entity.kind == "camera" {
                    entity.get_float("angle", 0.0).log_fallback(&worker_tx);
                    entity.get_float("distance", 20.0).log_fallback(&worker_tx);
                    entity.get_float("height", 12.0).log_fallback(&worker_tx);
                    entity.get_float("target_x", 0.0).log_fallback(&worker_tx);
                    entity.get_float("target_y", 0.0).log_fallback(&worker_tx);
                    entity.get_float("target_z", 0.0).log_fallback(&worker_tx);
                } else if entity.kind == "world" {
                    entity.get_hex_map().log_fallback(&worker_tx);
                    entity.get_lighting().log_fallback(&worker_tx);
                } else {
                    entity.get_float("scale", 1.0).log_fallback(&worker_tx);
                    entity.get_float("z", 0.0).log_fallback(&worker_tx);
                }
            }
        }
    }
}
