// Test-only deterministic player-boundary overlay for browser UI contracts.
// Production jobs, abilities, roster, positions, and resources remain canonical.
// @include "/scenarios/skirmish.rhai"
sim.set_agent_controller(1, "Player");
sim.set_agent_controller(2, "WaitOnly");
sim.set_agent_controller(5, "WaitOnly");
sim.set_agent_controller(3, "WaitOnly");
sim.set_agent_controller(4, "WaitOnly");
sim.set_agent_controller(6, "WaitOnly");
sim.set_agent_position(3, cell(hex(1, 0), 0));
