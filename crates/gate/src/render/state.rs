use pystral_core::log::{MovementWaypoint, PropertyValue, TransitionConfig};
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
    pub last_sequence_ack_sent: Option<(u64, f64)>,
    /// Invalidation clock for visual overlays created from history.
    pub playback_epoch: u64,
}

pub(crate) fn sequence_ack_due(last: Option<(u64, f64)>, sequence: u64, now: f64) -> bool {
    !last.is_some_and(|(sent, sent_at)| sent == sequence && now - sent_at < 500.0)
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
            last_sequence_ack_sent: None,
            playback_epoch: 0,
        }
    }
}

pub struct MovementTween {
    pub playback_epoch: u64,
    pub event_index: usize,
    pub path: Vec<MovementWaypoint>,
    pub start_time_ms: f64,
    pub duration_ms: f64,
    pub transition: TransitionConfig,
}

impl MovementTween {
    /// Returns the active axial edge and its local progress. A vertical-only
    /// move has no edge and therefore preserves the existing facing.
    pub fn segment_at(&self, elapsed_ms: f64) -> Option<(MovementWaypoint, MovementWaypoint, f32)> {
        let segments = self.path.len().saturating_sub(1);
        if segments == 0 {
            return None;
        }
        let whole = (elapsed_ms / self.duration_ms.max(1.0)).clamp(0.0, 1.0);
        let scaled = whole * segments as f64;
        let index = (scaled as usize).min(segments - 1);
        Some((
            self.path[index],
            self.path[index + 1],
            (scaled - index as f64) as f32,
        ))
    }
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
    use super::{CameraTween, MovementTween, sequence_ack_due};
    use hexx::Hex;
    use proptest::prelude::*;
    use pystral_core::log::{MovementWaypoint, TransitionConfig, TweenKind};

    fn waypoint(hex: Hex, layer: i32) -> MovementWaypoint {
        MovementWaypoint { hex, layer }
    }

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

    #[test]
    fn sequence_ack_retries_after_transport_timeout() {
        assert!(!sequence_ack_due(Some((12, 100.0)), 12, 599.0));
        assert!(sequence_ack_due(Some((12, 100.0)), 12, 600.0));
        assert!(sequence_ack_due(Some((11, 100.0)), 12, 101.0));
    }

    #[test]
    fn movement_segments_follow_axial_path_in_logical_order() {
        let tween = MovementTween {
            playback_epoch: 0,
            event_index: 0,
            path: Hex::ZERO
                .line_to(Hex::new(2, -1))
                .map(|hex| waypoint(hex, 0))
                .collect(),
            start_time_ms: 0.0,
            duration_ms: 600.0,
            transition: TransitionConfig {
                duration_ms: 600,
                delta_time_ms: 16.0,
                tween: TweenKind::SineInOut,
            },
        };
        assert_eq!(
            tween.segment_at(0.0),
            Some((waypoint(Hex::ZERO, 0), waypoint(Hex::new(1, 0), 0), 0.0))
        );
        assert_eq!(
            tween.segment_at(450.0),
            Some((
                waypoint(Hex::new(1, 0), 0),
                waypoint(Hex::new(2, -1), 0),
                0.5
            ))
        );
    }

    #[test]
    fn movement_segment_preserves_vertical_waypoint_layers() {
        let tween = MovementTween {
            playback_epoch: 0,
            event_index: 0,
            path: vec![waypoint(Hex::ZERO, 0), waypoint(Hex::ZERO, 2)],
            start_time_ms: 0.0,
            duration_ms: 500.0,
            transition: TransitionConfig {
                duration_ms: 500,
                delta_time_ms: 16.0,
                tween: TweenKind::SineInOut,
            },
        };
        assert_eq!(
            tween.segment_at(250.0),
            Some((waypoint(Hex::ZERO, 0), waypoint(Hex::ZERO, 2), 0.5))
        );
    }

    proptest! {
        #[test]
        fn movement_segment_is_always_an_edge_in_the_authored_path(
            duration_ms in 1.0f64..10_000.0,
            elapsed_ms in 0.0f64..20_000.0,
        ) {
            let path = Hex::ZERO.line_to(Hex::new(3, -1)).map(|hex| waypoint(hex, 0)).collect::<Vec<_>>();
            let tween = MovementTween {
                playback_epoch: 7,
                event_index: 12,
                path: path.clone(),
                start_time_ms: 0.0,
                duration_ms,
                transition: TransitionConfig { duration_ms: duration_ms as u32, delta_time_ms: 16.0, tween: TweenKind::SineInOut },
            };
            let (from, to, progress) = tween.segment_at(elapsed_ms).expect("non-vertical path");
            prop_assert!(path.windows(2).any(|edge| edge == [from, to]));
            prop_assert!((0.0..=1.0).contains(&progress));
        }
    }
}
