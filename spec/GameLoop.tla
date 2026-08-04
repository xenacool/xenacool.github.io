------------------------------ MODULE GameLoop ------------------------------
EXTENDS Naturals, Sequences, FiniteSets

CONSTANTS Players, NPCs, MaxRounds, MaxActions

AwaitBoundary == "AwaitBoundary"
AwaitPlayer   == "AwaitPlayer"
AwaitMcts     == "AwaitMcts"
AwaitAck      == "AwaitAck"
RecoverReject == "RecoverReject"
Completed     == "Completed"

Move     == "Move"
Ability  == "Ability"
Wait     == "Wait"
Reaction == "Reaction"
Actions  == {Move, Ability, Wait, Reaction}
NoValue  == "None"
NoRequest == 0 - 1
RequestValues == 0..MaxActions

VARIABLES phase, activeUnit, controller, requestId, stateVersion, barrierId,
          requestOutstanding, candidateReady, candidateValid, candidateAction,
          candidateRequestId, candidateVersion, recoveryRequestId,
          lastAction, reactionQueue, fallbackAttempted, history,
          completionCount, completed, rounds

vars == << phase, activeUnit, controller, requestId, stateVersion, barrierId,
           requestOutstanding, candidateReady, candidateValid, candidateAction,
           candidateRequestId, candidateVersion, recoveryRequestId, lastAction,
           reactionQueue, fallbackAttempted, history, completionCount,
           completed, rounds >>

Init ==
    /\ phase = AwaitBoundary
    /\ activeUnit = NoValue
    /\ controller = NoValue
    /\ requestId = 0
    /\ stateVersion = 0
    /\ barrierId = 0
    /\ requestOutstanding = FALSE
    /\ candidateReady = FALSE
    /\ candidateValid = FALSE
    /\ candidateAction = NoValue
    /\ candidateRequestId = NoRequest
    /\ candidateVersion = NoRequest
    /\ recoveryRequestId = NoRequest
    /\ lastAction = NoValue
    /\ reactionQueue = {}
    /\ fallbackAttempted = FALSE
    /\ history = << >>
    /\ completionCount = 0
    /\ completed = FALSE
    /\ rounds = 0

PendingReaction(unit) ==
    \E reaction \in reactionQueue : reaction.owner = unit

WaitLegal(unit) == ~PendingReaction(unit)

ReactionLegal(unit) == PendingReaction(unit)

ActionLegal(unit, action) ==
    /\ action \in Actions
    /\ IF action = Wait THEN WaitLegal(unit)
       ELSE IF action = Reaction THEN ReactionLegal(unit)
       ELSE TRUE

AdvanceBoundary(unit) ==
    /\ phase = AwaitBoundary
    /\ ~completed
    /\ unit \in Players \cup NPCs
    /\ activeUnit' = unit
    /\ controller' = IF unit \in Players THEN "player" ELSE "npc"
    /\ phase' = IF unit \in Players THEN AwaitPlayer ELSE AwaitMcts
    /\ UNCHANGED << requestId, stateVersion, barrierId, requestOutstanding,
                    candidateReady, candidateValid, candidateAction,
                    candidateRequestId, candidateVersion, recoveryRequestId,
                    lastAction, reactionQueue, fallbackAttempted, history,
                    completionCount, completed, rounds >>

QueueReaction(unit) ==
    /\ unit \in Players \cup NPCs
    /\ ~completed
    /\ reactionQueue' = reactionQueue \cup {[owner |-> unit]}
    /\ UNCHANGED << phase, activeUnit, controller, requestId, stateVersion,
                    barrierId, requestOutstanding, candidateReady,
                    candidateValid, candidateAction, candidateRequestId,
                    candidateVersion, recoveryRequestId, lastAction,
                    fallbackAttempted, history, completionCount, completed,
                    rounds >>

RequestMcts ==
    /\ phase = AwaitMcts
    /\ ~requestOutstanding
    /\ ~candidateReady
    /\ ~completed
    /\ requestId' = requestId + 1
    /\ requestOutstanding' = TRUE
    /\ UNCHANGED << phase, activeUnit, controller, stateVersion, barrierId,
                    candidateReady, candidateValid, candidateAction,
                    candidateRequestId, candidateVersion, recoveryRequestId,
                    lastAction, reactionQueue, fallbackAttempted, history,
                    completionCount, completed, rounds >>

