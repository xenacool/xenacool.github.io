------------------------------ MODULE GameLoopUi ------------------------------
EXTENDS Naturals, FiniteSets

(*
This is a concrete refinement layer over the phase ownership in GameLoop.tla.
The runtime phase is the abstract state.  The remaining variables model the
replicated target-menu and presentation state that the abstract model omits.

The model intentionally does not model tactical legality or collision
geometry.  It checks that a target token is committed only against the same
runtime snapshot that produced it, and that disabling animation playback does
not strand an accepted action in AwaitAck.
*)

CONSTANTS Units, Abilities, Versions, Sessions, MaxActions

AwaitPlayer == "AwaitPlayer"
AwaitAck == "AwaitAck"
Completed == "Completed"

Phases == {AwaitPlayer, AwaitAck, Completed}

TargetTokens == [unit: Units, ability: Abilities, target: Units]

VARIABLES phase, runtimeVersion, runtimeFingerprint, legalTargets,
          targetOpen, targetUnit, targetAbility, targetTarget,
          targetSession, targetVersion, targetFingerprint, nextSession,
          uiTargetOpen, uiTargetUnit, uiTargetAbility, uiTargetSession,
          uiTargetTarget, uiTargetVersion, uiTargetFingerprint,
          animationPlaying, barrier, pending,
          acknowledgedBarrier, staleRejection

vars == << phase, runtimeVersion, runtimeFingerprint, legalTargets,
           targetOpen, targetUnit, targetAbility, targetTarget,
           targetSession, targetVersion, targetFingerprint, nextSession, uiTargetOpen,
           uiTargetUnit, uiTargetAbility, uiTargetSession, uiTargetVersion,
           uiTargetTarget, uiTargetFingerprint, animationPlaying, barrier,
           pending, acknowledgedBarrier, staleRejection >>

Init ==
    /\ phase = AwaitPlayer
    /\ runtimeVersion = 0
    /\ runtimeFingerprint = 0
    /\ legalTargets = TargetTokens
    /\ targetOpen = FALSE
    /\ targetUnit = CHOOSE x \in Units : TRUE
    /\ targetAbility = CHOOSE x \in Abilities : TRUE
    /\ targetTarget = targetUnit
    /\ targetSession = 0
    /\ targetVersion = 0
    /\ targetFingerprint = 0
    /\ nextSession = 1
    /\ uiTargetOpen = FALSE
    /\ uiTargetUnit = targetUnit
    /\ uiTargetAbility = targetAbility
    /\ uiTargetTarget = targetTarget
    /\ uiTargetSession = 0
    /\ uiTargetVersion = 0
    /\ uiTargetFingerprint = 0
    /\ animationPlaying = TRUE
    /\ barrier = 0
    /\ pending = FALSE
    /\ acknowledgedBarrier = 0
    /\ staleRejection = FALSE

OpenTargets(unit, ability, target) ==
    /\ phase = AwaitPlayer
    /\ ~pending
    /\ unit \in Units
    /\ ability \in Abilities
    /\ [unit |-> unit, ability |-> ability, target |-> target] \in legalTargets
    /\ targetOpen' = TRUE
    /\ targetUnit' = unit
    /\ targetAbility' = ability
    /\ targetTarget' = target
    /\ targetSession' = nextSession
    /\ targetVersion' = runtimeVersion
    /\ targetFingerprint' = runtimeFingerprint
    /\ nextSession' = IF nextSession = MaxActions + 1 THEN 1 ELSE nextSession + 1
    /\ staleRejection' = FALSE
    /\ UNCHANGED << phase, runtimeVersion, runtimeFingerprint, legalTargets,
                    uiTargetOpen, uiTargetUnit, uiTargetAbility,
                    uiTargetTarget, uiTargetSession, uiTargetVersion,
                    uiTargetFingerprint,
                    animationPlaying, barrier, pending,
                    acknowledgedBarrier >>

RenderTargets ==
    /\ targetOpen
    /\ uiTargetOpen' = TRUE
    /\ uiTargetUnit' = targetUnit
    /\ uiTargetAbility' = targetAbility
    /\ uiTargetTarget' = targetTarget
    /\ uiTargetSession' = targetSession
    /\ uiTargetVersion' = targetVersion
    /\ uiTargetFingerprint' = targetFingerprint
    /\ UNCHANGED << phase, runtimeVersion, runtimeFingerprint, legalTargets,
                    targetOpen, targetUnit, targetAbility, targetTarget,
                    targetSession, targetVersion, targetFingerprint, nextSession,
                    animationPlaying, barrier, pending,
                    acknowledgedBarrier, staleRejection >>

(* A new authoritative snapshot may arrive while the browser still displays
   the previous target menu. *)
RuntimeStateAdvance(nextLegalTargets) ==
    /\ phase = AwaitPlayer
    /\ ~pending
    /\ runtimeVersion < MaxActions
    /\ runtimeVersion' = runtimeVersion + 1
    /\ runtimeFingerprint' = runtimeFingerprint + 1
    /\ nextLegalTargets \subseteq TargetTokens
    /\ legalTargets' = nextLegalTargets
    /\ targetOpen' = FALSE
    /\ UNCHANGED << phase, targetUnit, targetAbility,
                    targetTarget, targetSession, targetVersion,
                    targetFingerprint, nextSession, uiTargetOpen, uiTargetUnit,
                    uiTargetAbility, uiTargetTarget, uiTargetSession,
                    uiTargetVersion, uiTargetFingerprint,
                    animationPlaying, barrier, pending,
                    acknowledgedBarrier, staleRejection >>

