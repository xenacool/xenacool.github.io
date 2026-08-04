use pystral_core::domain::SpritePart;
use pystral_core::log::{Event, PropertyValue};
use pystral_core::history::HistoryManager;
use hexx::Hex;
use crate::character::make_rect_commands;

pub fn setup_rocks(history: &mut HistoryManager) {
    let rock_hexes = vec![
        Hex::new(-4, -4),
        Hex::new(4, -4),
        Hex::new(-4, 4),
        Hex::new(4, 4),
        Hex::new(0, -5),
    ];

    for (i, hex) in rock_hexes.into_iter().enumerate() {
        let id = 10 + i as u64;
        history.push_and_apply(Event::SpawnEntity {
            id,
            kind: "rock".to_string(),
            hex,
        });
        history.push_and_apply(Event::UpdateProperty {
            id,
            property: "scale".to_string(),
            value: PropertyValue::Float(0.5),
        });
        history.push_and_apply(Event::UpdateProperty {
            id,
            property: "z".to_string(),
            value: PropertyValue::Float(0.0),
        });
        history.push_and_apply(Event::UpdateProperty {
            id,
            property: "material".to_string(),
            value: PropertyValue::String("rock".to_string()),
        });
        history.push_and_apply(Event::UpdateProperty {
            id,
            property: "sprite_parts".to_string(),
            value: PropertyValue::SpriteParts(vec![
                SpritePart { 
                    x_prop: "x".into(), 
                    y_prop: "y".into(), 
                    z_prop: "z".into(), 
                    rotation_prop: None, 
                    color: [0.5, 0.5, 0.5], 
                    scale: 1.0, 
                    painter_commands: make_rect_commands([0.4, 0.4, 0.4, 1.0], 64.0, 64.0, 128.0, 128.0),
                    spritestack: None,
                },
            ]),
        });
    }
}

pub fn setup_spritestack_demo(_history: &mut HistoryManager) {
    // Demo is now integrated into setup_character
}
