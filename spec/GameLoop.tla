------------------------------ MODULE GameLoop ------------------------------
EXTENDS Naturals, Sequences, FiniteSets

CONSTANTS Players, NPCs, MaxRounds, MaxActions

AwaitBoundary == "AwaitBoundary"
AwaitPlayer   == "AwaitPlayer"
AwaitMcts     == "AwaitMcts"
AwaitPlayerReaction == "AwaitPlayerReaction"
AwaitNpcReaction == "AwaitNpcReaction"
AwaitAck      == "AwaitAck"
RecoverReject == "RecoverReject"
Completed     == "Completed"

Move     == "Move"
Ability  == "Ability"
Wait     == "Wait"
Reaction == "Reaction"
OrdinaryActions == {Move, Ability, Wait}
Actions  == OrdinaryActions \cup {Reaction}
NoValue  == "None"
NoRequest == 0 - 1
RequestValues == 0..MaxActions

VARIABLES phase, activeUnit, controller, requestId, stateVersion, barrierId,
          requestOutstanding, candidateReady, candidateValid, candidateAction,
          candidateRequestId, candidateVersion, recoveryRequestId,
          lastAction, reactionQueue, fallbackAttempted, history,
          completionCount, completed, rounds, reactionSerial, activeReaction,
          resumeUnit, resumeController, turnLedger, reactionLedger

vars == << phase, activeUnit, controller, requestId, stateVersion, barrierId,
           requestOutstanding, candidateReady, candidateValid, candidateAction,
           candidateRequestId, candidateVersion, recoveryRequestId, lastAction,
           reactionQueue, fallbackAttempted, history, completionCount,
           completed, rounds, reactionSerial, activeReaction, resumeUnit,
           resumeController, turnLedger, reactionLedger >>

ReactionRecord(owner, target, instance) ==
    [owner |-> owner, target |-> target, instance |-> instance]

NoReaction == ReactionRecord(NoValue, NoValue, 0)

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
    /\ reactionSerial = 0
    /\ activeReaction = NoReaction
    /\ resumeUnit = NoValue
    /\ resumeController = NoValue
    /\ turnLedger = {}
    /\ reactionLedger = {}

PendingReaction(unit) ==
    \E reaction \in reactionQueue : reaction.owner = unit

WaitLegal(unit) == ~PendingReaction(unit)

ActionLegal(unit, action) ==
    /\ action \in OrdinaryActions
    /\ IF action = Wait THEN WaitLegal(unit)
       ELSE TRUE

AdvanceBoundary(unit) ==
    /\ phase = AwaitBoundary
    /\ ~completed
    /\ reactionQueue = {}
    /\ unit \in Players \cup NPCs
    /\ activeUnit' = unit
    /\ controller' = IF unit \in Players THEN "player" ELSE "npc"
    /\ phase' = IF unit \in Players THEN AwaitPlayer ELSE AwaitMcts
    /\ UNCHANGED << requestId, stateVersion, barrierId, requestOutstanding,
                    candidateReady, candidateValid, candidateAction,
                    candidateRequestId, candidateVersion, recoveryRequestId,
                    lastAction, reactionQueue, fallbackAttempted, history,
                    completionCount, completed, rounds, reactionSerial,
                    activeReaction, resumeUnit, resumeController, turnLedger,
                    reactionLedger >>

QueueReaction(unit, target) ==
    /\ unit \in Players \cup NPCs
    /\ target \in Players \cup NPCs
    /\ phase = AwaitAck
    /\ ~completed
    /\ reactionSerial < MaxActions
    /\ reactionSerial' = reactionSerial + 1
    /\ reactionQueue' = reactionQueue \cup
          {ReactionRecord(unit, target, reactionSerial + 1)}
    /\ UNCHANGED << phase, activeUnit, controller, requestId, stateVersion,
                    barrierId, requestOutstanding, candidateReady,
                    candidateValid, candidateAction, candidateRequestId,
                    candidateVersion, recoveryRequestId, lastAction,
                    fallbackAttempted, history, completionCount, completed, rounds,
                    activeReaction, resumeUnit, resumeController, turnLedger,
                    reactionLedger >>

