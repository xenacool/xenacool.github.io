-------------------------- MODULE PlaybackPresentation --------------------------
EXTENDS Naturals, FiniteSets

(* Bounded model of log playback overlays in LoopHandler.  History is the
   authority; a tween is only a presentation owned by one cursor epoch. *)
CONSTANT MaxHistory
NoTween == MaxHistory + 1

VARIABLES historyLen, cursor, playing, epoch, tweenEpoch, tweenEvent
vars == <<historyLen, cursor, playing, epoch, tweenEpoch, tweenEvent>>

Init ==
    /\ historyLen = 0 /\ cursor = 0 /\ playing = TRUE /\ epoch = 0
    /\ tweenEpoch = NoTween /\ tweenEvent = NoTween

Append ==
    /\ historyLen < MaxHistory
    /\ historyLen' = historyLen + 1
    /\ cursor' = IF playing THEN historyLen + 1 ELSE cursor
    /\ UNCHANGED <<playing, epoch, tweenEpoch, tweenEvent>>

Advance ==
    /\ playing /\ cursor < historyLen
    /\ cursor' = cursor + 1
    /\ tweenEpoch' = epoch /\ tweenEvent' = cursor
    /\ UNCHANGED <<historyLen, playing, epoch>>

Scrub ==
    /\ \E target \in 0..historyLen : target # cursor
    /\ cursor' \in 0..historyLen /\ playing' = FALSE
    /\ epoch' = IF epoch < MaxHistory THEN epoch + 1 ELSE epoch
    /\ tweenEpoch' = NoTween /\ tweenEvent' = NoTween
    /\ UNCHANGED historyLen

TogglePlay ==
    /\ playing' = ~playing
    /\ epoch' = IF epoch < MaxHistory THEN epoch + 1 ELSE epoch
    /\ tweenEpoch' = NoTween /\ tweenEvent' = NoTween
    /\ UNCHANGED <<historyLen, cursor>>

Reset ==
    /\ historyLen' = 0 /\ cursor' = 0 /\ playing' = TRUE
    /\ epoch' = IF epoch < MaxHistory THEN epoch + 1 ELSE epoch
    /\ tweenEpoch' = NoTween /\ tweenEvent' = NoTween

Stutter == UNCHANGED vars
Next == Append \/ Advance \/ Scrub \/ TogglePlay \/ Reset \/ Stutter

TypeInvariant ==
    /\ historyLen \in 0..MaxHistory /\ cursor \in 0..historyLen
    /\ playing \in BOOLEAN /\ epoch \in 0..MaxHistory
    /\ tweenEpoch \in {NoTween} \cup Nat /\ tweenEvent \in {NoTween} \cup Nat

NoStaleTween == tweenEpoch # NoTween => /\ tweenEpoch = epoch /\ tweenEvent < cursor
PausedHasNoTween == ~playing => tweenEpoch = NoTween

Spec == Init /\ [][Next]_vars
=============================================================================
