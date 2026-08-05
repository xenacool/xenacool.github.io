pub mod camera;

use pystral_core::history::HistoryManager;

pub fn validate_history(history: &HistoryManager) -> Vec<String> {
    let mut errors = Vec::new();

    if let Err(mut camera_errors) = camera::check_camera_symmetry(history) {
        errors.append(&mut camera_errors);
    }

    errors
}
