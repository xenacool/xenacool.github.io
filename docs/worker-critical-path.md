# Simulation acknowledgement critical path

The startup and animation-ack paths are deliberately split into observable
segments. A reduced replay only needs the ordered `actionInputs` in a replay
fixture; assets and code remain the canonical inputs.

```mermaid
sequenceDiagram
    participant UI
    participant U as UnifiedWorker
    participant UB as Unified↔Simulation bridge
    participant S as SimulationWorker
    participant R as Runtime

    UI->>U: GeneratePgRpgLog / ActionNav
    U->>U: assign simulation request seq
    U->>UB: SimulationRequest(seq)
    Note over UB: trace: simulation bridge send request seq N
    UB->>S: Request(seq)
    S->>R: process_request(request)
    R-->>S: response + continuation + snapshot
    S->>S: enqueue response, watermark, heartbeat
    Note over UB: trace: simulation worker heartbeat
    S-->>UB: Response(request_seq=N)
    Note over UB: trace: simulation bridge received response
    UB-->>U: SimulationResponse
    U->>U: match pending seq; update continuation/transient
    U-->>UI: RuntimeResponse / TransientState / heartbeat
    UI->>U: animation ACK watermark
    U->>S: AcknowledgeAnimation(seq)
```

Interpretation of a captured trace:

- No `send request`: the unified worker enqueue/scheduler path failed.
- `send request`, no simulation heartbeat: bridge delivery or simulation
  worker startup failed.
- Simulation heartbeat, no received response: runtime processing or simulation
  worker output flush failed.
- Received response, no action menu: unified response matching or transient
  state application failed.
- Action menu followed by `WaitingForAnimationAck`: renderer ACK/barrier
  delivery is the remaining segment.

The trace strings are emitted through the existing debug-trace export path and
are intentionally independent of gameplay history. This keeps replay fixtures
compact while retaining enough evidence to localize a failure.
