use super::*;
use proptest::prop_assert_eq;
use pystral_core::log::{AvailableActions, AvailableJobActions, Event};

#[test]
fn simulation_retry_waits_for_a_bounded_heartbeat_window() {
    assert!(!simulation_retry_due(0));
    assert!(!simulation_retry_due(7));
    assert!(simulation_retry_due(8));
    assert!(simulation_retry_due(u8::MAX));
}

#[test]
fn availability_history_becomes_transient_state() {
    let mut history = pystral_core::history::HistoryManager::new();
    history.push_and_apply(Event::AvailableActions(AvailableActions {
        unit_id: 7,
        movement: Vec::new(),
        primary_job: AvailableJobActions {
            name: "Caveman".into(),
            abilities: Vec::new(),
        },
        secondary_jobs: Vec::new(),
    }));

    let transient = transient_state_from_history(&history).unwrap();
    assert_eq!(transient.active_unit_id, Some(7));
    assert_eq!(
        transient.available_actions.unwrap().primary_job.name,
        "Caveman"
    );
    assert!(transient.menu_path.is_empty());
    assert!(transient.preview.is_none());
}

#[test]
fn unrelated_history_ack_does_not_match_animation_barrier() {
    assert_eq!(matching_animation_barrier(Some((12, true)), 11), None);
    assert_eq!(
        matching_animation_barrier(Some((12, true)), 12),
        Some((12, true))
    );
    assert_eq!(
        matching_animation_barrier(Some((12, true)), 13),
        Some((12, true))
    );
    assert_eq!(matching_animation_barrier(None, 12), None);
}

#[test]
fn animation_ack_watermark_survives_barrier_publish_race() {
    let highest_ack = 13;
    assert_eq!(
        matching_animation_barrier(Some((13, true)), highest_ack),
        Some((13, true))
    );
    assert_eq!(
        matching_animation_barrier(Some((14, true)), highest_ack),
        None
    );
}

#[test]
fn status_explains_idle_and_blocking_states_with_stable_precedence() {
    assert_eq!(status_for(false, false, false, false), WorkerStatus::Idle);
    assert_eq!(
        status_for(false, false, false, true),
        WorkerStatus::AwaitingPlayerDecision
    );
    assert_eq!(
        status_for(false, false, true, false),
        WorkerStatus::Simulating
    );
    assert_eq!(
        status_for(false, true, true, true),
        WorkerStatus::WaitingForAnimationAck
    );
    assert_eq!(status_for(true, true, true, true), WorkerStatus::Completed);
}

#[test]
fn terminal_animation_ack_resumes_boundary_even_without_wait() {
    assert!(animation_ack_resumes_boundary(
        false,
        &RuntimeContinuation::AwaitBoundary
    ));
    assert!(animation_ack_resumes_boundary(
        true,
        &RuntimeContinuation::AwaitPlayerDecision { unit_id: 1 }
    ));
    assert!(!animation_ack_resumes_boundary(
        false,
        &RuntimeContinuation::AwaitPlayerDecision { unit_id: 1 }
    ));
}

proptest::proptest! {
    #[test]
    fn stale_simulation_response_never_matches(
        expected in 0u64..1000,
        received in 0u64..1000,
    ) {
        prop_assert_eq!(
            simulation_response_matches(expected, received),
            expected == received,
        );
    }
}

proptest::proptest! {
    #[test]
    fn status_is_never_reported_as_idle_while_work_is_owned(
        completed: bool,
        waiting_for_ack: bool,
        simulating: bool,
        available_actions: bool,
    ) {
        let status = status_for(completed, waiting_for_ack, simulating, available_actions);
        if completed {
            prop_assert_eq!(status, WorkerStatus::Completed);
        } else if waiting_for_ack {
            prop_assert_eq!(status, WorkerStatus::WaitingForAnimationAck);
        } else if simulating {
            prop_assert_eq!(status, WorkerStatus::Simulating);
        } else if available_actions {
            prop_assert_eq!(status, WorkerStatus::AwaitingPlayerDecision);
        } else {
            prop_assert_eq!(status, WorkerStatus::Idle);
        }
    }
}
