------------------------------ MODULE CasualtyBoundary ------------------------------
EXTENDS Naturals, Sequences, FiniteSets

(***************************************************************************)
(* A deliberately small refinement of the gameplay boundary protocol.      *)
(*                                                                         *)
(* This model does not model combat legality or damage values. It models   *)
(* the ownership consequences of an authoritative mutation that kills a    *)
(* unit: dead units remain identifiable for history, but can never own a    *)
(* continuation, and terminal classification happens before readiness.     *)
(***************************************************************************)

CONSTANTS Players, NPCs, MaxActions

Units == Players \cup NPCs
TeamOf == [unit \in Units |-> IF unit \in Players THEN 1 ELSE 2]
AllPhases == {"AwaitBoundary", "AwaitPlayer", "AwaitMcts", "AwaitAck", "Completed"}
DecisionPhases == {"AwaitPlayer", "AwaitMcts"}
NoValue == "None"
Actions == {"Move", "Ability", "Wait"}
Outcomes == {"Victory", "Defeat", "Draw"}

VARIABLES phase, alive, activeUnit, controller, lastAction, lastTarget,
          casualtyPresented, pendingBarrier, stateVersion, history,
          completionCount, outcome

vars == << phase, alive, activeUnit, controller, lastAction, lastTarget,
           casualtyPresented, pendingBarrier, stateVersion, history,
           completionCount, outcome >>

Init ==
    /\ phase = "AwaitBoundary"
    /\ alive = Units
    /\ activeUnit = NoValue
    /\ controller = NoValue
    /\ lastAction = NoValue
    /\ lastTarget = NoValue
    /\ casualtyPresented = FALSE
    /\ pendingBarrier = FALSE
    /\ stateVersion = 0
    /\ history = << >>
    /\ completionCount = 0
    /\ outcome = NoValue

LivingTeams == {TeamOf[unit] : unit \in alive}
Terminal == Cardinality(LivingTeams) <= 1

ClassifyOutcome ==
    IF Cardinality(LivingTeams) = 0 THEN "Draw"
    ELSE IF TeamOf[CHOOSE unit \in alive : TRUE] = 1 THEN "Victory"
    ELSE "Defeat"

ChooseReady ==
    /\ phase = "AwaitBoundary"
    /\ ~Terminal
    /\ \E unit \in alive :
          /\ activeUnit' = unit
          /\ controller' = IF unit \in Players THEN "player" ELSE "npc"
          /\ phase' = IF unit \in Players THEN "AwaitPlayer" ELSE "AwaitMcts"
    /\ UNCHANGED << alive, lastAction, lastTarget, casualtyPresented,
                    pendingBarrier, stateVersion, history, completionCount,
                    outcome >>

CompleteBoundary ==
    /\ phase = "AwaitBoundary"
    /\ Terminal
    /\ phase' = "Completed"
    /\ outcome' = ClassifyOutcome
    /\ completionCount' = completionCount + 1
    /\ history' = Append(history,
          [event |-> "GameCompleted", outcome |-> ClassifyOutcome])
    /\ UNCHANGED << alive, activeUnit, controller, lastAction, lastTarget,
                    casualtyPresented, pendingBarrier, stateVersion >>

(* A commit represents the authoritative mutation and presentation barrier.
   The target may be the active unit to model reaction/self-casualty paths. *)
Commit(action, target) ==
    /\ phase \in DecisionPhases
    /\ activeUnit \in alive
    /\ action \in Actions
    /\ target \in alive
    /\ stateVersion < MaxActions
    /\ phase' = "AwaitAck"
    /\ pendingBarrier' = TRUE
    /\ lastAction' = action
    /\ lastTarget' = target
    /\ alive' = alive \ {target}
    /\ casualtyPresented' = TRUE
    /\ stateVersion' = stateVersion + 1
    /\ history' = Append(history,
          [event |-> "CasualtyPresented", unit |-> target,
           actor |-> activeUnit])
    /\ UNCHANGED << activeUnit, controller, completionCount, outcome >>

Acknowledge ==
    /\ phase = "AwaitAck"
    /\ pendingBarrier
    /\ pendingBarrier' = FALSE
    /\ phase' = "AwaitBoundary"
    /\ UNCHANGED << alive, activeUnit, controller, lastAction, lastTarget,
                    casualtyPresented, stateVersion, history,
                    completionCount, outcome >>

Stutter == UNCHANGED vars

Next ==
    \/ ChooseReady
    \/ CompleteBoundary
    \/ \E action \in Actions, target \in Units : Commit(action, target)
    \/ Acknowledge
    \/ Stutter

TypeInvariant ==
    /\ phase \in AllPhases
    /\ alive \subseteq Units
    /\ activeUnit \in Units \cup {NoValue}
    /\ controller \in {"player", "npc", NoValue}
    /\ lastAction \in Actions \cup {NoValue}
    /\ lastTarget \in Units \cup {NoValue}
    /\ pendingBarrier \in BOOLEAN
    /\ stateVersion \in 0..MaxActions
    /\ completionCount \in Nat
    /\ outcome \in Outcomes \cup {NoValue}

NoDeadContinuation ==
    phase \in DecisionPhases => activeUnit \in alive

NoDeadTarget ==
    phase = "AwaitAck" => lastTarget \notin alive

TerminalOwnsBoundary ==
    phase \in DecisionPhases => ~Terminal

PresentationBeforeCompletion ==
    phase = "Completed" => casualtyPresented /\ Len(history) > 0

CompletionIsQuiescent ==
    phase = "Completed" => ~pendingBarrier /\ completionCount = 1

NoPostCompletionMutation ==
    phase = "Completed" => alive \subseteq Units

SingleCompletion == completionCount <= 1

Fairness ==
    /\ WF_vars(Acknowledge)
    /\ WF_vars(CompleteBoundary)
    /\ WF_vars(ChooseReady)

CompletionEventuallyClassified ==
    [](phase = "AwaitBoundary" /\ Terminal ~> phase = "Completed")

Spec == Init /\ [][Next]_vars /\ Fairness

=============================================================================
