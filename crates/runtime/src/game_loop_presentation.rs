use super::*;

impl Runtime {
    /// Resolve the visual projectile from the same pre-commit tactical state
    /// for human and NPC ability paths. Presentation must not depend on
    /// which controller selected the ability.
    pub(super) fn ability_projectile_route(
        simulation: &pg_rpg::simulation::TacticalSimulation,
        unit_id: u64,
        ability: pystral_games::AbilityId,
        target: &RuntimeAbilityTarget,
    ) -> Option<(String, GridCell, GridCell)> {
        let RuntimeAbilityTarget::Unit { unit_id: target_id } = target else {
            return None;
        };
        let profile = simulation
            .state
            .ability_registry
            .get(&ability)?
            .projectile_profile
            .clone()?;
        let source = simulation
            .state
            .agents
            .get(&npc_engine_core::AgentId(unit_id as u32))?
            .position;
        let target = simulation
            .state
            .agents
            .get(&npc_engine_core::AgentId(*target_id as u32))?
            .position;
        Some((profile, source, target))
    }

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
                path: vec![
                    pystral_core::log::MovementWaypoint {
                        hex: source.hex,
                        layer: source.layer,
                    },
                    pystral_core::log::MovementWaypoint {
                        hex: target.hex,
                        layer: target.layer,
                    },
                ],
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
                (
                    "team_id",
                    pystral_core::log::PropertyValue::Float(unit.team_id as f32),
                ),
                (
                    "primary_job",
                    pystral_core::log::PropertyValue::String(
                        after
                            .state
                            .job_registry
                            .get(&unit.primary_job)
                            .map(|job| job.name.clone())
                            .unwrap_or_default(),
                    ),
                ),
                ("scale", pystral_core::log::PropertyValue::Float(0.6)),
                ("z", pystral_core::log::PropertyValue::Float(0.0)),
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
