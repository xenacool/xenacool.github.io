Xenacool's Pystral Gate
=======================

## Configurable limits and defaults

| Area | Limit / validation | Default | Configuration or authority |
| --- | --- | ---: | --- |
| Tactical scenario | CT readiness threshold; valid range `1..=1_000_000` | `100` | [`SkirmishConfig::new_empty`](crates/games/src/skirmish.rs#L39-L61), [`set_ct_threshold`](crates/games/src/skirmish.rs#L156-L163) |
| Tactical scenario | Maximum rounds; `0` means unlimited | `0` | [`set_maximum_turn_count`](crates/games/src/skirmish.rs#L166-L168), [`turn_limit_reached`](crates/runtime/src/pg_rpg/simulation.rs#L458-L460) |
| Tactical map | Built-in horizontal radius | `6` | [`SkirmishConfig::new_empty`](crates/games/src/skirmish.rs#L46-L55) |
| Tactical map | Built-in layer bounds | `0..=0` | [`GridBounds` initialization](crates/games/src/skirmish.rs#L46-L51) |
| Unit resources | Action points per refreshed turn | `4` | [`UnitState` construction](crates/games/src/skirmish.rs#L402-L432) |
| Movement search | Frontier destinations exposed to MCTS | `6` | [`MoveBehavior::add_own_tasks`](crates/games/src/tasks.rs#L149-L163) |
| Cell areas | Area radius when omitted for an `Area` ability | `1` | [`ScriptAbilityDef::resolve`](crates/games/src/ruleset.rs#L65-L82) |
| NPC search | Rhai pg_rpg visits / depth / seed | `50` / `10` / `42` | [`default_mcts_config`](crates/runtime/src/pg_rpg/scripting/simulation.rs#L232-L238) |
| NPC search | Tactical adapter quality tolerance | `5` score points | [`request_npc_decision`](crates/runtime/src/pg_rpg/simulation.rs#L227-L280) |
| Rhai execution | Expression depths | `500`, `500` | [`register_all`](crates/runtime/src/pg_rpg/scripting.rs#L437-L444) |
| Rhai execution | Maximum operations | `100_000_000` | [`register_all`](crates/runtime/src/pg_rpg/scripting.rs#L437-L444) |
| Rhai execution | Maximum variables | `1000` | [`register_all`](crates/runtime/src/pg_rpg/scripting.rs#L437-L444) |
| Projectile solver | Speed range / step | `10..=25` / `5` | [`TrajectoryRequest::new`](crates/physics/src/lib.rs#L121-L137) |
| Projectile solver | Angle range / step | `5..=85` degrees / `2` | [`TrajectoryRequest::new`](crates/physics/src/lib.rs#L121-L137) |
| Projectile solver | Gravity / integration step / max steps | `9.81` / `0.05` / `100` | [`TrajectoryRequest::new`](crates/physics/src/lib.rs#L121-L137) |
| Projectile solver | Ground cutoff / collider radius | `-5.0` / `0.05` | [`TrajectoryRequest::new`](crates/physics/src/lib.rs#L121-L137) |
| Movement presentation | Default transition duration / timestep / tween | `500 ms` / `16 ms` / `SineInOut` | [`default_movement_transition`](crates/runtime/src/game_loop.rs#L356-L362) |

For the built-in defaults, the equivalent Rhai configuration is:

```rhai
let scenario = new_skirmish_config(42);
scenario.set_ct_threshold(100);
scenario.set_maximum_turn_count(0);

let mcts = new_mcts_config(); // visits 50, depth 10, seed 42
```

Limits that are owned by the external `npc-engine` library, such as the native
`MCTSConfiguration` field defaults, are documented in the
[`reference/npc-engine` source](reference/npc-engine/npc-engine-core/src/config.rs).

## Temporarily disabled acceptance tests

These tests were audited against a clean git baseline on 2026-08-14. The
remaining disabled tests fail independently of the current casualty and worker
protocol changes; each has a local TODO describing its re-enable condition.

- [`worker_heartbeat.spec.js`](tests/playwright/worker_heartbeat.spec.js) — the
  secondary-job Fireball followed by Wait fixture is unstable.

Do not treat these skips as gameplay approval; re-enable them when their TODO
conditions are met and retain the focused casualty/outcome gates.
