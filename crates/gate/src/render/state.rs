use hexx::Hex;
use pystral_core::log::PropertyValue;

pub struct PlaybackState {
    pub playing_log: bool,
    pub playing_animations: bool,
    pub debug_mode: bool,
    pub last_tick_ms: f64,
    pub last_debug_mode: bool,
    pub last_history_log_len: usize,
    pub last_debug_index: usize,
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
        }
    }
}

pub struct MovementTween {
    pub from_hex: Hex,
    pub to_hex: Hex,
    pub start_time_ms: f64,
    pub duration_ms: f64,
}

pub struct PropertyTween {
    pub property: String,
    pub from_value: PropertyValue,
    pub to_value: PropertyValue,
    pub start_time_ms: f64,
    pub duration_ms: f64,
}
