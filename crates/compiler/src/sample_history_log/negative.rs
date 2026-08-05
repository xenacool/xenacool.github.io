use pystral_core::history::HistoryManager;
use pystral_core::log::{Event, PropertyValue};
use hexx::Hex;

pub fn generate_asymmetric_camera_log(history: &mut HistoryManager) {
    // Camera 1
    history.push_and_apply(Event::SpawnEntity { id: 20, kind: "camera".to_string(), hex: Hex::ZERO });
    history.push_and_apply(Event::UpdateProperty { id: 20, property: "neighbor_right".to_string(), value: PropertyValue::Float(21.0) });

    // Camera 2
    history.push_and_apply(Event::SpawnEntity { id: 21, kind: "camera".to_string(), hex: Hex::ZERO });
    // MISSING neighbor_left back to 20
}

pub fn generate_broken_chain_camera_log(history: &mut HistoryManager) {
    // Camera 1
    history.push_and_apply(Event::SpawnEntity { id: 30, kind: "camera".to_string(), hex: Hex::ZERO });
    history.push_and_apply(Event::UpdateProperty { id: 30, property: "neighbor_right".to_string(), value: PropertyValue::Float(31.0) });

    // Camera 2
    history.push_and_apply(Event::SpawnEntity { id: 31, kind: "camera".to_string(), hex: Hex::ZERO });
    history.push_and_apply(Event::UpdateProperty { id: 31, property: "neighbor_left".to_string(), value: PropertyValue::Float(30.0) });
    history.push_and_apply(Event::UpdateProperty { id: 31, property: "neighbor_up".to_string(), value: PropertyValue::Float(32.0) });

    // Camera 3
    history.push_and_apply(Event::SpawnEntity { id: 32, kind: "camera".to_string(), hex: Hex::ZERO });
    // Camera 3 points back to 30 instead of 31
    history.push_and_apply(Event::UpdateProperty { id: 32, property: "neighbor_down".to_string(), value: PropertyValue::Float(30.0) });
}
