use super::*;
use npc_engine_core::{AgentId, MCTSConfiguration};
use pystral_games::{GridCell, SkirmishConfig};

fn simulation_with_two_player_units() -> pg_rpg::simulation::TacticalSimulation {
    let mut scenario = SkirmishConfig::new(42);
    scenario
        .add_unit(1, 1, "Caveman", GridCell::new(hexx::Hex::ZERO, 0))
        .unwrap();
    scenario
        .add_unit(2, 1, "Mage", GridCell::new(hexx::Hex::new(1, -1), 0))
        .unwrap();
    scenario
        .add_unit(3, 2, "Mage", GridCell::new(hexx::Hex::new(4, 0), 0))
        .unwrap();
    pg_rpg::simulation::TacticalSimulation::from_scenario(
        scenario,
        MCTSConfiguration {
            seed: Some(42),
            ..Default::default()
        },
    )
}

#[test]
fn boundary_resolution_skips_dead_ready_units() {
    let mut simulation = simulation_with_two_player_units();
    simulation.state.agents.get_mut(&AgentId(1)).unwrap().health = 0;
    assert_eq!(
        resolve_pg_rpg_boundary(&simulation, &[AgentId(1), AgentId(2)]),
        BoundaryResolution::Ready(AgentId(2))
    );
}

#[test]
fn boundary_resolution_completes_before_ready_selection() {
    let mut simulation = simulation_with_two_player_units();
    simulation.state.agents.get_mut(&AgentId(2)).unwrap().health = 0;
    simulation.state.agents.get_mut(&AgentId(3)).unwrap().health = 0;
    assert_eq!(
        resolve_pg_rpg_boundary(&simulation, &[AgentId(1)]),
        BoundaryResolution::Completed(GameOutcome::Victory { winning_team: 1 })
    );
}

#[test]
fn attack_aim_precedes_the_one_shot_cue_without_mutating_tactical_facing() {
    let mut history = HistoryManager::new();
    history.push_and_apply(Event::SpawnEntity {
        id: 1,
        kind: "character".to_string(),
        hex: hexx::Hex::ZERO,
        init_properties: vec![],
    });
    let mut sequence = 0;
    Runtime::append_ability_animation_barrier(
        &mut history,
        &mut sequence,
        1,
        Some("ranged:Ranged_Magic_Shoot"),
        Some(-std::f32::consts::FRAC_PI_6),
    );
    let aim = history.log.iter().position(|event| matches!(event,
        Event::UpdateProperty { property, value: pystral_core::log::PropertyValue::Float(value), .. }
            if property == "presentation_aim_yaw" && (*value + std::f32::consts::FRAC_PI_6).abs() < f32::EPSILON
    )).unwrap();
    let cue = history
        .log
        .iter()
        .position(|event| {
            matches!(event,
                Event::UpdateProperty { property, .. } if property == "animation_cue"
            )
        })
        .unwrap();
    let barrier = history
        .log
        .iter()
        .position(|event| matches!(event, Event::SequenceNumber(1)))
        .unwrap();
    assert!(aim < cue && cue < barrier);
    assert!(!history.log.iter().any(|event| matches!(event,
        Event::UpdateProperty { property, .. } if property == "facing"
    )));
}

#[test]
fn ability_aim_yaw_matches_all_six_pointy_hex_neighbours_and_exact_off_axis_targets() {
    let mut simulation = simulation_with_two_player_units();
    let expected = [
        ((0, -1), std::f32::consts::FRAC_PI_6),
        ((1, -1), -std::f32::consts::FRAC_PI_6),
        ((1, 0), -std::f32::consts::FRAC_PI_2),
        ((0, 1), -std::f32::consts::FRAC_PI_6 * 5.0),
        ((-1, 1), std::f32::consts::FRAC_PI_6 * 5.0),
        ((-1, 0), std::f32::consts::FRAC_PI_2),
    ];
    for ((q, r), yaw) in expected {
        simulation
            .state
            .agents
            .get_mut(&AgentId(2))
            .unwrap()
            .position
            .hex = hexx::Hex::new(q, r);
        assert!(
            (Runtime::ability_presentation_aim_yaw(
                &simulation,
                1,
                &RuntimeAbilityTarget::Unit { unit_id: 2 },
            )
            .unwrap()
                - yaw)
                .abs()
                < 0.0001
        );
    }
    simulation
        .state
        .agents
        .get_mut(&AgentId(2))
        .unwrap()
        .position
        .hex = hexx::Hex::new(1, 1);
    let off_axis = Runtime::ability_presentation_aim_yaw(
        &simulation,
        1,
        &RuntimeAbilityTarget::Unit { unit_id: 2 },
    )
    .unwrap();
    assert!((off_axis + 2.094_395_2).abs() < 0.0001);
    simulation
        .state
        .agents
        .get_mut(&AgentId(2))
        .unwrap()
        .position
        .hex = hexx::Hex::ZERO;
    assert_eq!(
        Runtime::ability_presentation_aim_yaw(
            &simulation,
            1,
            &RuntimeAbilityTarget::Unit { unit_id: 2 },
        ),
        None
    );
}
