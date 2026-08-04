// Test-only deterministic Necromancer economy overlay. The canonical scenario
// owns every ability, tag, summon template, cost, and effect program.
// @include "/scenarios/skirmish.rhai"
sim.set_agent_controller(1, "WaitOnly");
sim.set_agent_controller(2, "WaitOnly");
sim.set_agent_controller(5, "Player");
sim.set_agent_controller(3, "WaitOnly");
sim.set_agent_controller(4, "WaitOnly");
sim.set_agent_controller(6, "WaitOnly");
sim.set_agent_summon_controller(5, "WaitOnly");
sim.set_agent_health(1, 0);
sim.set_agent_health(2, 0);
sim.set_agent_health(4, 0);
sim.set_agent_health(6, 0);
sim.set_agent_position(3, cell(hex(1, 1), 0));
