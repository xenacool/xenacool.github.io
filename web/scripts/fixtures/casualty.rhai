// Test-only fixture — loaded by tests/playwright/casualty_boundary.spec.js and fireball_victory.spec.js; never in manifest.json.
// Deterministic low-health overlay for the canonical pg_rpg browser gate.
// The scenario definition, assets, and runtime protocol remain shared with
// skirmish.rhai; only the initial health values are specialized here.
// @include "/scenarios/skirmish.rhai"
sim.set_agent_controller(1, "Player");
sim.set_agent_controller(2, "WaitOnly");
sim.set_agent_controller(5, "WaitOnly");
sim.set_agent_controller(3, "WaitOnly");
sim.set_agent_controller(4, "WaitOnly");
sim.set_agent_controller(6, "WaitOnly");
sim.set_agent_health(3, 1);
sim.set_agent_health(4, 0);
sim.set_agent_health(6, 0);
