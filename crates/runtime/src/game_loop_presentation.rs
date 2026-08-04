use super::*;

impl Runtime {
    pub(super) fn projectile_flight_events(
        &mut self,
        profile: String,
        source: GridCell,
        target: GridCell,
    ) -> Vec<Event> {
        let id = 1_000_000_u64.saturating_add(self.next_transient_entity_id);
        self.next_transient_entity_id = self.next_transient_entity_id.saturating_add(1);
        self.pending_projectile_despawns.push(id);
        vec![
            Event::SpawnEntity {
                id,
                kind: "projectile".into(),
                hex: source.hex,
                init_properties: [
                    "scale",
                    "layer",
                    "rotation_x",
                    "rotation_y",
                    "rotation_z",
                    "cam_offset_x",
                    "cam_offset_y",
                    "cam_offset_z",
                    "x_offset",
                    "y_offset",
                    "z_offset",
                    "material",
                    "spritestack",
                ]
                .into_iter()
                .map(String::from)
                .collect(),
            },
            Event::UpdateProperty {
                id,
                property: "asset".into(),
                value: pystral_core::log::PropertyValue::AssetRef(profile),
            },
            Event::UpdateProperty {
                id,
                property: "scale".into(),
                value: pystral_core::log::PropertyValue::Float(0.35),
            },
            Event::UpdateProperty {
                id,
                property: "z".into(),
                value: pystral_core::log::PropertyValue::Float(1.8),
            },
            Event::UpdateProperty {
                id,
                property: "layer".into(),
                value: pystral_core::log::PropertyValue::Float(source.layer as f32),
            },
            Event::MoveSprite {
                id,
                destination: target.hex,
                transition: Some(Self::default_movement_transition()),
            },
        ]
    }

    pub(super) fn summon_lifecycle_events(
        before: &pg_rpg::simulation::TacticalSimulation,
        after: &pg_rpg::simulation::TacticalSimulation,
    ) -> Vec<Event> {
        let mut events = Vec::new();
        let mut summoned = after
            .state
            .summons
            .keys()
            .filter(|id| !before.state.summons.contains_key(id))
            .copied()
            .collect::<Vec<_>>();
        summoned.sort();
        for id in summoned {
            let Some(unit) = after.state.agents.get(&id) else {
                continue;
            };
            let kind = after.get_agent_kind(id.0 as i64);
            events.push(Event::SpawnEntity {
                id: id.0 as u64,
                kind: kind.clone(),
                hex: unit.position.hex,
                init_properties: [
                    "scale",
                    "z",
                    "layer",
                    "rotation_x",
                    "rotation_y",
                    "rotation_z",
                    "cam_offset_x",
                    "cam_offset_y",
                    "cam_offset_z",
                    "x_offset",
                    "y_offset",
                    "z_offset",
                    "material",
                    "spritestack",
                ]
                .into_iter()
                .map(String::from)
                .collect(),
            });
            for (property, value) in [
                ("asset", pystral_core::log::PropertyValue::AssetRef(kind)),
                ("scale", pystral_core::log::PropertyValue::Float(0.6)),
                (
                    "layer",
                    pystral_core::log::PropertyValue::Float(unit.position.layer as f32),
                ),
                (
                    "material",
                    pystral_core::log::PropertyValue::String("grass".into()),
                ),
            ] {
                events.push(Event::UpdateProperty {
                    id: id.0 as u64,
                    property: property.into(),
                    value,
                });
            }
        }
        let mut destroyed = before
            .state
            .summons
            .keys()
            .filter(|id| {
                before
                    .state
                    .agents
                    .get(id)
                    .is_some_and(|unit| unit.health > 0)
                    && after
                        .state
                        .agents
                        .get(id)
                        .is_none_or(|unit| unit.health <= 0)
            })
            .copied()
            .collect::<Vec<_>>();
        destroyed.sort();
        events.extend(
            destroyed
                .into_iter()
                .map(|id| Event::DespawnEntity { id: id.0 as u64 }),
        );
        events
    }
}
