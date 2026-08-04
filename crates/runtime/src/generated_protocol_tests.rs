use super::*;

proptest! {
    #[test]
    fn protocol_safety_holds_for_generated_request_traces(
        commands in prop::collection::vec(any::<u8>(), 1..64)
    ) {
        let mut runtime = runtime_with_unit();

        for command in commands {
            let before_phase = runtime.continuation();
            let before_history_len = runtime.demo_history.as_ref().unwrap().log.len();
            let before_completed = before_phase == RuntimeContinuation::Completed;

            let (response, _logs) = match command % 7 {
                0 => runtime.process_request(RuntimeRequest::StepDemoSimulation),
                1 => runtime.process_request(RuntimeRequest::CommitWait {
                    request_id: u64::from(command),
                    unit_id: 1,
                }),
                2 => runtime.process_request(RuntimeRequest::AcknowledgeAnimation {
                    barrier_id: u64::from(command % 3),
                }),
                3 => runtime.process_request(RuntimeRequest::ResumeBoundary),
                4 => runtime.process_request(RuntimeRequest::CommitDecision {
                    request_id: u64::from(command),
                    decision: RuntimeDecision {
                        unit_id: 1,
                        action: RuntimeDecisionAction::Wait,
                    },
                    provenance: None,
                }),
                5 => runtime.process_request(RuntimeRequest::ResumeRejected {
                    request_id: u64::from(command),
                }),
                _ => runtime.process_request(RuntimeRequest::StepDemoSimulation),
            };

            if command % 7 == 0 && !before_completed
                && before_phase != RuntimeContinuation::AwaitBoundary
            {
                prop_assert!(matches!(response, RuntimeResponse::Error(_)));
            }
            if before_completed {
                prop_assert!(matches!(response, RuntimeResponse::Error(_)));
            }

            let after_history = runtime.demo_history.as_ref().unwrap();
            prop_assert!(after_history.log.len() >= before_history_len);
            let completion_count = after_history
                .log
                .iter()
                .filter(|event| matches!(event, Event::GameCompleted { .. }))
                .count();
            prop_assert!(completion_count <= 1);
            if before_completed {
                prop_assert_eq!(after_history.log.len(), before_history_len);
                prop_assert_eq!(runtime.continuation(), RuntimeContinuation::Completed);
            }
        }
    }
}
