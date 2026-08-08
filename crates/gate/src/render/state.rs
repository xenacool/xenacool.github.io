use hexx::Hex;
use pystral_core::log::{PropertyValue, TransitionConfig};
use tween::{SineInOut, Tweener};

pub struct PlaybackState {
    pub playing_log: bool,
    pub playing_animations: bool,
    pub debug_mode: bool,
    pub last_tick_ms: f64,
    pub last_debug_mode: bool,
    pub last_history_log_len: usize,
    pub last_debug_index: usize,
    pub history_step_ms: f64,
}

impl Default for PlaybackState {
    fn default() -> Self {
        Self {
            playing_log: true,
            playing_animations: true,
            debug_mode: false,
            last_tick_ms: 0.0,
            last_debug_mode: false,
            last_history_log_len: 0,
            last_debug_index: 999999,
            history_step_ms: 17.0,
        }
    }
}

pub struct MovementTween {
    pub from_hex: Hex,
    pub to_hex: Hex,
    pub start_time_ms: f64,
    pub duration_ms: f64,
    pub transition: TransitionConfig,
    pub tweeners: Option<[Tweener<f32, f64, SineInOut>; 3]>,
}

pub struct PropertyTween {
    pub property: String,
    pub from_value: PropertyValue,
    pub to_value: PropertyValue,
    pub start_time_ms: f64,
    pub duration_ms: f64,
}

pub struct CameraTween {
    pub camera_id: u64,
    pub target: [f32; 6],
    pub values: [Tweener<f32, f64, SineInOut>; 6],
    pub delta_time_ms: f64,
}

impl CameraTween {
    pub fn new(camera_id: u64, start: [f32; 6], end: [f32; 6], config: &TransitionConfig) -> Self {
        let duration = f64::from(config.duration_ms.max(1));
        Self {
            camera_id,
            target: end,
            values: std::array::from_fn(|i| {
                Tweener::new_at(start[i], end[i], duration, SineInOut, 0.0)
            }),
            delta_time_ms: f64::from(config.delta_time_ms),
        }
    }

    pub fn advance(&mut self, delta_ms: f64) -> [f32; 6] {
        let delta = if self.delta_time_ms > 0.0 {
            self.delta_time_ms.min(delta_ms)
        } else {
            delta_ms
        };
        std::array::from_fn(|i| self.values[i].move_by(delta))
    }

    pub fn finished(&self) -> bool {
        self.values.iter().all(Tweener::is_finished)
    }
}

#[cfg(test)]
mod tests {
    use super::CameraTween;
    use pystral_core::log::{TransitionConfig, TweenKind};

    #[test]
    fn camera_tween_uses_sine_in_out_and_completes() {
        let config = TransitionConfig {
            duration_ms: 100,
            delta_time_ms: 16.0,
            tween: TweenKind::SineInOut,
        };
        let mut tween = CameraTween::new(1, [0.0; 6], [10.0; 6], &config);
        let mut midpoint = 0.0;
        for _ in 0..3 {
            midpoint = tween.advance(50.0)[0];
        }
        assert!(midpoint > 0.0 && midpoint < 10.0);
        for _ in 0..4 {
            tween.advance(100.0);
        }
        assert!(tween.finished());
    }
}
