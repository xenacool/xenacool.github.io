// One resumable scheduler boundary. Rust owns validation and presentation;
// Rhai owns deciding when the tactical simulation advances.
fn resume_game() {
    return sim.advance_to_boundary();
}

fn resume_game_budgeted(budget) {
    return sim.advance_to_boundary_budgeted(budget);
}
