    /// Test-only proof of concept for the composite-action integration. The
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
            TacticalDomain::get_tasks(Context::with_state_and_diff(0, &state, &diff, AgentId(1)))
                .into_iter()
                .find_map(|task| match task.display_action() {
                    TacticalDisplayAction::Ability { ability, target: actual } if actual == target => {
                        Some(TacticalDisplayAction::Ability { ability, target })
                    }
                    _ => None,
                })
        };
        let attack_two = attacks(AgentId(2)).expect("target two should be attackable");
        let attack_three = attacks(AgentId(3)).expect("target three should be attackable");
        let forward = execute_composite(&state, AgentId(1), &[attack_two.clone(), attack_three.clone()]).unwrap();
        let reverse = execute_composite(&state, AgentId(1), &[attack_three.clone(), attack_two.clone()]).unwrap();
        assert_eq!(canonical_successor(&forward), canonical_successor(&reverse));

        let mut order_sensitive = state;
        order_sensitive.agents.get_mut(&AgentId(1)).unwrap().modifier_deck = AbilityModifierDeck {
            draw_pile: vec![ModifierCard::Critical, ModifierCard::Null],
            discard_pile: Vec::new(),
            needs_reshuffle: false,
        };
        let forward = execute_composite(&order_sensitive, AgentId(1), &[attack_two.clone(), attack_three.clone()]).unwrap();
        let reverse = execute_composite(&order_sensitive, AgentId(1), &[attack_three, attack_two]).unwrap();
        assert_ne!(canonical_successor(&forward), canonical_successor(&reverse));
    }

    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    struct CompositeCandidate {
        root: TacticalDisplayAction,
        continuation: Vec<TacticalDisplayAction>,
    }

    fn reference_dedup_composites(candidates: &[CompositeCandidate]) -> Vec<CompositeCandidate> {
        candidates.iter().fold(Vec::new(), |mut unique, candidate| {
            if !unique.contains(candidate) { unique.push(candidate.clone()); }
            unique
        })
    }

    fn optimized_dedup_composites(candidates: &[CompositeCandidate]) -> Vec<CompositeCandidate> {
        let mut seen = std::collections::HashSet::new();
        candidates.iter().filter(|candidate| seen.insert((*candidate).clone())).cloned().collect()
    }

    proptest! {
        #[test]
        fn optimized_composite_dedup_matches_reference(
            candidates in prop::collection::vec(
                (
                    prop_oneof![
                        Just(TacticalDisplayAction::Wait),
                        (0i32..=3, 0i32..=3).prop_map(|(x, y)| TacticalDisplayAction::Move {
                            to: GridCell::new(hexx::Hex::new(x, y), 0),
                        }),
                        (1u32..=3, 1u32..=3).prop_map(|(target, ability)| TacticalDisplayAction::Ability {
                            target: AgentId(target), ability: AbilityId(ability),
                        }),
                    ],
                    prop::collection::vec(
                        prop_oneof![
                            Just(TacticalDisplayAction::Wait),
                            (0i32..=3, 0i32..=3).prop_map(|(x, y)| TacticalDisplayAction::Move {
                                to: GridCell::new(hexx::Hex::new(x, y), 0),
                            }),
                            (1u32..=3, 1u32..=3).prop_map(|(target, ability)| TacticalDisplayAction::Ability {
                                target: AgentId(target), ability: AbilityId(ability),
                            }),
                        ],
                        0..=3,
                    ),
                ).prop_map(|(root, continuation)| CompositeCandidate { root, continuation }),
                0..=64,
            )
        ) {
            prop_assert_eq!(optimized_dedup_composites(&candidates), reference_dedup_composites(&candidates));
        }
    }