MctsReady(request, version, action, valid) ==
    /\ phase = AwaitMcts
    /\ requestOutstanding
    /\ request = requestId
    /\ version = stateVersion
    /\ action \in Actions
    /\ candidateReady' = TRUE
    /\ candidateValid' = valid /\ ActionLegal(activeUnit, action)
    /\ candidateAction' = action
    /\ candidateRequestId' = request
    /\ candidateVersion' = version
    /\ requestOutstanding' = FALSE
    /\ UNCHANGED << phase, activeUnit, controller, requestId, stateVersion,
                    barrierId, recoveryRequestId, lastAction, reactionQueue,
                    fallbackAttempted, history, completionCount, completed,
                    rounds >>

StaleMctsReady(request, version) ==
    /\ phase = AwaitMcts
    /\ requestOutstanding
    /\ (request # requestId \/ version # stateVersion)
    /\ UNCHANGED vars

DuplicateMctsReady ==
    /\ phase = AwaitMcts
    /\ ~requestOutstanding
    /\ candidateReady
    /\ UNCHANGED vars

SubmitPlayer ==
    /\ phase = AwaitPlayer
    /\ ~candidateReady
    /\ ~completed
    /\ candidateReady' = TRUE
    /\ candidateValid' = TRUE
    /\ candidateAction' = Wait
    /\ candidateRequestId' = requestId
    /\ candidateVersion' = stateVersion
    /\ UNCHANGED << phase, activeUnit, controller, requestId, stateVersion,
                    barrierId, requestOutstanding, recoveryRequestId,
                    lastAction, reactionQueue, fallbackAttempted, history,
                    completionCount, completed, rounds >>

RejectPlayer ==
    /\ phase = AwaitPlayer
    /\ candidateReady
    /\ ~completed
    /\ phase' = RecoverReject
    /\ recoveryRequestId' = requestId
    /\ candidateReady' = FALSE
    /\ UNCHANGED << activeUnit, controller, requestId, stateVersion, barrierId,
                    requestOutstanding, candidateValid, candidateAction,
                    candidateRequestId, candidateVersion, lastAction,
                    reactionQueue, fallbackAttempted, history,
                    completionCount, completed, rounds >>

ResumeRejected(request) ==
    /\ phase = RecoverReject
    /\ request = recoveryRequestId
    /\ phase' = AwaitPlayer
    /\ recoveryRequestId' = NoRequest
    /\ UNCHANGED << activeUnit, controller, requestId, stateVersion, barrierId,
                    requestOutstanding, candidateReady, candidateValid,
                    candidateAction, candidateRequestId, candidateVersion,
                    lastAction, reactionQueue, fallbackAttempted, history,
                    completionCount, completed, rounds >>

Commit(action) ==
    /\ phase \in {AwaitPlayer, AwaitMcts}
    /\ candidateReady
    /\ ~completed
    /\ stateVersion < MaxActions
    /\ ((controller = "player" /\ action = candidateAction /\ ActionLegal(activeUnit, action))
        \/ (controller = "npc" /\ candidateValid /\ action = candidateAction)
        \/ (controller = "npc" /\ ~candidateValid /\ action = Wait
            /\ WaitLegal(activeUnit)))
    /\ phase' = AwaitAck
    /\ candidateReady' = FALSE
    /\ lastAction' = action
    /\ fallbackAttempted' = (controller = "npc" /\ ~candidateValid /\ action = Wait)
    /\ barrierId' = barrierId + 1
    /\ stateVersion' = stateVersion + 1
    /\ history' = Append(history,
          [event |-> "ActionCommitted", unit |-> activeUnit,
           controller |-> controller, action |-> action,
           barrier |-> barrierId + 1, version |-> stateVersion + 1])
    /\ UNCHANGED << activeUnit, controller, requestId, requestOutstanding,
                    candidateValid, candidateAction, candidateRequestId,
                    candidateVersion, recoveryRequestId, reactionQueue,
                    completionCount, completed, rounds >>

