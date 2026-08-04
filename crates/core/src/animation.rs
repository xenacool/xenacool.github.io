use crate::log::PropertyValue;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PropertyTrack {
    pub property: String,
    pub keyframes: Vec<Keyframe>,
    pub loop_behavior: LoopBehavior,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Keyframe {
    pub time_ms: f32,
    pub value: PropertyValue,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum LoopBehavior {
    None,
    Loop,
    PingPong,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnimationState {
    pub name: String,
    pub tracks: Vec<PropertyTrack>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InactiveFSMDefinition {
    pub states: HashMap<String, AnimationState>,
}

pub struct ActiveFSM {
    pub definition: InactiveFSMDefinition,
    pub current_state_name: String,
    pub state_start_time_ms: f64,
    pub current_properties: HashMap<String, PropertyValue>,
}

impl ActiveFSM {
    pub fn new(definition: InactiveFSMDefinition, initial_state: String, now_ms: f64) -> Self {
        Self {
            definition,
            current_state_name: initial_state,
            state_start_time_ms: now_ms,
            current_properties: HashMap::new(),
        }
    }

    pub fn transition_to(&mut self, target_state: String, now_ms: f64) {
        if self.current_state_name == target_state {
            return;
        }

        self.current_state_name = target_state;
        self.state_start_time_ms = now_ms;
    }

    pub fn update(&mut self, now_ms: f64) {
        self.current_properties = self.get_state_properties(&self.current_state_name, now_ms);
    }

    fn get_state_properties(
        &self,
        state_name: &str,
        now_ms: f64,
    ) -> HashMap<String, PropertyValue> {
        let mut props = HashMap::new();
        if let Some(state) = self.definition.states.get(state_name) {
            #[allow(clippy::cast_possible_truncation)]
            let elapsed = (now_ms - self.state_start_time_ms) as f32;
            for track in &state.tracks {
                if let Some(val) = evaluate_track(track, elapsed) {
                    props.insert(track.property.clone(), val);
                }
            }
        }
        props
    }
}

fn evaluate_track(track: &PropertyTrack, time_ms: f32) -> Option<PropertyValue> {
    if track.keyframes.is_empty() {
        return None;
    }
    if track.keyframes.len() == 1 {
        return Some(track.keyframes[0].value.clone());
    }

    let total_duration = track.keyframes.last().expect("track has keyframes").time_ms;
    if total_duration <= 0.0 {
        return Some(track.keyframes[0].value.clone());
    }

    let t = match track.loop_behavior {
        LoopBehavior::None => time_ms.min(total_duration),
        LoopBehavior::Loop => time_ms % total_duration,
        LoopBehavior::PingPong => {
            #[allow(clippy::cast_possible_truncation)]
            let cycle = (time_ms / total_duration) as i32;
            let rem = time_ms % total_duration;
            if cycle % 2 == 0 {
                rem
            } else {
                total_duration - rem
            }
        }
    };

    // Find the two keyframes to interpolate between
    for i in 0..track.keyframes.len() - 1 {
        let k1 = &track.keyframes[i];
        let k2 = &track.keyframes[i + 1];
        if t >= k1.time_ms && t <= k2.time_ms {
            let local_t = (t - k1.time_ms) / (k2.time_ms - k1.time_ms);
            return Some(interpolate(&k1.value, &k2.value, local_t));
        }
    }

    Some(
        track
            .keyframes
            .last()
            .expect("track has keyframes")
            .value
            .clone(),
    )
}

pub fn interpolate(start: &PropertyValue, end: &PropertyValue, t: f32) -> PropertyValue {
    match (start, end) {
        (PropertyValue::Float(s), PropertyValue::Float(e)) => PropertyValue::Float(s + (e - s) * t),
        (PropertyValue::Vec3(s), PropertyValue::Vec3(e)) => PropertyValue::Vec3(
            glam::Vec3::from_array(s.to_array())
                + (glam::Vec3::from_array(e.to_array()) - glam::Vec3::from_array(s.to_array())) * t,
        ),
        (PropertyValue::Color(s), PropertyValue::Color(e)) => {
            let r = s[0] + (e[0] - s[0]) * t;
            let g = s[1] + (e[1] - s[1]) * t;
            let b = s[2] + (e[2] - s[2]) * t;
            PropertyValue::Color([r, g, b])
        }
        _ => end.clone(),
    }
}