BeginReaction(reaction) ==
    /\ phase = AwaitBoundary
    /\ reaction \in reactionQueue
    /\ activeReaction = NoReaction
    /\ activeReaction' = reaction
    /\ resumeUnit' = IF resumeUnit = NoValue THEN activeUnit ELSE resumeUnit
    /\ resumeController' = IF resumeController = NoValue THEN controller ELSE resumeController
    /\ activeUnit' = reaction.owner
    /\ controller' = IF reaction.owner \in Players THEN "player" ELSE "npc"
    /\ phase' = IF reaction.owner \in Players THEN AwaitPlayerReaction
                 ELSE AwaitNpcReaction
    /\ UNCHANGED << requestId, stateVersion, barrierId, requestOutstanding,
                    candidateReady, candidateValid, candidateAction,
                    candidateRequestId, candidateVersion, recoveryRequestId,
                    lastAction, reactionQueue, fallbackAttempted, history,
                    completionCount, completed, rounds, reactionSerial,
                    turnLedger, reactionLedger >>

ResolveReaction ==
    /\ phase \in {AwaitPlayerReaction, AwaitNpcReaction}
    /\ activeReaction \in reactionQueue
    /\ activeReaction.owner = activeUnit
    /\ phase' = AwaitAck
    /\ lastAction' = Reaction
    /\ barrierId' = barrierId + 1
    /\ stateVersion' = stateVersion + 1
    /\ reactionLedger' = turnLedger
    /\ history' = Append(history,
          [event |-> "ReactionCommitted", unit |-> activeUnit,
           trigger |-> activeReaction.instance, barrier |-> barrierId + 1,
           version |-> stateVersion + 1])
    /\ UNCHANGED << activeUnit, controller, requestId, requestOutstanding,
                    candidateReady, candidateValid, candidateAction,
                    candidateRequestId, candidateVersion, recoveryRequestId,
                    reactionQueue, fallbackAttempted, completionCount, completed,
                    rounds, reactionSerial, activeReaction, resumeUnit,
                    resumeController, turnLedger >>

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
                    completionCount, completed, rounds, reactionSerial,
                    activeReaction, resumeUnit, resumeController, turnLedger,
                    reactionLedger >>

MctsReady(request, version, action, valid) ==
    /\ phase = AwaitMcts
    /\ requestOutstanding
    /\ request = requestId
    /\ version = stateVersion
    /\ action \in OrdinaryActions
    /\ candidateReady' = TRUE
    /\ candidateValid' = valid /\ ActionLegal(activeUnit, action)
    /\ candidateAction' = action
    /\ candidateRequestId' = request
    /\ candidateVersion' = version
    /\ requestOutstanding' = FALSE
    /\ UNCHANGED << phase, activeUnit, controller, requestId, stateVersion,
                    barrierId, recoveryRequestId, lastAction, reactionQueue,
                    fallbackAttempted, history, completionCount, completed,
                    rounds, reactionSerial, activeReaction, resumeUnit,
                    resumeController, turnLedger, reactionLedger >>

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
                    completionCount, completed, rounds, reactionSerial,
                    activeReaction, resumeUnit, resumeController, turnLedger,
                    reactionLedger >>

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
                    completionCount, completed, rounds, reactionSerial,
                    activeReaction, resumeUnit, resumeController, turnLedger,
                    reactionLedger >>

ResumeRejected(request) ==
    /\ phase = RecoverReject
    /\ request = recoveryRequestId
    /\ phase' = AwaitPlayer
    /\ recoveryRequestId' = NoRequest
    /\ UNCHANGED << activeUnit, controller, requestId, stateVersion, barrierId,
                    requestOutstanding, candidateReady, candidateValid,
                    candidateAction, candidateRequestId, candidateVersion,
                    lastAction, reactionQueue, fallbackAttempted, history,
                    completionCount, completed, rounds, reactionSerial,
                    activeReaction, resumeUnit, resumeController, turnLedger,
                    reactionLedger >>

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
    /\ turnLedger' = turnLedger \cup {activeUnit}
    /\ UNCHANGED << activeUnit, controller, requestId, requestOutstanding,
                    candidateValid, candidateAction, candidateRequestId,
                    candidateVersion, recoveryRequestId, reactionQueue,
                    completionCount, completed, rounds, reactionSerial,
                    activeReaction, resumeUnit, resumeController, reactionLedger >>

