-------------------------- MODULE PlaybackPresentation --------------------------
EXTENDS Naturals, FiniteSets

(* Bounded model of log playback overlays in LoopHandler.  History is the
   authority; a tween is only a presentation owned by one cursor epoch. *)
CONSTANTS MaxHistory, MaxClock
NoTween == MaxHistory + 1

VARIABLES historyLen, cursor, playing, epoch, tweenEpoch, tweenEvent,
          clock, tweenStarted, tweenCompleted, ackClock
vars == <<historyLen, cursor, playing, epoch, tweenEpoch, tweenEvent,
          clock, tweenStarted, tweenCompleted, ackClock>>

Init ==
    /\ historyLen = 0 /\ cursor = 0 /\ playing = TRUE /\ epoch = 0
    /\ tweenEpoch = NoTween /\ tweenEvent = NoTween /\ clock = 0
    /\ tweenStarted = NoTween /\ tweenCompleted = NoTween /\ ackClock = NoTween

Append ==
    /\ historyLen < MaxHistory
    /\ historyLen' = historyLen + 1
    /\ cursor' = IF playing THEN historyLen + 1 ELSE cursor
    /\ UNCHANGED <<playing, epoch, tweenEpoch, tweenEvent, clock, tweenStarted, tweenCompleted, ackClock>>

Advance ==
    /\ playing /\ cursor < historyLen /\ clock < MaxClock
    /\ cursor' = cursor + 1
    /\ tweenEpoch' = epoch /\ tweenEvent' = cursor /\ clock' = clock + 1
    /\ tweenStarted' = clock' /\ tweenCompleted' = NoTween /\ ackClock' = NoTween
    /\ UNCHANGED <<historyLen, playing, epoch>>

CompleteTween ==
    /\ tweenEpoch # NoTween /\ tweenCompleted = NoTween /\ clock < MaxClock
    /\ clock' = clock + 1 /\ tweenCompleted' = clock'
    /\ UNCHANGED <<historyLen, cursor, playing, epoch, tweenEpoch, tweenEvent, tweenStarted, ackClock>>

AckTween ==
    /\ tweenCompleted # NoTween /\ ackClock = NoTween /\ clock < MaxClock
    /\ clock' = clock + 1 /\ ackClock' = clock'
    /\ UNCHANGED <<historyLen, cursor, playing, epoch, tweenEpoch, tweenEvent, tweenStarted, tweenCompleted>>

Scrub ==
    /\ \E target \in 0..historyLen : target # cursor
    /\ cursor' \in 0..historyLen /\ playing' = FALSE
    /\ epoch' = IF epoch < MaxHistory THEN epoch + 1 ELSE epoch
    /\ tweenEpoch' = NoTween /\ tweenEvent' = NoTween /\ tweenStarted' = NoTween
    /\ tweenCompleted' = NoTween /\ ackClock' = NoTween
    /\ UNCHANGED <<historyLen, clock>>

TogglePlay ==
    /\ playing' = ~playing
    /\ epoch' = IF epoch < MaxHistory THEN epoch + 1 ELSE epoch
    /\ tweenEpoch' = NoTween /\ tweenEvent' = NoTween /\ tweenStarted' = NoTween
    /\ tweenCompleted' = NoTween /\ ackClock' = NoTween
    /\ UNCHANGED <<historyLen, cursor, clock>>

Reset ==
    /\ historyLen' = 0 /\ cursor' = 0 /\ playing' = TRUE
    /\ epoch' = IF epoch < MaxHistory THEN epoch + 1 ELSE epoch
    /\ tweenEpoch' = NoTween /\ tweenEvent' = NoTween /\ tweenStarted' = NoTween
    /\ tweenCompleted' = NoTween /\ ackClock' = NoTween
    /\ UNCHANGED clock

Stutter == UNCHANGED vars
Next == Append \/ Advance \/ CompleteTween \/ AckTween \/ Scrub \/ TogglePlay \/ Reset \/ Stutter

TypeInvariant ==
    /\ historyLen \in 0..MaxHistory /\ cursor \in 0..historyLen
    /\ playing \in BOOLEAN /\ epoch \in 0..MaxHistory
    /\ tweenEpoch \in {NoTween} \cup Nat /\ tweenEvent \in {NoTween} \cup Nat
    /\ clock \in 0..MaxClock /\ tweenStarted \in {NoTween} \cup Nat
    /\ tweenCompleted \in {NoTween} \cup Nat /\ ackClock \in {NoTween} \cup Nat

NoStaleTween == tweenEpoch # NoTween => /\ tweenEpoch = epoch /\ tweenEvent < cursor
PausedHasNoTween == ~playing => tweenEpoch = NoTween
CompletionFollowsStart == tweenCompleted # NoTween => tweenStarted < tweenCompleted
AckFollowsCompletion == ackClock # NoTween => tweenCompleted < ackClock

Spec == Init /\ [][Next]_vars
=============================================================================
