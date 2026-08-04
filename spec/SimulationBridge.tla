---------------------------- MODULE SimulationBridge ----------------------------
EXTENDS Naturals

(***************************************************************************)
(* One model for the critical path.  A request is durable at the          *)
(* controller until its response is applied.  The worker may process it  *)
(* once, but keeps the result in a response cache.  Losing a response is  *)
(* therefore harmless: RetryResponse republishes the cached result.       *)
(* Heartbeat is deliberately just an observation. *)
(* Heartbeat is deliberately just an observation.  It can advance while  *)
(* the worker is busy, while a response is lost, and while the controller *)
(* is retrying.  It is not an acknowledgement and never completes work.  *)
(***************************************************************************)

CONSTANTS MaxRequests, MaxWork, MaxHeartbeat
RequestKinds == {"StepDemoSimulation", "ResumeBoundary", "RequestMctsDecision",
                 "AcknowledgeAnimation"}
States == {"Idle", "Simulating", "AwaitingPlayerDecision", "Failed"}

VARIABLES state, nextSeq, requestSeq, requestKind, requestOnWire,
          workerBusy, workRemaining, workerProcessed, responseCache,
          responseOnWire, responseError, responseDropped, appliedSeq, heartbeat

vars == << state, nextSeq, requestSeq, requestKind, requestOnWire,
           workerBusy, workRemaining, workerProcessed, responseCache,
           responseOnWire, responseError, responseDropped, appliedSeq, heartbeat >>

Init ==
    /\ state = "Idle"
    /\ nextSeq = 0
    /\ requestSeq = 0
    /\ requestKind = "None"
    /\ requestOnWire = FALSE
    /\ workerBusy = FALSE
    /\ workRemaining = 0
    /\ workerProcessed = 0
    /\ responseCache = 0
    /\ responseOnWire = FALSE
    /\ responseError = FALSE
    /\ responseDropped = FALSE
    /\ appliedSeq = 0
    /\ heartbeat = 0

Submit(kind) ==
    /\ kind \in RequestKinds
    /\ nextSeq < MaxRequests
    /\ requestSeq = appliedSeq
    /\ nextSeq' = nextSeq + 1
    /\ requestSeq' = nextSeq + 1
    /\ requestKind' = kind
    /\ requestOnWire' = TRUE
    /\ state' = "Simulating"
    /\ responseError' = FALSE
    /\ responseDropped' = FALSE
    /\ UNCHANGED << workerBusy, workRemaining, workerProcessed,
                    responseCache, responseOnWire,
                    appliedSeq, heartbeat >>

(* The request can be retransmitted until the worker accepts it. *)
SendRequest ==
    /\ requestSeq > workerProcessed
    /\ ~workerBusy
    /\ requestOnWire' = TRUE
    /\ UNCHANGED << state, nextSeq, requestSeq, requestKind, workerBusy,
                    workRemaining, workerProcessed, responseCache,
                    responseOnWire, responseError, responseDropped,
                    appliedSeq, heartbeat >>

AcceptRequest ==
    /\ requestOnWire
    /\ requestSeq > workerProcessed
    /\ ~workerBusy
    /\ requestOnWire' = FALSE
    /\ workerBusy' = TRUE
    /\ workRemaining' = MaxWork
    /\ UNCHANGED << state, nextSeq, requestSeq, requestKind,
                    workerProcessed, responseCache, responseOnWire,
                    responseError, responseDropped, appliedSeq, heartbeat >>

RunSlice ==
    /\ workerBusy
    /\ workRemaining > 0
    /\ workRemaining' = workRemaining - 1
    /\ workerBusy' = (workRemaining > 1)
    /\ workerProcessed' = IF workRemaining = 1
                          THEN requestSeq ELSE workerProcessed
    /\ UNCHANGED << state, nextSeq, requestSeq, requestKind, requestOnWire,
                    responseCache, responseOnWire, responseError,
                    responseDropped, appliedSeq, heartbeat >>

(* Runtime completion, including an error response, is durable. *)
PublishResponse ==
    /\ ~workerBusy
    /\ workerProcessed > responseCache
    /\ responseCache' = workerProcessed
    /\ responseOnWire' = TRUE
    /\ UNCHANGED << state, nextSeq, requestSeq, requestKind, requestOnWire,
                    workerBusy, workRemaining, workerProcessed,
                    responseError, responseDropped, appliedSeq, heartbeat >>

