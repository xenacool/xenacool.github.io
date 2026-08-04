------------------------------ MODULE AnimationAck ------------------------------
EXTENDS Naturals

(***************************************************************************)
(* Critical-path model for the renderer/worker animation barrier.          *)
(* Runtime work is deliberately split into bounded non-preemptible slices:*)
(* probes can queue while a synchronous call is busy, but cannot be served *)
(* until that call yields. This models the Simulating heartbeat failure.   *)
(***************************************************************************)

CONSTANTS MaxBarrier, MaxControlBacklog, MaxSteps, MaxSimulationWork
Max(a, b) == IF a > b THEN a ELSE b

Ready       == "Ready"
Publishing  == "Publishing"
AwaitAck    == "AwaitAck"
Simulating  == "Simulating"

VARIABLES publishedBarrier, workerBarrier, visibleBarrier, ackWatermark,
          phase, ackPending, ackRetryDue, controlBacklog, probePending,
          heartbeatResponses, servicedInputs, simulationSteps,
          runtimeBusy, simulationWorkRemaining

vars == << publishedBarrier, workerBarrier, visibleBarrier, ackWatermark,
           phase, ackPending, ackRetryDue, controlBacklog, probePending,
           heartbeatResponses, servicedInputs, simulationSteps,
           runtimeBusy, simulationWorkRemaining >>

Init ==
    /\ publishedBarrier = 0
    /\ workerBarrier = 0
    /\ visibleBarrier = 0
    /\ ackWatermark = 0
    /\ phase = Ready
    /\ ackPending = FALSE
    /\ ackRetryDue = FALSE
    /\ controlBacklog = 0
    /\ probePending = FALSE
    /\ heartbeatResponses = 0
    /\ servicedInputs = 0
    /\ simulationSteps = 0
    /\ runtimeBusy = FALSE
    /\ simulationWorkRemaining = 0

CommitAction ==
    /\ phase = Ready
    /\ publishedBarrier < MaxBarrier
    /\ publishedBarrier' = publishedBarrier + 1
    /\ phase' = Publishing
    /\ UNCHANGED << workerBarrier, visibleBarrier, ackWatermark,
                    ackPending, ackRetryDue, controlBacklog, probePending,
                    heartbeatResponses, servicedInputs, simulationSteps,
                    runtimeBusy, simulationWorkRemaining >>

InstallPendingBarrier ==
    /\ phase = Publishing
    /\ workerBarrier' = publishedBarrier
    /\ phase' = IF ackWatermark >= publishedBarrier THEN Ready ELSE AwaitAck
    /\ UNCHANGED << publishedBarrier, visibleBarrier, ackWatermark,
                    ackPending, ackRetryDue, controlBacklog, probePending,
                    heartbeatResponses, servicedInputs, simulationSteps,
                    runtimeBusy, simulationWorkRemaining >>

RenderVisibleBarrier ==
    /\ visibleBarrier < publishedBarrier
    /\ visibleBarrier' = publishedBarrier
    /\ UNCHANGED << publishedBarrier, workerBarrier, ackWatermark, phase,
                    ackPending, ackRetryDue, controlBacklog, probePending,
                    heartbeatResponses, servicedInputs, simulationSteps,
                    runtimeBusy, simulationWorkRemaining >>

SendBarrierAck ==
    /\ visibleBarrier > 0
    /\ visibleBarrier > ackWatermark \/ ackRetryDue
    /\ ackPending' = TRUE
    /\ ackRetryDue' = FALSE
    /\ UNCHANGED << publishedBarrier, workerBarrier, visibleBarrier,
                    ackWatermark, phase, controlBacklog, probePending,
                    heartbeatResponses, servicedInputs, simulationSteps,
                    runtimeBusy, simulationWorkRemaining >>

ReceiveBarrierAck ==
    /\ ackPending
    /\ ackWatermark' = Max(ackWatermark, visibleBarrier)
    /\ ackPending' = FALSE
    /\ phase' = IF phase = AwaitAck /\ workerBarrier <= ackWatermark'
                  THEN Ready ELSE phase
    /\ UNCHANGED << publishedBarrier, workerBarrier, visibleBarrier,
                    ackRetryDue, controlBacklog, probePending,
                    heartbeatResponses, servicedInputs, simulationSteps,
                    runtimeBusy, simulationWorkRemaining >>

RetryTimer ==
    /\ phase = AwaitAck
    /\ ackRetryDue' = TRUE
    /\ UNCHANGED << publishedBarrier, workerBarrier, visibleBarrier,
                    ackWatermark, phase, ackPending, controlBacklog,
                    probePending, heartbeatResponses, servicedInputs,
                    simulationSteps, runtimeBusy, simulationWorkRemaining >>

QueueControlInput ==
    /\ controlBacklog < MaxControlBacklog
    /\ controlBacklog' = controlBacklog + 1
    /\ UNCHANGED << publishedBarrier, workerBarrier, visibleBarrier,
                    ackWatermark, phase, ackPending, ackRetryDue,
                    probePending, heartbeatResponses, servicedInputs,
                    simulationSteps, runtimeBusy, simulationWorkRemaining >>

