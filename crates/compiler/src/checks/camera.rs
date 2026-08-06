use pystral_core::history::HistoryManager;
use pystral_core::log::PropertyValue;
use std::collections::HashMap;

pub fn check_camera_symmetry(history: &HistoryManager) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    
    // We want to check the final state of the history
    let state = &history.current_state;
    
    let cameras: Vec<_> = state.entities.iter()
        .filter(|e| e.kind == "camera")
        .collect();

    let mut neighbors: HashMap<u64, HashMap<String, u64>> = HashMap::new();

    for camera in &cameras {
        let mut cam_neighbors = HashMap::new();
        for (prop, val) in &camera.properties {
            if prop.starts_with("neighbor_") && let PropertyValue::Float(target_id) = val {
                cam_neighbors.insert(prop.clone(), *target_id as u64);
            }
        }
        neighbors.insert(camera.id, cam_neighbors);
    }

    for (&cam_id, cam_neighbors) in &neighbors {
        for (dir, &target_id) in cam_neighbors {
            let opposite_dir = match dir.as_str() {
                "neighbor_right" => "neighbor_left",
                "neighbor_left" => "neighbor_right",
                "neighbor_up" => "neighbor_down",
                "neighbor_down" => "neighbor_up",
                _ => continue,
            };

            let is_symmetric = neighbors.get(&target_id)
                .and_then(|target_neighbors| target_neighbors.get(opposite_dir))
                .is_some_and(|&back_id| back_id == cam_id);

            if !is_symmetric {
                let target_neighbor_id = neighbors.get(&target_id)
                    .and_then(|target_neighbors| target_neighbors.get(opposite_dir))
                    .copied();

                let error_msg = if let Some(back_id) = target_neighbor_id {
                    format!(
                        "Camera symmetry error: Camera {} has {} pointing to {}, but Camera {}'s {} points back to {} instead of {}.",
                        cam_id, dir, target_id, target_id, opposite_dir, back_id, cam_id
                    )
                } else {
                    format!(
                        "Camera symmetry error: Camera {} has {} pointing to {}, but Camera {} has no {} pointing back to {}.",
                        cam_id, dir, target_id, target_id, opposite_dir, cam_id
                    )
                };
                errors.push(error_msg);
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sample_history_log::positive::generate_valid_camera_log;
    use crate::sample_history_log::negative::{generate_asymmetric_camera_log, generate_broken_chain_camera_log};

    #[test]
    fn test_valid_camera_symmetry() {
        let mut history = HistoryManager::new();
        generate_valid_camera_log(&mut history);
        assert!(check_camera_symmetry(&history).is_ok());
    }

    #[test]
    fn test_asymmetric_camera_symmetry() {
        let mut history = HistoryManager::new();
        generate_asymmetric_camera_log(&mut history);
        let result = check_camera_symmetry(&history);
        assert!(result.is_err());
        let errs = result.expect_err("should be error");
        assert_eq!(errs.len(), 1);
        assert!(errs[0].contains("Camera 20 has neighbor_right pointing to 21, but Camera 21 has no neighbor_left pointing back to 20"));
    }

    #[test]
    fn test_broken_chain_camera_symmetry() {
        let mut history = HistoryManager::new();
        generate_broken_chain_camera_log(&mut history);
        let result = check_camera_symmetry(&history);
        assert!(result.is_err());
        let errs = result.expect_err("should be error");
        // 30 -> 31 (OK)
        // 31 -> 30 (OK)
        // 31 -> 32 (OK)
        // 32 -> 30 (FAIL, should be 31)
        // Actually:
        // 31 has neighbor_up -> 32. 32 should have neighbor_down -> 31. But 32 has neighbor_down -> 30.
        // 32 has neighbor_down -> 30. 30 should have neighbor_up -> 32. But 30 has no neighbor_up.
        assert!(errs.iter().any(|e| e.contains("Camera 31 has neighbor_up pointing to 32, but Camera 32's neighbor_down points back to 30 instead of 31")));
        assert!(errs.iter().any(|e| e.contains("Camera 32 has neighbor_down pointing to 30, but Camera 30 has no neighbor_up pointing back to 32")));
    }
}
