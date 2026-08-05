use pystral_core::history::HistoryManager;
use pystral_core::log::{Event, PropertyValue};
use hexx::Hex;

pub fn generate_valid_camera_log(history: &mut HistoryManager) {
    // Camera 1
    history.push_and_apply(Event::SpawnEntity { id: 10, kind: "camera".to_string(), hex: Hex::ZERO });
    history.push_and_apply(Event::UpdateProperty { id: 10, property: "neighbor_right".to_string(), value: PropertyValue::Float(11.0) });

    // Camera 2
    history.push_and_apply(Event::SpawnEntity { id: 11, kind: "camera".to_string(), hex: Hex::ZERO });
    history.push_and_apply(Event::UpdateProperty { id: 11, property: "neighbor_left".to_string(), value: PropertyValue::Float(10.0) });
    history.push_and_apply(Event::UpdateProperty { id: 11, property: "neighbor_up".to_string(), value: PropertyValue::Float(12.0) });

    // Camera 3
    history.push_and_apply(Event::SpawnEntity { id: 12, kind: "camera".to_string(), hex: Hex::ZERO });
    history.push_and_apply(Event::UpdateProperty { id: 12, property: "neighbor_down".to_string(), value: PropertyValue::Float(11.0) });
}