QueueProbe ==
    /\ probePending' = TRUE
    /\ UNCHANGED << publishedBarrier, workerBarrier, visibleBarrier,
                    ackWatermark, phase, ackPending, ackRetryDue,
                    controlBacklog, heartbeatResponses, servicedInputs,
                    simulationSteps, runtimeBusy, simulationWorkRemaining >>

ServiceProbe ==
    /\ probePending
    /\ ~runtimeBusy
    /\ probePending' = FALSE
    /\ heartbeatResponses' = IF heartbeatResponses < MaxSteps
                              THEN heartbeatResponses + 1 ELSE heartbeatResponses
    /\ servicedInputs' = IF servicedInputs < MaxSteps
                          THEN servicedInputs + 1 ELSE servicedInputs
    /\ UNCHANGED << publishedBarrier, workerBarrier, visibleBarrier,
                    ackWatermark, phase, ackPending, ackRetryDue,
                    controlBacklog, simulationSteps, runtimeBusy,
                    simulationWorkRemaining >>

ServiceControl ==
    /\ controlBacklog > 0
    /\ ~runtimeBusy
    /\ controlBacklog' = controlBacklog - 1
    /\ servicedInputs' = IF servicedInputs < MaxSteps
                          THEN servicedInputs + 1 ELSE servicedInputs
    /\ UNCHANGED << publishedBarrier, workerBarrier, visibleBarrier,
                    ackWatermark, phase, ackPending, ackRetryDue,
                    probePending, heartbeatResponses, simulationSteps,
                    runtimeBusy, simulationWorkRemaining >>

BeginSimulation ==
    /\ phase = Ready
    /\ ~probePending
    /\ phase' = Simulating
    /\ runtimeBusy' = TRUE
    /\ simulationWorkRemaining' = MaxSimulationWork
    /\ UNCHANGED << publishedBarrier, workerBarrier, visibleBarrier,
                    ackWatermark, ackPending, ackRetryDue, controlBacklog,
                    probePending, heartbeatResponses, servicedInputs,
                    simulationSteps >>

RunSimulationSlice ==
    /\ phase = Simulating
    /\ runtimeBusy
    /\ simulationWorkRemaining > 0
    /\ simulationWorkRemaining' = simulationWorkRemaining - 1
    /\ runtimeBusy' = (simulationWorkRemaining > 1)
    /\ phase' = IF simulationWorkRemaining = 1 THEN Ready ELSE Simulating
    /\ simulationSteps' = IF simulationWorkRemaining = 1
                          THEN IF simulationSteps < MaxSteps
                               THEN simulationSteps + 1 ELSE simulationSteps
                          ELSE simulationSteps
    /\ UNCHANGED << publishedBarrier, workerBarrier, visibleBarrier,
                    ackWatermark, ackPending, ackRetryDue, controlBacklog,
                    probePending, heartbeatResponses, servicedInputs >>

Stutter == UNCHANGED vars

(* Legacy action union retained for reference; Next2 below is the executable union.
Next ==
    \/ CommitAction \/ InstallPendingBarrier \/ RenderVisibleBarrier
    \/ SendBarrierAck / ReceiveBarrierAck / RetryTimer
    \/ QueueControlInput \/ QueueProbe / ServiceProbe / ServiceControl
    \/ BeginSimulation \/ RunSimulationSlice \/ Stutter

*)
Next2 ==
    \/ CommitAction \/ InstallPendingBarrier \/ RenderVisibleBarrier
    \/ SendBarrierAck \/ ReceiveBarrierAck \/ RetryTimer
    \/ QueueControlInput \/ QueueProbe \/ ServiceProbe \/ ServiceControl
    \/ BeginSimulation \/ RunSimulationSlice \/ Stutter

TypeInvariant ==
    /\ publishedBarrier \in 0..MaxBarrier
    /\ workerBarrier \in 0..MaxBarrier
    /\ visibleBarrier \in 0..MaxBarrier
    /\ ackWatermark \in 0..MaxBarrier
    /\ workerBarrier <= publishedBarrier
    /\ visibleBarrier <= publishedBarrier
    /\ ackWatermark <= visibleBarrier
    /\ phase \in {Ready, Publishing, AwaitAck, Simulating}
    /\ controlBacklog \in 0..MaxControlBacklog
    /\ heartbeatResponses \in 0..MaxSteps
    /\ servicedInputs \in 0..MaxSteps
    /\ simulationSteps \in 0..MaxSteps
    /\ runtimeBusy \in BOOLEAN
    /\ simulationWorkRemaining \in 0..MaxSimulationWork
    /\ runtimeBusy = TRUE => phase = Simulating

AckOwnership == phase = AwaitAck => ackWatermark < workerBarrier
AckMonotonic == ackWatermark <= visibleBarrier
ProbeServiceIsSafe == heartbeatResponses <= servicedInputs

CriticalPathLiveness ==
    /\ []((phase = AwaitAck) ~> (phase # AwaitAck))
    /\ [](probePending ~> ~probePending)
    /\ [](runtimeBusy ~> ~runtimeBusy)

Spec == Init /\ [][Next2]_vars
        /\ WF_vars(InstallPendingBarrier)
        /\ WF_vars(RenderVisibleBarrier)
        /\ WF_vars(SendBarrierAck)
        /\ WF_vars(ReceiveBarrierAck)
        /\ WF_vars(ServiceProbe)
        /\ WF_vars(RunSimulationSlice)

=============================================================================