CommitTarget ==
    /\ phase = AwaitPlayer
    /\ uiTargetOpen
    /\ pending = FALSE
    /\ barrier < MaxActions
    /\ IF targetOpen
          /\ uiTargetSession = targetSession
          /\ uiTargetVersion = targetVersion
          /\ uiTargetFingerprint = targetFingerprint
          /\ uiTargetFingerprint = runtimeFingerprint
          /\ [uiTargetUnit |-> uiTargetUnit,
              ability |-> uiTargetAbility,
              target |-> uiTargetTarget] \in legalTargets
       THEN
          /\ phase' = AwaitAck
          /\ pending' = TRUE
          /\ barrier' = barrier + 1
          /\ staleRejection' = FALSE
       ELSE
          /\ phase' = AwaitPlayer
          /\ pending' = FALSE
          /\ staleRejection' = TRUE
          /\ UNCHANGED << barrier >>
    /\ UNCHANGED << runtimeVersion, runtimeFingerprint, legalTargets,
                    targetOpen, targetUnit, targetAbility, targetTarget,
                    targetSession, targetVersion, targetFingerprint, nextSession,
                    uiTargetOpen, uiTargetUnit, uiTargetAbility,
                    uiTargetTarget, uiTargetSession, uiTargetVersion,
                    uiTargetFingerprint, animationPlaying,
                    acknowledgedBarrier >>

(* This is the required behavior when animation playback is disabled: the
   presentation barrier is acknowledged immediately rather than waiting for a
   render-loop sequence callback that will never be sent. *)
Acknowledge ==
    /\ phase = AwaitAck
    /\ pending
    /\ phase' = AwaitPlayer
    /\ pending' = FALSE
    /\ acknowledgedBarrier' = barrier
    /\ UNCHANGED << runtimeVersion, runtimeFingerprint, legalTargets,
                    targetOpen, targetUnit, targetAbility, targetTarget,
                    targetSession, targetVersion, targetFingerprint, nextSession,
                    uiTargetOpen, uiTargetUnit, uiTargetAbility,
                    uiTargetTarget, uiTargetSession, uiTargetVersion,
                    uiTargetFingerprint, animationPlaying,
                    barrier, staleRejection >>

ToggleAnimation ==
    /\ animationPlaying' = ~animationPlaying
    /\ UNCHANGED << phase, runtimeVersion, runtimeFingerprint, legalTargets,
                    targetOpen, targetUnit, targetAbility, targetTarget,
                    targetSession, targetVersion, targetFingerprint, nextSession,
                    uiTargetOpen, uiTargetUnit, uiTargetAbility,
                    uiTargetTarget, uiTargetSession, uiTargetVersion,
                    uiTargetFingerprint, barrier, pending,
                    acknowledgedBarrier, staleRejection >>

Complete ==
    /\ phase = AwaitPlayer
    /\ ~pending
    /\ phase' = Completed
    /\ staleRejection' = FALSE
    /\ UNCHANGED << runtimeVersion, runtimeFingerprint, legalTargets,
                    targetOpen, targetUnit, targetAbility, targetTarget,
                    targetSession, targetVersion, targetFingerprint, nextSession,
                    uiTargetOpen, uiTargetUnit, uiTargetAbility,
                    uiTargetTarget, uiTargetSession, uiTargetVersion,
                    uiTargetFingerprint, animationPlaying,
                    barrier, pending, acknowledgedBarrier >>

Stutter == UNCHANGED vars

Next ==
    \/ \E unit \in Units, ability \in Abilities, target \in Units :
          OpenTargets(unit, ability, target)
    \/ RenderTargets
    \/ \E nextLegalTargets \in SUBSET TargetTokens :
          RuntimeStateAdvance(nextLegalTargets)
    \/ CommitTarget
    \/ Acknowledge
    \/ ToggleAnimation
    \/ Complete
    \/ Stutter

TypeInvariant ==
    /\ phase \in Phases
    /\ runtimeVersion \in 0..MaxActions
    /\ runtimeFingerprint \in 0..MaxActions
    /\ legalTargets \subseteq TargetTokens
    /\ targetSession \in 0..MaxActions + 1
    /\ targetVersion \in 0..MaxActions
    /\ targetFingerprint \in 0..MaxActions
    /\ uiTargetSession \in 0..MaxActions + 1
    /\ uiTargetVersion \in 0..MaxActions
    /\ uiTargetFingerprint \in 0..MaxActions
    /\ barrier \in 0..MaxActions
    /\ acknowledgedBarrier \in 0..MaxActions

NoStaleCommit ==
    pending =>
        /\ phase = AwaitAck
        /\ uiTargetSession = targetSession
        /\ uiTargetVersion = runtimeVersion
        /\ uiTargetFingerprint = runtimeFingerprint
        /\ [uiTargetUnit |-> uiTargetUnit,
            ability |-> uiTargetAbility,
            target |-> uiTargetTarget] \in legalTargets

PendingOwnsBarrier == pending => barrier > acknowledgedBarrier

RejectedStaleTargets == staleRejection => phase = AwaitPlayer /\ ~pending

CompletedIsQuiescent == phase = Completed => ~pending

Spec == Init /\ [][Next]_vars

=============================================================================
