use pystral_core::log::{Event, PropertyValue};
use std::collections::BTreeSet;

pub(super) fn initial_character_assets(events: &[Event]) -> Vec<String> {
    let mut assets = BTreeSet::new();
    for event in events {
        match event {
            Event::UpdateProperty {
                property, value, ..
            } if property == "asset" => {
                if let PropertyValue::String(asset) | PropertyValue::AssetRef(asset) = value {
                    assets.insert(asset.clone());
                }
            }
            _ => {}
        }
    }
    assets.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::initial_character_assets;
    use pystral_core::log::{Event, PropertyValue};

    #[test]
    fn collects_sorted_actor_assets_without_replaying_history() {
        let events = vec![
            Event::UpdateProperty {
                id: 2,
                property: "asset".into(),
                value: PropertyValue::AssetRef("Mage".into()),
            },
            Event::UpdateProperty {
                id: 1,
                property: "asset".into(),
                value: PropertyValue::String("Caveman".into()),
            },
            Event::UpdateProperty {
                id: 1,
                property: "scale".into(),
                value: PropertyValue::Float(1.0),
            },
        ];
        assert_eq!(initial_character_assets(&events), ["Caveman", "Mage"]);
    }
}