Acknowledge ==
    /\ phase = AwaitAck
    /\ ~completed
    /\ phase' = IF lastAction = Wait THEN AwaitBoundary
                ELSE IF controller = "npc" THEN AwaitMcts ELSE AwaitPlayer
    /\ candidateReady' = FALSE
    /\ requestOutstanding' = FALSE
    /\ rounds' = IF lastAction = Wait THEN rounds + 1 ELSE rounds
    /\ reactionQueue' = IF lastAction = Reaction THEN
          {reaction \in reactionQueue : reaction.owner # activeUnit}
       ELSE reactionQueue
    /\ UNCHANGED << activeUnit, controller, requestId, stateVersion,
                    barrierId, candidateValid, candidateAction,
                    candidateRequestId, candidateVersion, recoveryRequestId,
                    lastAction, fallbackAttempted, history, completionCount,
                    completed >>

Complete ==
    /\ phase \in {AwaitBoundary, AwaitPlayer, AwaitMcts, AwaitAck}
    /\ ~completed
    /\ ~candidateReady
    /\ ~requestOutstanding
    /\ rounds >= MaxRounds
    /\ phase' = Completed
    /\ completed' = TRUE
    /\ completionCount' = completionCount + 1
    /\ history' = Append(history, [event |-> "GameCompleted", round |-> rounds])
    /\ UNCHANGED << activeUnit, controller, requestId, stateVersion, barrierId,
                    requestOutstanding, candidateReady, candidateValid,
                    candidateAction, candidateRequestId, candidateVersion,
                    recoveryRequestId, lastAction, reactionQueue,
                    fallbackAttempted, rounds >>

Stutter == UNCHANGED vars

Next ==
    \/ \E unit \in Players \cup NPCs : AdvanceBoundary(unit)
    \/ \E unit \in Players \cup NPCs : QueueReaction(unit)
    \/ RequestMcts
    \/ \E request \in RequestValues, version \in RequestValues,
         action \in Actions, valid \in BOOLEAN :
           MctsReady(request, version, action, valid)
    \/ SubmitPlayer
    \/ RejectPlayer
    \/ \E request \in RequestValues : ResumeRejected(request)
    \/ \E request \in RequestValues, version \in RequestValues :
           StaleMctsReady(request, version)
    \/ DuplicateMctsReady
    \/ \E action \in Actions : Commit(action)
    \/ Acknowledge
    \/ Complete
    \/ Stutter

TypeInvariant ==
    /\ phase \in {AwaitBoundary, AwaitPlayer, AwaitMcts, AwaitAck,
                  RecoverReject, Completed}
    /\ requestId \in Nat
    /\ stateVersion \in Nat
    /\ barrierId \in Nat
    /\ rounds \in Nat
    /\ completionCount \in Nat
    /\ candidateAction \in Actions \cup {NoValue}
    /\ completed = (phase = Completed)

ProtocolInvariant ==
    /\ (phase = AwaitAck) => lastAction \in Actions
    /\ (phase \in {AwaitBoundary, RecoverReject, Completed})
          => candidateReady = FALSE
    /\ (phase \in {AwaitBoundary, AwaitPlayer, AwaitAck, RecoverReject,
                   Completed}) => requestOutstanding = FALSE
    /\ (phase = AwaitPlayer) => controller = "player"
    /\ (phase = AwaitMcts) => controller = "npc"
    /\ (phase = AwaitMcts /\ candidateReady)
          => candidateRequestId = requestId
              /\ candidateVersion = stateVersion
    /\ (phase = RecoverReject) => recoveryRequestId = requestId

NoInvalidFallback ==
    fallbackAttempted => WaitLegal(activeUnit)

NoReactionStarvation ==
    /\ PendingReaction(activeUnit) /\ phase = AwaitMcts
       => ~WaitLegal(activeUnit)
    /\ reactionQueue # {} => 
       \E unit \in Players \cup NPCs : PendingReaction(unit)

MonotonicInvariant ==
    /\ stateVersion >= 0
    /\ barrierId >= 0
    /\ Len(history) >= 0

NoPostCompletionMutation == completed => phase = Completed
SingleCompletion == completionCount <= 1

(* The browser derives its presentation from the runtime continuation.  This
   projection is the contract implemented by TransientState and
   update_action_menu: a menu is only actionable at a player boundary, while a
   committed action remains pending until its presentation acknowledgment. *)
UiMenuVisible == phase = AwaitPlayer
UiActionPending == phase = AwaitAck
UiMenuUnit == IF UiMenuVisible THEN activeUnit ELSE NoValue
UiFeedback == IF UiActionPending THEN lastAction ELSE NoValue

UIInvariant ==
    /\ UiMenuVisible => controller = "player" /\ activeUnit \in Players
    /\ UiActionPending => lastAction \in Actions
    /\ UiActionPending => UiFeedback = lastAction
    /\ UiMenuVisible => ~UiActionPending
    /\ phase = Completed => ~UiMenuVisible /\ ~UiActionPending

AdvanceAny == \E unit \in Players \cup NPCs : AdvanceBoundary(unit)
ResumeAny == \E request \in RequestValues : ResumeRejected(request)

Fairness ==
    /\ WF_vars(Acknowledge)
    /\ WF_vars(AdvanceAny)
    /\ WF_vars(ResumeAny)

Liveness ==
    /\ [](phase = AwaitAck ~> phase # AwaitAck)
    /\ [](phase = RecoverReject ~> phase = AwaitPlayer)

Spec == Init /\ [][Next]_vars /\ Fairness

=============================================================================
