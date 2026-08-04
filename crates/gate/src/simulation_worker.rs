use futures::{SinkExt, StreamExt};
use gloo_worker::reactor::{Reactor, ReactorBridge, ReactorScope, ReactorSpawner};
use pystral_runtime::{
    Runtime, RuntimeContinuation, RuntimeRequest, RuntimeResponse, UnitStateInfo,
};
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::{WorkerHeartbeat, WorkerStatus};

const MAX_REQUESTS_PER_POLL: usize = 8;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SimulationEnvelope<T> {
    pub seq: u64,
    pub msg: T,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum SimulationInput {
    Request(SimulationEnvelope<RuntimeRequest>),
    HeartbeatProbe,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SimulationResponse {
    pub request_seq: u64,
    pub response: RuntimeResponse,
    pub logs: Vec<String>,
    pub continuation: RuntimeContinuation,
    pub unit_states: Vec<UnitStateInfo>,
    pub snapshot_fingerprint: Option<u64>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum SimulationOutput {
    Response(SimulationEnvelope<SimulationResponse>),
    Heartbeat(WorkerHeartbeat),
    Watermark(u64),
}

pub struct SimulationWorker {
    scope: ReactorScope<SimulationInput, SimulationOutput>,
    runtime: Runtime,
    next_output_seq: u64,
    last_received_seq: u64,
    outbox: VecDeque<SimulationOutput>,
    last_progress_seq: u64,
    active_request_seq: Option<u64>,
    cached_response: Option<SimulationResponse>,
}

pub(crate) fn accepts_sequence(last_received: u64, incoming: u64) -> bool {
    incoming > last_received
}

pub(crate) fn replays_sequence(
    last_received: u64,
    incoming: u64,
    has_cached_response: bool,
) -> bool {
    incoming == last_received && has_cached_response
}

fn status_for(runtime: &Runtime) -> WorkerStatus {
    match runtime.continuation() {
        pystral_runtime::RuntimeContinuation::Completed => WorkerStatus::Completed,
        pystral_runtime::RuntimeContinuation::AwaitAnimationAck { .. } => {
            WorkerStatus::WaitingForAnimationAck
        }
        pystral_runtime::RuntimeContinuation::AwaitBoundary
        | pystral_runtime::RuntimeContinuation::AwaitMctsDecision { .. } => {
            WorkerStatus::Simulating
        }
        pystral_runtime::RuntimeContinuation::AwaitPlayerDecision { .. } => {
            WorkerStatus::AwaitingPlayerDecision
        }
        pystral_runtime::RuntimeContinuation::RecoverRejected { .. } => WorkerStatus::Idle,
    }
}

impl Reactor for SimulationWorker {
    type Scope = ReactorScope<SimulationInput, SimulationOutput>;

    fn create(scope: Self::Scope) -> Self {
        Self {
            scope,
            runtime: Runtime::new(),
            next_output_seq: 0,
            last_received_seq: 0,
            outbox: VecDeque::new(),
            last_progress_seq: 0,
            active_request_seq: None,
            cached_response: None,
        }
    }
}

impl Future for SimulationWorker {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if !self.flush_outbox(cx) {
            return Poll::Pending;
        }

        let mut processed = 0;
        while processed < MAX_REQUESTS_PER_POLL {
            let Poll::Ready(Some(input)) = self.scope.poll_next_unpin(cx) else {
                break;
            };
            processed += 1;
            match input {
                SimulationInput::HeartbeatProbe => self.push_heartbeat(),
                SimulationInput::Request(envelope) => {
                    if replays_sequence(
                        self.last_received_seq,
                        envelope.seq,
                        self.cached_response.is_some(),
                    ) {
                        let response = self.cached_response.clone().expect("cached response");
                        self.queue_response(envelope.seq, response);
                        self.push_heartbeat();
                        continue;
                    }
                    if !accepts_sequence(self.last_received_seq, envelope.seq) {
                        continue;
                    }
                    self.last_received_seq = envelope.seq;
                    // Publish ownership before entering synchronous runtime
                    // work.  The reactor cannot process another input while
                    // MCTS is running, so this heartbeat is the only useful
                    // liveness signal during that interval.
                    self.active_request_seq = Some(envelope.seq);
                    self.push_heartbeat();
                    let _ = self.flush_outbox(cx);
                    let (response, logs) = self.runtime.process_request(envelope.msg);
                    self.active_request_seq = None;
                    let continuation = self.runtime.continuation();
                    let unit_states = self.runtime.unit_states();
                    let snapshot_fingerprint = self.runtime.snapshot_fingerprint();
                    let response = SimulationResponse {
                        request_seq: envelope.seq,
                        response,
                        logs,
                        continuation,
                        unit_states,
                        snapshot_fingerprint,
                    };
                    self.cached_response = Some(response.clone());
                    self.queue_response(envelope.seq, response);
                    self.last_progress_seq = envelope.seq;
                    self.push_heartbeat();
                }
            }
        }

        // Requests are processed after the initial output-drain phase. Flush
        // responses before returning: if the input stream goes quiet, there
        // may be no later wake-up to revisit an outbox populated in this poll.
        let _ = self.flush_outbox(cx);
        let _ = self.scope.poll_flush_unpin(cx);
        Poll::Pending
    }
}

impl SimulationWorker {
    fn queue_response(&mut self, request_seq: u64, mut response: SimulationResponse) {
        response.request_seq = request_seq;
        let output_seq = self.next_output_seq;
        self.outbox
            .push_back(SimulationOutput::Response(SimulationEnvelope {
                seq: output_seq,
                msg: response,
            }));
        self.next_output_seq += 1;
        self.outbox
            .push_back(SimulationOutput::Watermark(self.last_received_seq));
    }

    fn flush_outbox(&mut self, cx: &mut Context<'_>) -> bool {
        while self.outbox.front().is_some() {
            match self.scope.poll_ready_unpin(cx) {
                Poll::Ready(Ok(())) => {
                    let output = self.outbox.pop_front().expect("outbox is non-empty");
                    let _ = self.scope.start_send_unpin(output);
                }
                Poll::Ready(Err(_)) => {
                    self.outbox.pop_front();
                }
                Poll::Pending => {
                    let _ = self.scope.poll_flush_unpin(cx);
                    return false;
                }
            }
        }
        true
    }

    fn push_heartbeat(&mut self) {
        self.outbox
            .push_back(SimulationOutput::Heartbeat(WorkerHeartbeat {
                latest_output_seq: self.next_output_seq.saturating_sub(1),
                latest_input_seq: self.last_received_seq,
                status: status_for(&self.runtime),
                pending_barrier: None,
                active_request_seq: self.active_request_seq,
                last_progress_seq: self.last_progress_seq,
            }));
    }

    pub fn spawn() -> ReactorBridge<Self> {
        ReactorSpawner::<Self>::new()
            .with_loader(true)
            .spawn("/simulation_worker.js")
    }
}

#[wasm_bindgen::prelude::wasm_bindgen]
pub fn init_simulation_worker() {
    gloo_worker::reactor::ReactorRegistrar::<SimulationWorker>::new().register();
}

#[cfg(test)]
mod tests {
    use super::{accepts_sequence, replays_sequence};
    use proptest::prop_assert_eq;

    proptest::proptest! {
        #[test]
        fn simulation_input_sequences_are_strictly_monotonic(
            previous: u64,
            incoming: u64,
        ) {
            prop_assert_eq!(accepts_sequence(previous, incoming), incoming > previous);
        }
    }

    #[test]
    fn duplicate_sequence_replays_only_when_a_response_is_cached() {
        assert!(replays_sequence(7, 7, true));
        assert!(!replays_sequence(7, 7, false));
        assert!(!replays_sequence(7, 8, true));
        assert!(!replays_sequence(7, 6, true));
    }
}
