use hexx::Hex;
use pystral_core::log::PropertyValue;

#[derive(Default)]
pub struct PlaybackState {
    pub playing_log: bool,
    pub playing_animations: bool,
    pub debug_mode: bool,
    pub last_tick_ms: f64,
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
