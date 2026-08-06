use crate::{ReliableInput, ReliableOutput, WorkerInput, WorkerOutput, Envelope};
use gloo_worker::reactor::{Reactor, ReactorScope};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::collections::VecDeque;
use futures::{StreamExt, SinkExt};
use pystral_runtime::Runtime;
use wasm_bindgen::prelude::*;

pub struct UnifiedWorker {
    scope: ReactorScope<ReliableInput, ReliableOutput>,
    logger: pystral_core::ui_log::Logger,
    runtime: Runtime,
    next_output_seq: u64,
    last_received_seq: u64,
    outbox: VecDeque<ReliableOutput>,
}

impl Reactor for UnifiedWorker {
    type Scope = ReactorScope<ReliableInput, ReliableOutput>;

    fn create(scope: Self::Scope) -> Self {
        Self {
            scope,
            logger: pystral_core::ui_log::Logger::new(),
            runtime: Runtime::new(),
            next_output_seq: 0,
            last_received_seq: 0,
            outbox: VecDeque::new(),
        }
    }
}

impl Future for UnifiedWorker {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // web_sys::console::log_1(&"Worker polling...".into());

        // Drain outbox
        while self.outbox.front().is_some() {
            match self.scope.poll_ready_unpin(cx) {
                Poll::Ready(Ok(())) => {
                    let msg = self.outbox.pop_front().expect("Outbox should not be empty");
                    let _ = self.scope.start_send_unpin(msg);
                }
                Poll::Ready(Err(_)) => {
                    self.outbox.pop_front();
                }
                Poll::Pending => {
                    let _ = self.scope.poll_flush_unpin(cx);
                    return Poll::Pending;
                }
            }
        }

        let mut received_any = false;
        while let Poll::Ready(Some(input)) = self.scope.poll_next_unpin(cx) {
            received_any = true;
            match input {
                ReliableInput::Msg(envelope) => {
                    self.last_received_seq = envelope.seq;
                    
                    let response = match envelope.msg {
                        WorkerInput::Log(msg) => {
                            self.logger.apply_command(pystral_core::ui_log::LogCommand::Log(msg));
                            Some(WorkerOutput::LogUpdate {
                                messages: self.logger.get_messages(),
                                total_errors: self.logger.total_errors as u32,
                            })
                        }
                        WorkerInput::ResetLog => {
                            self.logger.apply_command(pystral_core::ui_log::LogCommand::Reset);
                            Some(WorkerOutput::LogUpdate {
                                messages: self.logger.get_messages(),
                                total_errors: self.logger.total_errors as u32,
                            })
                        }
                        WorkerInput::RuntimeRequest(task) => {
                            let (res, logs) = self.runtime.process_request(task);
                            for log in logs {
                                self.logger.apply_command(pystral_core::ui_log::LogCommand::Log(log));
                            }
                            Some(WorkerOutput::RuntimeResponse(Box::new(res)))
                        }
                    };
                    
                    if let Some(msg) = response {
                        let out_envelope = Envelope {
                            seq: self.next_output_seq,
                            msg,
                        };
                        self.next_output_seq += 1;
                        self.outbox.push_back(ReliableOutput::Msg(Box::new(out_envelope)));
                    }
                    
                    let ack_seq = self.last_received_seq;
                    self.outbox.push_back(ReliableOutput::Watermark(ack_seq));
                }
                ReliableInput::Watermark(_seq) => {
                    // Handle ACK
                }
            }
        }
        
        if received_any {
            // Re-drain outbox if anything was added
            while self.outbox.front().is_some() {
                match self.scope.poll_ready_unpin(cx) {
                    Poll::Ready(Ok(())) => {
                        let msg = self.outbox.pop_front().expect("Outbox should not be empty");
                        let _ = self.scope.start_send_unpin(msg);
                    }
                    Poll::Ready(Err(_)) => {
                        self.outbox.pop_front();
                    }
                    Poll::Pending => break,
                }
            }
        }

        let _ = self.scope.poll_flush_unpin(cx);
        Poll::Pending
    }
}

#[wasm_bindgen]
pub fn init_worker() {
    gloo_worker::reactor::ReactorRegistrar::<UnifiedWorker>::new().register();
}

pub use gloo_worker::reactor::ReactorBridge;

impl UnifiedWorker {
    pub fn spawn() -> ReactorBridge<UnifiedWorker> {
        gloo_worker::reactor::ReactorSpawner::<UnifiedWorker>::new()
            .as_module(true)
            .with_loader(true)
            .spawn("/worker.js")
    }
}
