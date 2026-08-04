# Game-loop protocol model

`GameLoop.tla` models the player/NPC continuation protocol independently of
Rust and the browser. It checks the ownership and acknowledgment boundaries:

```text
boundary → player/MCTS decision → commit → presentation acknowledgment
         → next decision or next scheduler boundary
```

The model deliberately keeps tactical legality abstract and uses a small
`MaxActions` bound for finite TLC exploration. Its first purpose is to catch
protocol errors such as stepping outside `AwaitBoundary`, accepting two
candidates for one request, or allowing mutation after completion.

Local TLA+/TLC tools belong in `spec/.tla-tools/` and generated output belongs
in `spec/_build/`; both are gitignored. With `tla2tools.jar` available:

```sh
java -cp spec/.tla-tools/tla2tools.jar tlc2.TLC \
  -config spec/GameLoop.cfg spec/GameLoop.tla
```

The concrete ui/runtime refinement layer is checked separately:

```sh
make tla-ui-check
```

`GameLoopUi.tla` keeps the abstract phase contract but adds runtime target
snapshots, rendered target tokens, state versions, target sessions, and
presentation barriers. It checks that stale target commits are rejected and
that an accepted action can be acknowledged when animation playback is
disabled. It is intentionally not a model of tactical legality or collision
geometry.

The Rust implementation should preserve the same invariants, with the
additional memory invariant that one transition does not clone the complete
history and asset-bearing simulation into multiple owners.

The model also defines the UI projection contract used by the browser:
`AwaitPlayer` owns an actionable player menu, `AwaitAck` owns the pending
action state and committed feedback, and `Completed` owns neither. The runtime
`TransientState.action_feedback` field carries that committed feedback across
transient renders so presentation updates cannot overwrite an acknowledged
protocol state with the default menu prompt.

`WorkerReplication.tla` models the gloo worker/main replication boundary in
`crates/gate/src/worker.rs` and `crates/gate/src/main.rs`. `Watermark` is the
main-input acknowledgment; `Heartbeat(latest worker output seq)` is a separate
worker-progress signal. It checks ordered linearizable replication, no future
observation, safe watermarks, and eventual delivery under fair scheduling:

```sh
java -cp spec/.tla-tools/tla2tools.jar tlc2.TLC \
  -config spec/WorkerReplication.cfg spec/WorkerReplication.tla
```
