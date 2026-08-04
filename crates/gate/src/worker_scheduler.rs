pub(crate) const MAX_INPUTS_PER_POLL: usize = 32;

pub(crate) fn simulation_step_allowed_after_input_drain(processed_inputs: usize) -> bool {
    processed_inputs < MAX_INPUTS_PER_POLL
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prop_assert_eq;

    proptest::proptest! {
        #[test]
        fn input_budget_yields_before_simulation(processed_inputs in 0usize..512) {
            prop_assert_eq!(
                simulation_step_allowed_after_input_drain(processed_inputs),
                processed_inputs < MAX_INPUTS_PER_POLL,
            );
        }
    }
}