Acknowledge ==
    /\ phase = AwaitAck
    /\ ~completed
    /\ phase' = IF lastAction = Reaction THEN
                    IF reactionQueue \ {activeReaction} # {} THEN AwaitBoundary
                    ELSE IF resumeController = "npc" THEN AwaitMcts ELSE AwaitPlayer
                 ELSE IF reactionQueue # {} THEN AwaitBoundary
                 ELSE IF lastAction = Wait THEN AwaitBoundary
                 ELSE IF controller = "npc" THEN AwaitMcts ELSE AwaitPlayer
    /\ activeUnit' = IF lastAction = Reaction /\ reactionQueue \ {activeReaction} = {}
                     THEN resumeUnit ELSE activeUnit
    /\ controller' = IF lastAction = Reaction /\ reactionQueue \ {activeReaction} = {}
                   THEN resumeController ELSE controller
    /\ candidateReady' = FALSE
    /\ requestOutstanding' = FALSE
    /\ rounds' = IF lastAction = Wait THEN rounds + 1 ELSE rounds
    /\ reactionQueue' = IF lastAction = Reaction THEN
          reactionQueue \ {activeReaction}
       ELSE reactionQueue
    /\ activeReaction' = IF lastAction = Reaction THEN NoReaction ELSE activeReaction
    /\ resumeUnit' = IF lastAction = Reaction /\ reactionQueue \ {activeReaction} = {}
                  THEN NoValue ELSE resumeUnit
    /\ resumeController' = IF lastAction = Reaction /\ reactionQueue \ {activeReaction} = {}
                        THEN NoValue ELSE resumeController
    /\ UNCHANGED << requestId, stateVersion,
                    barrierId, candidateValid, candidateAction,
                    candidateRequestId, candidateVersion, recoveryRequestId,
                    lastAction, fallbackAttempted, history, completionCount,
                    completed, reactionSerial, turnLedger, reactionLedger >>

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
                    fallbackAttempted, rounds, reactionSerial, activeReaction,
                    resumeUnit, resumeController, turnLedger, reactionLedger >>

Stutter == UNCHANGED vars

Next ==
    \/ \E unit \in Players \cup NPCs : AdvanceBoundary(unit)
    \/ \E unit \in Players \cup NPCs, target \in Players \cup NPCs :
           QueueReaction(unit, target)
    \/ \E reaction \in reactionQueue : BeginReaction(reaction)
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
    \/ \E action \in OrdinaryActions : Commit(action)
    \/ ResolveReaction
    \/ Acknowledge
    \/ Complete
    \/ Stutter

TypeInvariant ==
    /\ phase \in {AwaitBoundary, AwaitPlayer, AwaitMcts, AwaitPlayerReaction,
                  AwaitNpcReaction, AwaitAck, RecoverReject, Completed}
    /\ requestId \in Nat
    /\ stateVersion \in Nat
    /\ barrierId \in Nat
    /\ rounds \in Nat
    /\ completionCount \in Nat
    /\ reactionSerial \in Nat
    /\ reactionQueue \subseteq
          {[owner |-> owner, target |-> target, instance |-> instance] :
             owner \in Players \cup NPCs, target \in Players \cup NPCs,
             instance \in 1..MaxActions}
    /\ activeReaction \in {NoReaction} \cup reactionQueue
    /\ turnLedger \subseteq Players \cup NPCs
    /\ reactionLedger \subseteq Players \cup NPCs
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
    /\ (phase = AwaitPlayerReaction) => controller = "player"
    /\ (phase = AwaitNpcReaction) => controller = "npc"
    /\ (phase \in {AwaitPlayerReaction, AwaitNpcReaction}) =>
          activeReaction # NoReaction /\ activeReaction.owner = activeUnit
    /\ (phase = AwaitMcts /\ candidateReady)
          => candidateRequestId = requestId
              /\ candidateVersion = stateVersion
    /\ (phase = RecoverReject) => recoveryRequestId = requestId
    /\ (phase = AwaitAck /\ lastAction = Reaction) => turnLedger = reactionLedger
    /\ (phase \in {AwaitPlayer, AwaitMcts}) => reactionQueue = {}

NoInvalidFallback ==
    fallbackAttempted => WaitLegal(activeUnit)

NoReactionStarvation ==
    /\ reactionQueue # {} =>
       <> (phase \in {AwaitPlayerReaction, AwaitNpcReaction})

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
