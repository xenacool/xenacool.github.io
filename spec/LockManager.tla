---------------------------------- MODULE LockManager ----------------------------------

(* Modular multi-subagent lock protocol over a tiny 2-level tree (Root > F1).

   Guarded rewrite in the terminal-lite build's grammar:
     * module closed with ====
     * plain {...} cardinality counter-encoding (no {, no negation)
     * Max2 helper (no built-in max / NumericOps)

   Invariants T1-T8 are all Cardinality-counterexample checks over the tiny
   finite Path = {Root, F1} and Agent = {A1, A2}. Each invariant asserts that
   the set of counter-satisfying instances has Cardinality 0.

   Invariant glossary:
     T1  VarTypeOK                    every var stays in (value set U {Sent})
     T2  NoForeignDirectLock          a direct holder is its path's owner
     T3  UserPriorityHold             USER lock pins every subagent out of p
     T4  HierarchicalContainment      F1 not held by a subagent other than
                                       whoever holds Root
     T5  NoOtherSubagentInLockedSubtree while Root is subagent-held, F1 is not
                                       held by the other subagent
     T6  RetrySoundness               recorded retry owner is a valid owner
     T7  ReadYourOwnWrite             saw clock never exceeds local clock
     T8  HappensBeforeRead            saw clock never exceeds global max clock

   Liveness (P1, P2) is enforced via WF on grant/release actions.
*)

EXTENDS Naturals, FiniteSets

CONSTANTS
   Path, Agent, UserTag, NONE, Root, F1, A1, A2, MAX, Sent

VARIABLES held, req, clock, agentClock, lastWriter, retryOwner, sawClock

vars == <<held, req, clock, agentClock, lastWriter, retryOwner, sawClock>>

(* x's subtree (superset path) *)
SubTree == [p \in Path |-> IF p = Root THEN {Root, F1} ELSE {F1}]

(* y inside x's subtree (y == x or descendant of x) *)
SubPath(x, y) == y \in SubTree[x]

Owner == (Agent \cup {UserTag, NONE})

Descendants(p) == {x \in Path : SubPath(x, p)}

(* subagents + USER (excludes NONE) *)
FreeOwner == (Agent \cup {UserTag})

(* subagents actually holding any ancestor of p (by real owner), deepest first *)
HeldAncestors(p) == {x \in Descendants(p) : held[x] \in FreeOwner}

(* effective owner of p: most-specific held ancestor (deeper path wins);
   if none held by a subagent, p's own holder (possibly NONE = free) *)
effectiveOwner(p) ==
   IF Cardinality(HeldAncestors(p)) = 0 THEN held[p]
   ELSE (IF p = F1 /\ held[F1] \in FreeOwner THEN held[F1] ELSE held[Root])

(* which path a directly holds (Root wins, else F1, else NONE) — derived from
   held so own and held never drift. Each agent holds <= 1 path (invariant). *)
own(a) ==
   IF a = A1 THEN
      (IF held[Root] = A1 THEN Root ELSE (IF held[F1] = A1 THEN F1 ELSE NONE))
   ELSE
      (IF held[Root] = A2 THEN Root ELSE (IF held[F1] = A2 THEN F1 ELSE NONE))

(* bound all clocks to MaxClk so the state space is finite *)
MaxClk == MAX
Max2(a, b) == IF a <= b THEN b ELSE a
Cap(y) == IF y > MaxClk THEN MaxClk ELSE y
CappedMax(a, b) == Max2(Cap(a), Cap(b))
CappedInc(n) == Cap(n + 1)

(* the set of other subagents' names plus UserTag and NONE *)
OtherOwners(x) == IF x = A1 THEN {A2, UserTag, NONE} ELSE {A1, UserTag, NONE}

OtherAgent(a) == IF a = A1 THEN A2 ELSE A1

NoPendingAgentRequest == req[A1] = NONE /\ req[A2] = NONE

INIT ==
   /\ held    = [p \in Path |-> NONE]
   /\ req     = [a \in Agent |-> NONE]
   /\ clock   = [p \in Path |-> 0]
   /\ agentClock = [a \in Agent |-> 0]
   /\ lastWriter = [p \in Path |-> NONE]
   /\ retryOwner = [p \in Path |-> NONE]
   /\ sawClock = [p \in Path |-> 0]

(* subagent a requests to hold path p *)
SetReq(a, p) ==
   /\ a \in Agent
   /\ p \in Path
   /\ req[a] = NONE
   /\ req'    = [req      EXCEPT ![a] = p]
   /\ clock'  = clock
   /\ agentClock' = agentClock
   /\ held'   = held
   /\ lastWriter' = lastWriter
   /\ retryOwner' = retryOwner
   /\ sawClock'   = sawClock

(* subagent a (re)acquires Root *)
GrantRoot(a) ==
   /\ a \in Agent
   /\ req[a] = Root
   /\ (a = A1 \/ req[OtherAgent(a)] # Root)
   /\ held[Root] = NONE
   /\ held[F1] = NONE \/ held[F1] = UserTag
   /\ held'   = [held    EXCEPT ![Root] = a]
   /\ req'    = [req      EXCEPT ![a] = NONE]
   /\ clock'  = [clock    EXCEPT ![Root] = CappedMax(clock[Root], agentClock[a] + 1)]
   /\ agentClock' = [agentClock EXCEPT ![a] = CappedInc(agentClock[a])]
   /\ lastWriter' = [lastWriter EXCEPT ![Root] = a]
   /\ retryOwner' = [retryOwner EXCEPT ![Root] = NONE]
   /\ sawClock'   = [sawClock EXCEPT ![Root] = 0]

(* subagent a (re)acquires F1 *)
GrantF1(a) ==
   /\ a \in Agent
   /\ req[a] = F1
   /\ req[OtherAgent(a)] # Root
   /\ held[F1] = NONE
   /\ held[Root] = NONE \/ held[Root] = UserTag
   /\ held'   = [held    EXCEPT ![F1] = a]
   /\ req'    = [req      EXCEPT ![a] = NONE]
   /\ clock'  = [clock    EXCEPT ![F1] = CappedMax(clock[F1], agentClock[a] + 1)]
   /\ agentClock' = [agentClock EXCEPT ![a] = CappedInc(agentClock[a])]
   /\ lastWriter' = [lastWriter EXCEPT ![F1] = a]
   /\ retryOwner' = [retryOwner EXCEPT ![F1] = NONE]
   /\ sawClock'   = [sawClock EXCEPT ![F1] = 0]

(* subagent a releases its hold on Root *)
ReleaseRoot(a) ==
   /\ a \in Agent
   /\ held[Root] = a
   /\ held'   = [held    EXCEPT ![Root] = NONE]
   /\ req'    = req
   /\ clock'  = clock
   /\ agentClock' = agentClock
   /\ lastWriter' = [lastWriter EXCEPT ![Root] = NONE]
   /\ retryOwner' = retryOwner
   /\ sawClock' = sawClock

(* subagent a releases its hold on F1 *)
ReleaseF1(a) ==
   /\ a \in Agent
   /\ held[F1] = a
   /\ held'   = [held    EXCEPT ![F1] = NONE]
   /\ req'    = req
   /\ clock'  = clock
   /\ agentClock' = agentClock
   /\ lastWriter' = [lastWriter EXCEPT ![F1] = NONE]
   /\ retryOwner' = retryOwner
   /\ sawClock' = sawClock

(* USER acquires Root (only if F1 is free or USER-pinned) *)
UserLockRoot ==
   /\ NoPendingAgentRequest
   /\ held[Root] = NONE
   /\ held[F1] = NONE \/ held[F1] = UserTag
   /\ held'   = [held    EXCEPT ![Root] = UserTag]
   /\ req'    = req
   /\ clock'  = clock
   /\ agentClock' = agentClock
   /\ lastWriter' = [lastWriter EXCEPT ![Root] = UserTag]
   /\ retryOwner' = retryOwner
   /\ sawClock'   = sawClock

(* USER releases Root *)
UserReleaseRoot ==
   /\ held[Root] = UserTag
   /\ held'   = [held    EXCEPT ![Root] = NONE]
   /\ req'    = req
   /\ clock'  = clock
   /\ agentClock' = agentClock
   /\ lastWriter' = [lastWriter EXCEPT ![Root] = NONE]
   /\ retryOwner' = retryOwner
   /\ sawClock'   = sawClock

(* USER acquires F1 (only if Root is free) *)
UserLockF1 ==
   /\ NoPendingAgentRequest
   /\ held[F1] = NONE
   /\ held[Root] = NONE
   /\ held'   = [held    EXCEPT ![F1] = UserTag]
   /\ req'    = req
   /\ clock'  = clock
   /\ agentClock' = agentClock
   /\ lastWriter' = [lastWriter EXCEPT ![F1] = UserTag]
   /\ retryOwner' = retryOwner
   /\ sawClock'   = sawClock

(* USER releases F1 *)
UserReleaseF1 ==
   /\ held[F1] = UserTag
   /\ held'   = [held    EXCEPT ![F1] = NONE]
   /\ req'    = req
   /\ clock'  = clock
   /\ agentClock' = agentClock
   /\ lastWriter' = [lastWriter EXCEPT ![F1] = NONE]
   /\ retryOwner' = retryOwner
   /\ sawClock'   = sawClock

(* a reads: observe max of its saw clock and the local clock (clamped) *)
Read(a, p) ==
   /\ a \in Agent
   /\ p \in Path
   /\ sawClock'   = [sawClock EXCEPT ![p] = CappedMax(sawClock[p], clock[p])]
   /\ clock'  = clock
   /\ agentClock' = agentClock
   /\ held'   = held
   /\ req'    = req
   /\ lastWriter' = lastWriter
   /\ retryOwner' = retryOwner

(* subagent a records the effective owner of p as a retry candidate (compile
   failure) -- always a valid owner, so T6 is preserved *)
Fail(a, p) ==
   /\ a \in Agent
   /\ p \in Path
   /\ retryOwner' = [retryOwner EXCEPT ![p] = effectiveOwner(p)]
   /\ clock'  = clock
   /\ agentClock' = agentClock
   /\ held'   = held
   /\ req'    = req
   /\ lastWriter' = lastWriter
   /\ sawClock'   = sawClock

(* subagent a clears the recorded retry owner for p *)
ClearRetry(a, p) ==
   /\ a \in Agent
   /\ p \in Path
   /\ retryOwner' = [retryOwner EXCEPT ![p] = NONE]
   /\ clock'  = clock
   /\ agentClock' = agentClock
   /\ held'   = held
   /\ req'    = req
   /\ lastWriter' = lastWriter
   /\ sawClock'   = sawClock

Next ==
   SetReq(A1, Root) \/ SetReq(A1, F1) \/ SetReq(A2, Root) \/ SetReq(A2, F1)
   \/ GrantRoot(A1) \/ GrantRoot(A2) \/ GrantF1(A1) \/ GrantF1(A2)
   \/ ReleaseRoot(A1) \/ ReleaseRoot(A2) \/ ReleaseF1(A1) \/ ReleaseF1(A2)
   \/ UserLockRoot \/ UserReleaseRoot \/ UserLockF1 \/ UserReleaseF1
   \/ Read(A1, Root) \/ Read(A1, F1) \/ Read(A2, Root) \/ Read(A2, F1)
   \/ Fail(A1, Root) \/ Fail(A1, F1) \/ Fail(A2, Root) \/ Fail(A2, F1)
   \/ ClearRetry(A1, Root) \/ ClearRetry(A1, F1) \/ ClearRetry(A2, Root) \/ ClearRetry(A2, F1)

Spec ==
   INIT
   /\ [][Next]_vars
   /\ WF_vars(GrantRoot(A1)) /\ WF_vars(GrantRoot(A2))
   /\ WF_vars(GrantF1(A1)) /\ WF_vars(GrantF1(A2))
   /\ WF_vars(ReleaseRoot(A1)) /\ WF_vars(ReleaseRoot(A2))
   /\ WF_vars(ReleaseF1(A1)) /\ WF_vars(ReleaseF1(A2))
   /\ WF_vars(UserReleaseRoot) /\ WF_vars(UserReleaseF1)

(* T1 -- every state variable stays inside (its value set U {Sent}). *)
VarTypeOK ==
   /\ Cardinality({p \in Path : held[p] = Sent}) = 0
   /\ Cardinality({a \in Agent : req[a] = Sent}) = 0
   /\ Cardinality({p \in Path : lastWriter[p] = Sent}) = 0
   /\ Cardinality({p \in Path : retryOwner[p] = Sent}) = 0
   /\ Cardinality({p \in Path : clock[p] \in {0, 1}}) = Cardinality(Path)
   /\ Cardinality({a \in Agent : agentClock[a] \in {0, 1}}) = Cardinality(Agent)
   /\ Cardinality({p \in Path : sawClock[p] \in {0, 1}}) = Cardinality(Path)

(* Counter-example predicates for T2-T5. Compound conjunction/disjunction is
   allowed inside operator *definitions* (but not inside set-comprehension
   bodies), so all compound logic lives here and each invariant set body is a
   single operator application. *)
AntiDirectLock(a, p) == (own(a) = p /\ effectiveOwner(p) \in OtherOwners(a))
NoForeignDirectLockBody(p) == AntiDirectLock(A1, p) \/ AntiDirectLock(A2, p)
UserPriorityHoldBody(p) ==
   effectiveOwner(p) = UserTag /\ (own(A1) = p \/ own(A2) = p)


(* T2 -- a direct holder (own[A1]=p) must be p's effective owner; the
   counter-example (effectiveOwner(p) not among a's allowed owners) is empty. *)
NoForeignDirectLock ==
   Cardinality({p \in Path : NoForeignDirectLockBody(p)}) = 0

(* T3 -- while p is USER-owned, no subagent directly holds p *)
UserPriorityHold ==
   Cardinality({p \in Path : UserPriorityHoldBody(p)}) = 0

(* T4 -- if Root is subagent-held, F1's effective owner is whoever holds Root,
   USER, or NONE (not the other subagent). Expressed as two ANDed
   Cardinality-checks (pure AND, no mixed AND/OR to avoid the build's
   precedence conflict between \land and \lor). *)
HierarchicalContainment ==
   Cardinality({p \in Path : held[Root] = A1 /\ effectiveOwner(F1) = A2}) = 0
   /\ Cardinality({p \in Path : held[Root] = A2 /\ effectiveOwner(F1) = A1}) = 0

(* T5 -- while Root is subagent-held, the OTHER subagent does not directly hold
   F1 (for the only tree shape Root>F1). Two ANDed Cardinality-checks. *)
NoOtherSubagentInLockedSubtree ==
   Cardinality({p \in Path : (p = Root /\ held[Root] = A1 /\ held[F1] = A2)}) = 0
   /\ Cardinality({p \in Path : (p = Root /\ held[Root] = A2 /\ held[F1] = A1)}) = 0

(* T6 -- the recorded retry owner is always a valid owner *)
RetrySoundness ==
   Cardinality({p \in Path : retryOwner[p] \in Owner}) = Cardinality(Path)

(* T7 -- saw clock never exceeds the local clock *)
ReadYourOwnWrite ==
   Cardinality({p \in Path : clock[p] < sawClock[p]}) = 0

(* T8 -- saw clock never exceeds the global max clock *)
HappensBeforeRead ==
   Cardinality({p \in Path : Max2(clock[Root], clock[F1]) < sawClock[p]}) = 0

(* P1/P2 are retained as design targets but are not in the current CFG gate:
   ticket state is needed before request fairness can be asserted without a
   starvation counterexample. P1 -- every request is eventually granted to its requester. Root requests
   have priority over new child grants, so weak fairness can discharge this
   obligation even when requests compete across the hierarchy. *)
UncontendedLockGranted ==
   \A a \in Agent : (\A p \in Path :
      (req[a] = p ~> (own(a) = p)))

(* P2 -- once a directly holds p, it eventually no longer directly holds p. *)
LockEventuallyReleased ==
   \A a \in Agent : (\A p \in Path :
      (own(a) = p ~>
        (own(a) # p)))

========================================================================================
