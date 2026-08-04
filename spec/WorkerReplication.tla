------------------------------ MODULE WorkerReplication ------------------------------
EXTENDS Naturals, FiniteSets

(***************************************************************************)
(* A bounded model of the gloo Reactor bridge used by worker.rs/main.rs.   *)
(* Main submits ordered requests. The worker is authoritative and emits   *)
(* one ordered response per request. Main applies responses in order and   *)
(* is therefore a replicated prefix of worker state. Watermarks acknowledge *)
(* main -> worker input; Heartbeat advertises worker output progress.       *)
(***************************************************************************)

CONSTANT MaxRequests
RequestIds == 1..MaxRequests

VARIABLES submitted, processed, applied, workerState, mainState,
          responseValue, heartbeat, lastHeartbeat, inputWatermark

vars == << submitted, processed, applied, workerState, mainState,
           responseValue, heartbeat, lastHeartbeat, inputWatermark >>

Init ==
    /\ submitted = 0
    /\ processed = 0
    /\ applied = 0
    /\ workerState = 0
    /\ mainState = 0
    /\ responseValue = [r \in RequestIds |-> 0]
    /\ heartbeat = 0
    /\ lastHeartbeat = 0
    /\ inputWatermark = 0

Submit ==
    /\ submitted < MaxRequests
    /\ submitted' = submitted + 1
    /\ UNCHANGED << processed, applied, workerState, mainState,
                    responseValue, heartbeat, lastHeartbeat, inputWatermark >>

Process ==
    /\ processed < submitted
    /\ processed' = processed + 1
    /\ workerState' = workerState + 1
    /\ responseValue' = [responseValue EXCEPT ![processed + 1] = workerState + 1]
    /\ UNCHANGED << submitted, applied, mainState,
                    heartbeat, lastHeartbeat, inputWatermark >>

(* This is distinct from the input watermark: it advertises worker output
   progress, including the no-output case. *)
Heartbeat ==
    /\ heartbeat' = processed
    /\ heartbeat' >= heartbeat
    /\ lastHeartbeat' = heartbeat'
    /\ UNCHANGED << submitted, processed, applied, workerState, mainState,
                    responseValue, inputWatermark >>

Deliver ==
    /\ applied < processed
    /\ applied' = applied + 1
    /\ mainState' = responseValue[applied + 1]
    /\ UNCHANGED << submitted, processed, workerState, responseValue,
                    heartbeat, lastHeartbeat, inputWatermark >>

InputWatermark ==
    /\ inputWatermark' = processed
    /\ inputWatermark' >= inputWatermark
    /\ UNCHANGED << submitted, processed, applied, workerState, mainState,
                    responseValue, heartbeat, lastHeartbeat >>

Stutter == UNCHANGED vars

Next == Submit \/ Process \/ Heartbeat \/ Deliver \/ InputWatermark \/ Stutter

TypeInvariant ==
    /\ submitted \in 0..MaxRequests
    /\ processed \in 0..MaxRequests
    /\ applied \in 0..MaxRequests
    /\ processed <= submitted
    /\ applied <= processed
    /\ workerState = processed
    /\ mainState = applied
    /\ responseValue \in [RequestIds -> 0..MaxRequests]
    /\ heartbeat \in 0..MaxRequests
    /\ lastHeartbeat \in 0..MaxRequests
    /\ inputWatermark \in 0..MaxRequests

(* Every observed operation has exactly the value returned by its unique
   ordered worker commit. The increment command makes the linearization
   points visible as the sequence 1, 2, ... . *)
Linearizable ==
    /\ mainState = applied
    /\ \A r \in 1..applied : responseValue[r] = r

NoFutureObservation == applied <= processed
HeartbeatIsSafe == lastHeartbeat = heartbeat /\ heartbeat <= processed
WatermarkIsSafe == inputWatermark <= processed

AllSubmittedEventuallyReplicated ==
    (submitted = MaxRequests) ~> (applied = MaxRequests)

HeartbeatEventuallyCurrent ==
    (processed = MaxRequests) ~> (heartbeat = MaxRequests)

(* Fairness excludes an indefinitely stalled worker or transport. Without
   these assumptions liveness is intentionally false for a disconnected UI.
   Heartbeat abstracts the periodic control probe/response scheduler: it must
   run even when gameplay is idle, while synchronous worker computation may
   still delay its response. *)
Spec == Init /\ [][Next]_vars
        /\ WF_vars(Process)
        /\ WF_vars(Deliver)
        /\ WF_vars(Heartbeat)

=============================================================================
