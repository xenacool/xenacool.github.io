// Deterministic low-health overlay for the canonical pg_rpg browser gate.
// The scenario definition, assets, and runtime protocol remain shared with
// skirmish.rhai; only the initial health values are specialized here.
// @include "skirmish.rhai"
sim.set_agent_health(3, 1);
sim.set_agent_health(4, 0);
