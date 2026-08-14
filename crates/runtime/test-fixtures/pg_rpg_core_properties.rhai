// X0 smoke fixture: the same authored API can describe rules, unit state, and
// deterministic NPC-boundary properties without duplicating Rust mutators.

fn require(condition, message) {
    if !condition {
        throw(message);
    }
}

fn fixture(seed) {
    let scenario = new_skirmish_config(seed).with_builtin_script_registry();
    scenario.add_unit(1, 1, "Caveman", cell(hex(0, 0), 0));
    scenario.add_unit(2, 2, "Mage", cell(hex(4, 0), 0));

    let policy = new_mcts_config();
    policy.set_visits(1);
    policy.set_depth(1);
    policy.set_seed(seed);
    new_simulation(scenario, policy)
}

fn unit_health_property() {
    // Unit-state property: health is observable and the authoritative setter
    // keeps the value normalized through the runtime simulation API.
    let unit_state = fixture(42);
    require(unit_state.get_agent_health(1) > 0, "fixture unit must start alive");
    unit_state.set_agent_health(1, 0);
    require(unit_state.get_agent_health(1) == 0, "health-zero state must be observable");
}

fn deterministic_boundary_property() {
    // Identical scenario and MCTS seeds must expose the same first boundary,
    // independent of the Rust test's traversal.
    let left = fixture(42);
    let right = fixture(42);
    let left_boundary = left.advance_to_boundary_budgeted(128);
    let right_boundary = right.advance_to_boundary_budgeted(128);
    require(left_boundary == right_boundary, "identical seeds must reach the same boundary");
    require(left_boundary.len() > 0, "fixture must reach a control boundary");
}

fn npc_decision_provenance_property() {
    let left = fixture(42);
    let right = fixture(42);
    let left_trace = left.get_npc_decision_trace(2);
    let right_trace = right.get_npc_decision_trace(2);
    require(
        left_trace["snapshot_fingerprint"] == left.get_snapshot_fingerprint(),
        "NPC trace must identify its source snapshot",
    );
    require(
        left_trace == right_trace,
        "identical snapshots and seeds must produce identical NPC traces",
    );
    require(left_trace["action"] != "", "NPC trace must expose a candidate action");
}
