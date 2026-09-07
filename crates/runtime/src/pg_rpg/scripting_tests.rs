use super::*;

#[test]
fn test_invalid_script() {
    let mut history = HistoryManager::new();
    let result = generate_pg_rpg_log_rhai(&mut history, "syntax error here", "", &[], 0);
    assert!(result.is_err());
}

#[test]
fn test_runtime_error_script() {
    let mut history = HistoryManager::new();
    let script = "history.spawn(\"world\", hex(0, 0)); history.non_existent_function();";
    let result = generate_pg_rpg_log_rhai(&mut history, script, "", &[], 0);
    assert!(result.is_err());
}

#[test]
fn ct_threshold_setter_accepts_valid_values_and_rejects_invalid_values() {
    let mut engine = Engine::new();
    register_all(&mut engine);
    engine
        .eval::<()>("let config = new_skirmish_config(42); config.set_ct_threshold(250);")
        .unwrap();
    assert!(
        engine
            .eval::<()>("let config = new_skirmish_config(42); config.set_ct_threshold(0);")
            .is_err()
    );
    assert!(
        engine
            .eval::<()>("let config = new_skirmish_config(42); config.set_ct_threshold(1000001);")
            .is_err()
    );
}

#[test]
fn maximum_turn_count_setter_accepts_unlimited_and_explicit_values() {
    let mut engine = rhai::Engine::new();
    crate::pg_rpg::scripting::register_all(&mut engine);
    engine
        .eval::<()>("let config = new_skirmish_config(42); config.set_maximum_turn_count(12);")
        .unwrap();
    engine
        .eval::<()>("let config = new_skirmish_config(42); config.set_maximum_turn_count(0);")
        .unwrap();
}

#[test]
fn presentation_config_is_fluent_and_validated_at_install() {
    let mut engine = Engine::new();
    register_all(&mut engine);
    let mut scope = rhai::Scope::new();
    scope.push("history", HistoryManager::new());
    engine
        .eval_with_scope::<()>(
            &mut scope,
            r#"
            history.spawn_entity(0, "world", hex(0, 0), []);
            let config = new_presentation_config();
            config.reachable_color = [0.1, 0.2, 0.3];
            config.marker_scale = 1.5;
            config.animation_crossfade_ms = 80;
            config.reduced_motion_policy = "crossfade";
            install_presentation_config(history, config);
        "#,
        )
        .unwrap();
    let history = scope.get_value::<HistoryManager>("history").unwrap();
    assert_eq!(
        history.current_state.entities[0]
            .properties
            .get("presentation_waypoint_reachable_color"),
        Some(&PropertyValue::Color([0.1, 0.2, 0.3]))
    );
    assert_eq!(
        history.current_state.entities[0]
            .properties
            .get("presentation_animation_crossfade_ms"),
        Some(&PropertyValue::Float(80.0))
    );

    let result = engine.eval_with_scope::<()>(
        &mut scope,
        r#"
        let config = new_presentation_config();
        config.reachable_color = [2.0, 0.0, 0.0];
        install_presentation_config(history, config);
    "#,
    );
    assert!(result.is_err());
}

#[test]
fn script_job_schema_can_construct_a_complete_caveman_record() {
    let mut engine = Engine::new();
    register_all(&mut engine);
    engine
        .eval::<()>(
            r#"
        let job = new_script_job("Caveman");
        job.set_base_stats(unit_stats(15, 8, 4, 4, 6, 14, 8, 12, 12, 4));
        job.add_equipment_slot("MainHand");
        job.add_equipment_slot("Body");
        job.add_equipment_slot("Accessory");
        job.set_passive_slots(1);
        job.set_reaction_slots(1);
        job.set_secondary_job_slots(1);
        job.add_ability("Club Smash");
        job.add_ability("Rock Throw");
        job.add_ability("Primal Roar");
        job.add_passive("Thick Skin");
        job.add_reaction("Counter-Swing");
        job.set_movement("Plain Move");
    "#,
        )
        .unwrap();
}