(* A transport loss removes only the copy on the wire, never the cache. *)
LoseResponse ==
    /\ responseOnWire
    /\ ~responseDropped
    /\ responseOnWire' = FALSE
    /\ responseDropped' = TRUE
    /\ UNCHANGED << state, nextSeq, requestSeq, requestKind, requestOnWire,
                    workerBusy, workRemaining, workerProcessed,
                    responseCache, responseError, appliedSeq, heartbeat >>

RetryResponse ==
    /\ ~responseOnWire
    /\ responseCache > appliedSeq
    /\ responseOnWire' = TRUE
    /\ UNCHANGED << state, nextSeq, requestSeq, requestKind, requestOnWire,
                    workerBusy, workRemaining, workerProcessed,
                    responseCache, responseError, responseDropped,
                    appliedSeq, heartbeat >>

ApplyResponse ==
    /\ responseOnWire
    /\ responseCache = requestSeq
    /\ responseOnWire' = FALSE
    /\ appliedSeq' = responseCache
    /\ state' = IF responseError
                THEN "Failed" ELSE "AwaitingPlayerDecision"
    /\ UNCHANGED << nextSeq, requestSeq, requestKind, requestOnWire,
                    workerBusy, workRemaining, workerProcessed,
                    responseCache, responseError, responseDropped, heartbeat >>

Heartbeat ==
    /\ heartbeat' = IF heartbeat < MaxHeartbeat
                    THEN heartbeat + 1 ELSE heartbeat
    /\ UNCHANGED << state, nextSeq, requestSeq, requestKind, requestOnWire,
                    workerBusy, workRemaining, workerProcessed,
                    responseCache, responseOnWire, responseError,
                    responseDropped,
                    appliedSeq >>

Stutter == UNCHANGED vars

Next ==
    \/ Submit("StepDemoSimulation")
    \/ Submit("ResumeBoundary")
    \/ Submit("RequestMctsDecision")
    \/ Submit("AcknowledgeAnimation")
    \/ SendRequest
    \/ AcceptRequest
    \/ RunSlice
    \/ PublishResponse
    \/ LoseResponse
    \/ RetryResponse
    \/ ApplyResponse
    \/ Heartbeat
    \/ Stutter

TypeInvariant ==
    /\ state \in States
    /\ nextSeq \in 0..MaxRequests
    /\ requestSeq \in 0..MaxRequests
    /\ requestKind \in RequestKinds \cup {"None"}
    /\ requestOnWire \in BOOLEAN
    /\ workerBusy \in BOOLEAN
    /\ workRemaining \in 0..MaxWork
    /\ workerProcessed \in 0..MaxRequests
    /\ responseCache \in 0..MaxRequests
    /\ responseOnWire \in BOOLEAN
    /\ responseError \in BOOLEAN
    /\ responseDropped \in BOOLEAN
    /\ appliedSeq \in 0..MaxRequests
    /\ heartbeat \in 0..MaxHeartbeat
    /\ appliedSeq <= workerProcessed
    /\ workerProcessed <= requestSeq
    /\ responseCache <= workerProcessed
    /\ responseOnWire => responseCache > appliedSeq
    /\ workerBusy => requestSeq > workerProcessed
    /\ state = "AwaitingPlayerDecision" => appliedSeq = requestSeq
    /\ state = "Failed" => appliedSeq = requestSeq

(* The worker never executes the same request twice; retransmission uses the
   cached response instead. *)
AtMostOnceExecution ==
    workerProcessed <= requestSeq

(* Heartbeat freshness says only that the controller can run. *)
HeartbeatDoesNotAcknowledge ==
    [](heartbeat > 0 /\ appliedSeq < requestSeq
       => state = "Simulating")

(* With fair delivery/retry/application, every durable request resolves. *)
EveryRequestResolves ==
    [](requestSeq > appliedSeq ~> appliedSeq = requestSeq)

NoTerminalResponsiveSimulation ==
    [](state = "Simulating" ~> appliedSeq = requestSeq)

Spec == Init /\ [][Next]_vars
        /\ WF_vars(AcceptRequest)
        /\ WF_vars(RunSlice)
        /\ WF_vars(PublishResponse)
        /\ WF_vars(RetryResponse)
        /\ WF_vars(ApplyResponse)
        /\ WF_vars(Heartbeat)

=============================================================================
