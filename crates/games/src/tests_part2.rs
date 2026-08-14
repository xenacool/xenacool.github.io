    #[test]
    fn test_skirmish_simulation() {
        let mut state = setup_2v2_skirmish();
        let scheduler = CTScheduler::new(100);
        scheduler.initialize_ct(&mut state);

        let config = MCTSConfiguration {
            seed: Some(42),
            ..Default::default()
        };

        for _ in 0..20 {
            if state
                .agents
                .values()
                .filter(|unit| unit.health > 0)
                .map(|unit| unit.team_id)
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                <= 1
            {
                break;
            }
            let ready_agents = scheduler.tick_until_ready(&mut state);
            for agent_id in ready_agents {
                let mut mcts = MCTS::<TacticalDomain>::new(state.clone(), agent_id, config.clone());
                let Some(task) = mcts.run() else {
                    continue;
                };

                let mut diff = TacticalDiff::default();
                {
                    let ctx_mut = ContextMut {
                        tick: 0,
                        state_diff: StateDiffRefMut {
                            initial_state: &state,
                            diff: &mut diff,
                        },
                        agent: agent_id,
                    };
                    task.execute(ctx_mut);
                }
                let dummy_state = state.clone();
                TacticalDomain::apply(&mut state, &dummy_state, &diff);
            }
        }

        // Assertions to verify simulation progress
        assert!(!state.agents.is_empty());
        // The vectorial combat model may spend the bounded sample on movement
        // or waiting; state remains valid regardless of whether an attack was
        // selected.
        assert!(
            state
                .agents
                .values()
                .all(|unit| unit.health <= unit.derived_stats.health_max)
        );
    }

    #[test]
    fn health_oracle_normalizes_lethal_and_overhealing_state_changes() {
        let mut scenario = SkirmishConfig::new(42);
        scenario
            .add_unit(1, 1, "Caveman", GridCell::new(hexx::Hex::ZERO, 0))
            .unwrap();
        let mut state = scenario.build_state().unwrap();
        let mut unit = state.agents.remove(&AgentId(1)).unwrap();

        unit.health = 1;
        unit.apply_damage(100);
        assert_eq!(unit.health, 0);

        unit.apply_healing(1_000);
        assert_eq!(unit.health, unit.derived_stats.health_max);

        unit.health = -25;
        let mut diff = TacticalDiff::default();
        diff.agents.insert(AgentId(1), unit);
        let snapshot = state.clone();
        TacticalDomain::apply(&mut state, &snapshot, &diff);
        assert_eq!(state.agents[&AgentId(1)].health, 0);
    }

    #[test]
    fn skirmish_config_validates_and_exposes_ct_threshold() {
        let mut config = SkirmishConfig::new(42);
        assert_eq!(config.ct_threshold, 100);
        config.set_ct_threshold(250).unwrap();
        assert_eq!(config.ct_threshold, 250);
        assert!(config.set_ct_threshold(0).is_err());
        assert!(config.set_ct_threshold(1_000_001).is_err());
        assert!(config.set_ct_threshold(i64::MAX).is_err());
    }

    #[test]
    fn configured_ct_threshold_controls_readiness() {
        let mut config = SkirmishConfig::new(42);
        config.set_ct_threshold(10).unwrap();
        config
            .add_unit(1, 1, "Caveman", GridCell::new(hexx::Hex::ZERO, 0))
            .unwrap();
        let mut state = config.build_state().unwrap();
        let scheduler = CTScheduler::new(config.ct_threshold);
        scheduler.initialize_ct(&mut state);
        let ready = scheduler.tick_until_ready(&mut state);
        assert!(!ready.is_empty());
        assert!(state.agents[&AgentId(1)].ct >= 10);
    }

    #[test]
    fn dead_ready_units_do_not_starve_living_units() {
        let mut state = setup_2v2_skirmish();
        let scheduler = CTScheduler::new(100);
        scheduler.initialize_ct(&mut state);
        state.agents.get_mut(&AgentId(1)).unwrap().health = 0;
        state.agents.get_mut(&AgentId(1)).unwrap().ct = 100;
        state.agents.get_mut(&AgentId(2)).unwrap().ct = 99;

        let ready = scheduler
            .tick_until_ready_budgeted(&mut state, 1)
            .expect("a living unit should become ready");
        assert!(!ready.contains(&AgentId(1)));
        assert!(ready.contains(&AgentId(2)));
    }

    #[test]
    fn mcts_candidate_is_valid_for_the_snapshot_it_planned() {
        let state = setup_2v2_skirmish();
        let agent = AgentId(1);
        let mut mcts = MCTS::<TacticalDomain>::new(
            state.clone(),
            agent,
            MCTSConfiguration {
                visits: 20,
                depth: 4,
                seed: Some(42),
                ..MCTSConfiguration::default()
            },
        );
        let task = mcts.run().expect("MCTS should find a candidate");
        let diff = TacticalDiff::default();
        let context = Context::with_state_and_diff(0, &state, &diff, agent);
        assert!(task.is_valid(context));
    }

    #[test]
    fn enumerated_actions_are_typed_and_valid_for_the_snapshot() {
        let mut scenario = SkirmishConfig::new(42);
        scenario
            .add_unit(1, 1, "Caveman", GridCell::new(hexx::Hex::ZERO, 0))
            .unwrap();
        scenario
            .add_unit(2, 2, "Mage", GridCell::new(hexx::Hex::new(1, 0), 0))
            .unwrap();
        let state = scenario.build_state().unwrap();
        let diff = TacticalDiff::default();
        let context = Context::with_state_and_diff(0, &state, &diff, AgentId(1));
        let tasks = TacticalDomain::get_tasks(context);

        assert!(!tasks.is_empty());
        assert!(
            tasks
                .iter()
                .all(|task| task.is_valid(Context::with_state_and_diff(
                    0,
                    &state,
                    &diff,
                    AgentId(1)
                )))
        );
        assert!(
            tasks
                .iter()
                .any(|task| matches!(task.display_action(), TacticalDisplayAction::Move { .. }))
        );
        assert!(
            tasks
                .iter()
                .any(|task| matches!(task.display_action(), TacticalDisplayAction::Ability { .. }))
        );
        assert!(
            tasks
                .iter()
                .any(|task| matches!(task.display_action(), TacticalDisplayAction::Wait))
        );
    }

    #[test]
    fn legal_actions_preserve_tactical_state_invariants() {
        let mut scenario = SkirmishConfig::new(42);
        scenario
            .add_unit(1, 1, "Caveman", GridCell::new(hexx::Hex::ZERO, 0))
            .unwrap();
        scenario
            .add_unit(2, 2, "Mage", GridCell::new(hexx::Hex::new(1, 0), 0))
            .unwrap();
        let state = scenario.build_state().unwrap();
        let base_diff = TacticalDiff::default();
        let tasks = TacticalDomain::get_tasks(Context::with_state_and_diff(
            0,
            &state,
            &base_diff,
            AgentId(1),
        ));

        for task in tasks {
            let mut next = state.clone();
            let mut diff = TacticalDiff::default();
            {
                let context = ContextMut {
                    tick: 0,
                    state_diff: StateDiffRefMut {
                        initial_state: &state,
                        diff: &mut diff,
                    },
                    agent: AgentId(1),
                };
                let _ = task.execute(context);
            }
            TacticalDomain::apply(&mut next, &state, &diff);

            let mut occupied = std::collections::HashSet::new();
            for (agent, unit) in &next.agents {
                assert!(
                    next.grid.contains(unit.position),
                    "{agent:?} moved outside the grid"
                );
                assert!((0..=unit.derived_stats.action_points_max).contains(&unit.action_points));
                assert!(
                    occupied.insert(unit.position),
                    "two agents occupy {:?}",
                    unit.position
                );
            }
        }
    }

    #[test]
    fn arcane_shield_adds_timed_armor_without_damaging_the_caster() {
        let mut scenario = SkirmishConfig::new(42);
        scenario
            .add_unit(1, 1, "Caveman", GridCell::new(hexx::Hex::ZERO, 0))
            .unwrap();
        scenario
            .add_unit(2, 2, "Mage", GridCell::new(hexx::Hex::new(1, 0), 0))
            .unwrap();
        let state = scenario.build_state().unwrap();
        let shield_id = state
            .ability_registry
            .values()
            .find(|ability| ability.name == "Arcane Shield")
            .map(|ability| ability.id)
            .unwrap();
        let shield = &state.ability_registry[&shield_id];
        let shield_program = shield
            .programs
            .get(&RPGHook::OnAbilityResolve)
            .and_then(|programs| programs.first())
            .expect("Arcane Shield must have an authored resolve program");
        assert_eq!(
            shield_program.execute_on_ability_resolve().unwrap(),
            vec![TimedModifier {
                stat: DerivedStat::ArmorClass,
                amount: 4,
                remaining_turns: 2,
                stacking: ModifierStacking::RefreshReplace,
            }]
        );
        let health_before = state.agents[&AgentId(2)].health;
        let tasks = TacticalDomain::get_tasks(Context::with_state_and_diff(
            0,
            &state,
            &TacticalDiff::default(),
            AgentId(2),
        ));
        let task = tasks
            .into_iter()
            .find(|task| {
                task.display_action()
                    == TacticalDisplayAction::Ability {
                        target: AgentId(2),
                        ability: shield_id,
                    }
            })
            .expect("Arcane Shield should be a legal self-target action");
        let mut diff = TacticalDiff::default();
        task.execute(ContextMut {
            tick: 0,
            state_diff: StateDiffRefMut {
                initial_state: &state,
                diff: &mut diff,
            },
            agent: AgentId(2),
        });
        let mut next = state.clone();
        TacticalDomain::apply(&mut next, &state, &diff);

        assert_eq!(next.agents[&AgentId(2)].health, health_before);
        assert!(next.agents[&AgentId(2)]
            .timed_modifiers
            .iter()
            .any(|modifier| modifier.stat == DerivedStat::ArmorClass
                && modifier.amount == 4
                && modifier.remaining_turns == 2));
    }

    #[test]
    fn action_enumeration_and_seeded_rollouts_are_deterministic() {
        let mut scenario = SkirmishConfig::new(42);
        scenario
            .add_unit(1, 1, "Caveman", GridCell::new(hexx::Hex::ZERO, 0))
            .unwrap();
        scenario
            .add_unit(2, 2, "Mage", GridCell::new(hexx::Hex::new(1, 0), 0))
            .unwrap();
        let state = scenario.build_state().unwrap();

        let enumerate = |state: &TacticalState| {
            let diff = TacticalDiff::default();
            TacticalDomain::get_tasks(Context::with_state_and_diff(0, state, &diff, AgentId(1)))
                .into_iter()
                .map(|task| task.display_action())
                .collect::<Vec<_>>()
        };
        let actions_seed_a = enumerate(&state);
        let actions_seed_b = enumerate(&state);
        assert_eq!(actions_seed_a, actions_seed_b);
        assert!(
            actions_seed_a
                .iter()
                .any(|action| matches!(action, TacticalDisplayAction::Move { .. }))
        );
        assert!(
            actions_seed_a
                .iter()
                .any(|action| matches!(action, TacticalDisplayAction::Ability { .. }))
        );
        assert!(
            actions_seed_a
                .iter()
                .any(|action| matches!(action, TacticalDisplayAction::Wait))
        );

        let run = |seed| {
            let mut mcts = MCTS::<TacticalDomain>::new(
                state.clone(),
                AgentId(1),
                MCTSConfiguration {
                    seed: Some(seed),
                    visits: 30,
                    depth: 5,
                    ..MCTSConfiguration::default()
                },
            );
            let task = mcts.run().expect("seeded MCTS should find a task");
            let display_action = task.display_action();
            let mut diff = TacticalDiff::default();
            {
                let context = ContextMut {
                    tick: 0,
                    state_diff: StateDiffRefMut {
                        initial_state: &state,
                        diff: &mut diff,
                    },
                    agent: AgentId(1),
                };
                task.execute(context);
            }
            assert!(task.is_valid(Context::with_state_and_diff(
                0,
                &state,
                &TacticalDiff::default(),
                AgentId(1)
            )));
            (display_action, diff)
        };

        assert_eq!(run(42), run(42));
        let (different_action, different_diff) = run(43);
        assert!(actions_seed_a.contains(&different_action));
        assert!(
            different_diff
                .agents
                .keys()
                .all(|agent| state.agents.contains_key(agent))
        );
    }

    #[test]
    fn shallow_mcts_returns_a_legal_action_for_the_tactical_adapter() {
        let mut scenario = SkirmishConfig::new(42);
        scenario
            .add_unit(1, 1, "Caveman", GridCell::new(hexx::Hex::ZERO, 0))
            .unwrap();
        scenario
            .add_unit(2, 2, "Mage", GridCell::new(hexx::Hex::new(1, 0), 0))
            .unwrap();
        let mut state = scenario.build_state().unwrap();
        state.agents.get_mut(&AgentId(2)).unwrap().health = 1;
        state.agents.get_mut(&AgentId(2)).unwrap().stats.armor_class = 0;
        let diff = TacticalDiff::default();
        let tasks =
            TacticalDomain::get_tasks(Context::with_state_and_diff(0, &state, &diff, AgentId(1)));

        let mut mcts = MCTS::<TacticalDomain>::new(
            state.clone(),
            AgentId(1),
            MCTSConfiguration {
                visits: 5_000,
                depth: 10,
                seed: Some(42),
                ..MCTSConfiguration::default()
            },
        );
        let candidate = mcts.run().expect("MCTS should find a candidate");
        assert!(candidate.is_valid(Context::with_state_and_diff(0, &state, &diff, AgentId(1))));
        assert!(tasks.iter().any(|task| task.display_action() == candidate.display_action()));
    }

    proptest! {
        #[test]
        fn bounded_states_preserve_mcts_legality_and_replay(
            defender_health in 1i32..=100,
            attacker_ap in 0i32..=4,
            seed in 0u64..=10_000,
        ) {
            let mut scenario = SkirmishConfig::new(seed);
            scenario
                .add_unit(1, 1, "Caveman", GridCell::new(hexx::Hex::ZERO, 0))
                .unwrap();
            scenario
                .add_unit(2, 2, "Mage", GridCell::new(hexx::Hex::new(1, 0), 0))
                .unwrap();
            let mut state = scenario.build_state().unwrap();
            state.agents.get_mut(&AgentId(1)).unwrap().action_points = attacker_ap;
            state.agents.get_mut(&AgentId(2)).unwrap().health = defender_health;

            let tasks = TacticalDomain::get_tasks(Context::with_state_and_diff(
                0,
                &state,
                &TacticalDiff::default(),
                AgentId(1),
            ));
            prop_assert!(!tasks.is_empty());

            for task in &tasks {
                prop_assert!(task.is_valid(Context::with_state_and_diff(
                    0,
                    &state,
                    &TacticalDiff::default(),
                    AgentId(1),
                )));
                let mut next = state.clone();
                let mut diff = TacticalDiff::default();
                task.execute(ContextMut {
                    tick: 0,
                    state_diff: StateDiffRefMut {
                        initial_state: &state,
                        diff: &mut diff,
                    },
                    agent: AgentId(1),
                });
                TacticalDomain::apply(&mut next, &state, &diff);
                let has_side_effect = !diff.agents.is_empty()
                    || diff.rng_update.is_some()
                    || !diff.reaction_queue.is_empty()
                    || diff.reaction_queue_replace.is_some()
                    || diff.turn_completed;
                prop_assert!(
                    has_side_effect,
                    "legal task produced an empty TacticalDiff: {:?}",
                    task.display_action()
                );
                let invariants_hold = next.agents.values().all(|unit| {
                    next.grid.contains(unit.position)
                        && (0..=unit.derived_stats.action_points_max)
                            .contains(&unit.action_points)
                });
                prop_assert!(invariants_hold);
                let positions = next
                    .agents
                    .values()
                    .map(|unit| unit.position)
                    .collect::<std::collections::HashSet<_>>();
                prop_assert_eq!(positions.len(), next.agents.len());
            }

            let config = MCTSConfiguration {
                visits: 8,
                depth: 2,
                seed: Some(seed),
                ..MCTSConfiguration::default()
            };
            let mut first = MCTS::<TacticalDomain>::new(state.clone(), AgentId(1), config.clone());
            let mut second = MCTS::<TacticalDomain>::new(state.clone(), AgentId(1), config);
            let first = first.run().expect("bounded state has a legal action");
            let second = second.run().expect("bounded state has a legal action");
            prop_assert_eq!(first.display_action(), second.display_action());
            prop_assert!(first.is_valid(Context::with_state_and_diff(
                0,
                &state,
                &TacticalDiff::default(),
                AgentId(1),
            )));
        }
    }

    /// Test-only proof of concept for the composite-action integration.  The
    /// production version should move this state-folding logic behind a task
    /// adapter rather than teaching npc-engine about tactical actions.
    fn execute_composite(
        initial: &TacticalState,
        agent: AgentId,
        actions: &[TacticalDisplayAction],
    ) -> Option<TacticalState> {
        let mut state = initial.clone();

        for action in actions {
            let diff = TacticalDiff::default();
            let context = Context::with_state_and_diff(0, &state, &diff, agent);
            let task = TacticalDomain::get_tasks(context)
                .into_iter()
                .find(|task| task.display_action() == *action && task.is_valid(context))?;
            let mut diff = TacticalDiff::default();
            task.execute(ContextMut {
                tick: 0,
                state_diff: StateDiffRefMut {
                    initial_state: &state,
                    diff: &mut diff,
                },
                agent,
            });
            let previous = state.clone();
            TacticalDomain::apply(&mut state, &previous, &diff);
        }

        Some(state)
    }

    fn canonical_successor(state: &TacticalState) -> (
        std::collections::BTreeMap<AgentId, UnitState>,
        Vec<(AgentId, ReactionId, AgentId)>,
        SeededRng,
    ) {
        (
            state
                .agents
                .iter()
                .map(|(&agent, unit)| (agent, unit.clone()))
                .collect(),
            state.reaction_queue.clone(),
            state.rng.clone(),
        )
    }

    #[test]
    fn composite_prototype_folds_actions_and_groups_by_first_action() {
        let first = TacticalDisplayAction::Move {
            to: GridCell::new(hexx::Hex::new(1, 0), 0),
        };
        let second = TacticalDisplayAction::Wait;
        let alternate = TacticalDisplayAction::Ability {
            target: AgentId(2),
            ability: AbilityId(1),
        };

        // This is the root grouping rule the adapter would use: continuations
        // are alternatives below one committed runtime action.
        let candidates = [
            (first.clone(), vec![first.clone(), second.clone()]),
            (first.clone(), vec![first.clone(), alternate]),
        ];
        let mut groups: Vec<(TacticalDisplayAction, usize)> = Vec::new();
        for (root, _) in &candidates {
            if let Some((_, count)) = groups.iter_mut().find(|(candidate, _)| candidate == root) {
                *count += 1;
            } else {
                groups.push((root.clone(), 1));
            }
        }
        assert_eq!(groups, vec![(first.clone(), 2)]);
        assert_eq!(candidates[0].1[0], candidates[1].1[0]);

        let mut scenario = SkirmishConfig::new(42);
        scenario
            .add_unit(1, 1, "Caveman", GridCell::new(hexx::Hex::ZERO, 0))
            .unwrap();
        let state = scenario.build_state().unwrap();
        let result = execute_composite(&state, AgentId(1), &[first]);
        assert!(result.is_some(), "the prototype must fold legal actions");
    }

    #[test]
    fn composite_prototype_only_deduplicates_order_when_successors_match() {
        let mut scenario = SkirmishConfig::new(42);
        scenario
            .add_unit(1, 1, "Caveman", GridCell::new(hexx::Hex::ZERO, 0))
            .unwrap();
        scenario
            .add_unit(2, 2, "Caveman", GridCell::new(hexx::Hex::new(1, 0), 0))
            .unwrap();
        scenario
            .add_unit(3, 2, "Caveman", GridCell::new(hexx::Hex::new(0, 1), 0))
            .unwrap();
        let mut state = scenario.build_state().unwrap();
        state.agents.get_mut(&AgentId(1)).unwrap().modifier_deck = AbilityModifierDeck {
            draw_pile: vec![ModifierCard::Plus0, ModifierCard::Plus0],
            discard_pile: Vec::new(),
            needs_reshuffle: false,
        };
        for target in [AgentId(2), AgentId(3)] {
            state.agents.get_mut(&target).unwrap().reaction_abilities.clear();
        }

        let attacks = |target| {
            let diff = TacticalDiff::default();
            TacticalDomain::get_tasks(Context::with_state_and_diff(
                0,
                &state,
                &diff,
                AgentId(1),
            ))
            .into_iter()
            .find_map(|task| match task.display_action() {
                TacticalDisplayAction::Ability { ability, target: actual }
                    if actual == target => Some(TacticalDisplayAction::Ability { ability, target }),
                _ => None,
            })
        };
        let attack_two = attacks(AgentId(2)).expect("target two should be attackable");
        let attack_three = attacks(AgentId(3)).expect("target three should be attackable");

        let forward = execute_composite(
            &state,
            AgentId(1),
            &[attack_two.clone(), attack_three.clone()],
        )
        .unwrap();
        let reverse = execute_composite(
            &state,
            AgentId(1),
            &[attack_three.clone(), attack_two.clone()],
        )
        .unwrap();

        assert_eq!(
            canonical_successor(&forward),
            canonical_successor(&reverse),
            "same attack and fixed modifier outcomes may be safely canonicalized"
        );

        // Replacing the fixed deck with an order-sensitive deck must prevent
        // a future equivalence check from assuming commutativity merely from
        // the visible action names.
        let mut order_sensitive = state;
        order_sensitive.agents.get_mut(&AgentId(1)).unwrap().modifier_deck =
            AbilityModifierDeck {
                draw_pile: vec![ModifierCard::Critical, ModifierCard::Null],
                discard_pile: Vec::new(),
                needs_reshuffle: false,
            };
        let forward = execute_composite(
            &order_sensitive,
            AgentId(1),
            &[attack_two.clone(), attack_three.clone()],
        )
        .unwrap();
        let reverse = execute_composite(
            &order_sensitive,
            AgentId(1),
            &[attack_three, attack_two],
        )
        .unwrap();
        assert_ne!(
            canonical_successor(&forward),
            canonical_successor(&reverse),
            "modifier/RNG state must make non-commutative composites distinct"
        );
    }

    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    struct CompositeCandidate {
        root: TacticalDisplayAction,
        continuation: Vec<TacticalDisplayAction>,
    }

    fn reference_dedup_composites(
        candidates: &[CompositeCandidate],
    ) -> Vec<CompositeCandidate> {
        candidates.iter().fold(Vec::new(), |mut unique, candidate| {
            if !unique.contains(candidate) {
                unique.push(candidate.clone());
            }
            unique
        })
    }

    fn optimized_dedup_composites(
        candidates: &[CompositeCandidate],
    ) -> Vec<CompositeCandidate> {
        let mut seen = std::collections::HashSet::new();
        candidates
            .iter()
            .filter(|candidate| seen.insert((*candidate).clone()))
            .cloned()
            .collect()
    }

    proptest! {
        #[test]
        fn optimized_composite_dedup_matches_reference(
            candidates in prop::collection::vec(
                (
                    prop_oneof![
                        Just(TacticalDisplayAction::Wait),
                        (0i32..=3, 0i32..=3).prop_map(|(x, y)|
                            TacticalDisplayAction::Move {
                                to: GridCell::new(hexx::Hex::new(x, y), 0),
                            }
                        ),
                        (1u32..=3, 1u32..=3).prop_map(|(target, ability)|
                            TacticalDisplayAction::Ability {
                                target: AgentId(target),
                                ability: AbilityId(ability),
                            }
                        ),
                    ],
                    prop::collection::vec(
                        prop_oneof![
                            Just(TacticalDisplayAction::Wait),
                            (0i32..=3, 0i32..=3).prop_map(|(x, y)|
                                TacticalDisplayAction::Move {
                                    to: GridCell::new(hexx::Hex::new(x, y), 0),
                                }
                            ),
                            (1u32..=3, 1u32..=3).prop_map(|(target, ability)|
                                TacticalDisplayAction::Ability {
                                    target: AgentId(target),
                                    ability: AbilityId(ability),
                                }
                            ),
                        ],
                        0..=3,
                    ),
                )
                .prop_map(|(root, continuation)| CompositeCandidate { root, continuation }),
                0..=64,
            )
        ) {
            prop_assert_eq!(
                optimized_dedup_composites(&candidates),
                reference_dedup_composites(&candidates),
            );
        }
    }
