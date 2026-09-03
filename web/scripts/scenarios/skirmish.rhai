// Tactical scenario and runtime-owned simulation configuration.
// 0.036 keeps a roughly 10% margin inside the unit pointy-hex footprint while
// keeping the captured slice aspect ratio intact in the native renderer.
let scaling = 0.036;
collection.add_atlas_spritestack("Skeleton_Minion", scaling, atlas, spritesheet_rgba, spritesheet_width);
collection.add_atlas_spritestack("Necromancer", scaling, atlas, spritesheet_rgba, spritesheet_width);
collection.add_atlas_spritestack("Caveman", scaling, atlas, spritesheet_rgba, spritesheet_width);
collection.add_atlas_spritestack("Mage", scaling, atlas, spritesheet_rgba, spritesheet_width);
history.define_asset_collection("primitives", collection);

let mcts = new_mcts_config();
mcts.set_visits(24);
mcts.set_depth(6);
mcts.set_seed(42);
mcts.set_minimum_hit_probability(0.20);
mcts.set_allow_desperation(false);
let scenario = new_skirmish_config(42);
scenario.set_maximum_turn_count(12);
let thick_skin = new_script_passive("Thick Skin");
thick_skin.add_stat_modifier("ArmorClass", 5);
let spell_echo = new_script_passive("Spell Echo");
spell_echo.set_damage_multiplier(2, 1);
let deaths_embrace = new_script_passive("Death's Embrace");
deaths_embrace.set_on_kill_heal(20);
let undead_resilience = new_script_passive("Undead Resilience");
undead_resilience.add_stat_modifier("ArmorClass", 2);
undead_resilience.add_stat_modifier("Constitution", 2);
scenario.add_script_passive(thick_skin);
scenario.add_script_passive(spell_echo);
scenario.add_script_passive(deaths_embrace);
scenario.add_script_passive(undead_resilience);
let counter_swing = new_script_reaction("Counter-Swing");
counter_swing.set_ap_cost(1);
counter_swing.set_target_damage(10);
let mana_shield = new_script_reaction("Mana Shield");
mana_shield.set_ap_cost(0);
mana_shield.set_mana_cost(10);
mana_shield.set_self_heal(5);
let vengeful_spirit = new_script_reaction("Vengeful Spirit");
vengeful_spirit.set_ap_cost(1);
vengeful_spirit.set_target_damage(15);
let bone_splinter = new_script_reaction("Bone Splinter");
bone_splinter.set_ap_cost(0);
bone_splinter.set_target_damage(5);
scenario.add_script_reaction(counter_swing);
scenario.add_script_reaction(mana_shield);
scenario.add_script_reaction(vengeful_spirit);
scenario.add_script_reaction(bone_splinter);
let plain_move = new_script_movement("Plain Move");
plain_move.add_step_cost(1, 1);
let manafeet = new_script_movement("Manafeet");
manafeet.add_step_cost(1, 1);
manafeet.set_mana_gain(5);
let shadow_step = new_script_movement("Shadow Step");
shadow_step.add_step_cost(1, 2);
shadow_step.set_crosses_holes(true);
shadow_step.set_teleport_range(2);
let rattle_dash = new_script_movement("Rattle Dash");
rattle_dash.add_step_cost(1, 1);
rattle_dash.add_step_cost(4, 2);
scenario.add_script_movement(plain_move);
scenario.add_script_movement(manafeet);
scenario.add_script_movement(shadow_step);
scenario.add_script_movement(rattle_dash);
// Seven turn tags are registered for the demo. Movement currently consumes no
// tags; abilities emit/consume them (notably Fresh Soul for Soul Drain).
fn add_tag(scenario, name, max_stacks) {
    let tag = new_script_tag(name);
    tag.set_max_stacks(max_stacks);
    scenario.add_script_tag(tag);
    scenario
}
scenario = add_tag(scenario, "Stun", 1);
scenario = add_tag(scenario, "Slow", 1);
scenario = add_tag(scenario, "Shield", 2);
scenario = add_tag(scenario, "Armor", 2);
scenario = add_tag(scenario, "Fear", 1);
scenario = add_tag(scenario, "Mana", 1);
scenario = add_tag(scenario, "Fresh Soul", 1);
fn authored_ability(name, ap_cost, range, delivery, stat1, factor1, stat2, factor2, tag, stacks) {
    let ability = new_script_ability(name);
    ability.set_ap_cost(ap_cost);
    ability.set_range(range);
    ability.set_delivery(delivery);
    if stat1 != "" { ability.add_scaling(stat1, factor1); }
    if stat2 != "" { ability.add_scaling(stat2, factor2); }
    if tag != "" { ability.add_emit_tag(tag, stacks); }
    ability
}

let club_smash = authored_ability("Club Smash", 2, 1, "Melee", "STR", 1.5, "", 0.0, "", 0);
scenario.add_script_ability(club_smash);

let rock_throw = authored_ability("Rock Throw", 2, 6, "StraightProjectile", "STR", 1.0, "DEX", 0.5, "", 0);
rock_throw.set_projectile_profile("Rock");
scenario.add_script_ability(rock_throw);

let primal_roar = authored_ability("Primal Roar", 3, 3, "Area", "CHA", 0.5, "", 0.0, "Stun", 1);
scenario.add_script_ability(primal_roar);

let fireball = authored_ability("Fireball", 3, 8, "ArcProjectile", "INT", 2.0, "", 0.0, "", 0);
fireball.set_projectile_profile("FireballOrb");
scenario.add_script_ability(fireball);

let frost_bolt = authored_ability("Frost Bolt", 2, 6, "StraightProjectile", "INT", 1.0, "WIS", 0.5, "Slow", 1);
frost_bolt.set_projectile_profile("FrostBolt");
scenario.add_script_ability(frost_bolt);

let arcane_shield = authored_ability("Arcane Shield", 2, 0, "SelfTarget", "INT", 0.5, "WIS", 1.0, "Shield", 2);
let shield = new_rpg_program();
shield.add_timed_modifier("ArmorClass", 4, 2, "RefreshReplace");
let shield_programs = new_rpg_programs();
shield_programs.add_program("OnAbilityResolve", shield);
arcane_shield.set_programs(shield_programs);
scenario.add_script_ability(arcane_shield);

let raise_skeleton = authored_ability("Raise Skeleton", 2, 1, "Area", "", 0.0, "", 0.0, "", 0);
raise_skeleton.set_health_cost(20);
raise_skeleton.set_health_floor(1);
raise_skeleton.set_target_rule("EmptyCell");
let raise = new_rpg_program();
raise.add_spawn_owned_unit("Skeleton_Minion", 2);
let raise_programs = new_rpg_programs();
raise_programs.add_program("OnAbilityResolve", raise);
raise_skeleton.set_programs(raise_programs);
scenario.add_script_ability(raise_skeleton);

let harvest_skeleton = authored_ability("Harvest Skeleton", 1, 4, "Melee", "", 0.0, "", 0.0, "Fresh Soul", 1);
harvest_skeleton.set_target_rule("OwnedSummon");
let harvest = new_rpg_program();
harvest.add_destroy_owned_summon();
harvest.add_heal_caster(10);
harvest.add_caster_mana(10);
let harvest_programs = new_rpg_programs();
harvest_programs.add_program("OnAbilityResolve", harvest);
harvest_skeleton.set_programs(harvest_programs);
scenario.add_script_ability(harvest_skeleton);

let soul_drain = authored_ability("Soul Drain", 3, 4, "StraightProjectile", "INT", 1.0, "CHA", 1.0, "", 0);
soul_drain.set_projectile_profile("SoulOrb");
soul_drain.set_mana_cost(10);
soul_drain.add_consume_tag("Fresh Soul", 1, 1);
let drain = new_rpg_program();
drain.add_heal_from_actual_damage(1, 2);
let drain_programs = new_rpg_programs();
drain_programs.add_program("OnAbilityResolve", drain);
soul_drain.set_programs(drain_programs);
scenario.add_script_ability(soul_drain);

let bone_armor = authored_ability("Bone Armor", 2, 0, "SelfTarget", "INT", 1.0, "", 0.0, "Armor", 2);
scenario.add_script_ability(bone_armor);
let bony_strike = authored_ability("Bony Strike", 1, 1, "Melee", "STR", 1.0, "", 0.0, "", 0);
scenario.add_script_ability(bony_strike);
let shield_bash = authored_ability("Shield Bash", 2, 1, "Melee", "STR", 0.5, "CON", 0.5, "Stun", 1);
scenario.add_script_ability(shield_bash);
let screech = authored_ability("Screech", 2, 3, "Area", "CHA", 0.2, "", 0.0, "Fear", 1);
scenario.add_script_ability(screech);
let caveman_job = new_script_job("Caveman");
caveman_job.set_base_stats(unit_stats(15, 8, 4, 4, 6, 14, 8, 12, 12, 4));
caveman_job.add_equipment_slot("MainHand");
caveman_job.add_equipment_slot("Body");
caveman_job.add_equipment_slot("Accessory");
caveman_job.set_passive_slots(1);
caveman_job.set_reaction_slots(1);
caveman_job.set_secondary_job_slots(1);
caveman_job.add_ability("Club Smash");
caveman_job.add_ability("Rock Throw");
caveman_job.add_ability("Primal Roar");
caveman_job.add_passive("Thick Skin");
caveman_job.add_reaction("Counter-Swing");
caveman_job.set_movement("Plain Move");
scenario.add_script_job(caveman_job);
let mage_job = new_script_job("Mage");
mage_job.set_base_stats(unit_stats(4, 6, 16, 12, 8, 6, 12, 8, 8, 5));
mage_job.add_equipment_slot("MainHand");
mage_job.add_equipment_slot("Body");
mage_job.add_equipment_slot("Accessory");
mage_job.set_passive_slots(1);
mage_job.set_reaction_slots(1);
mage_job.set_secondary_job_slots(1);
mage_job.add_ability("Fireball");
mage_job.add_ability("Frost Bolt");
mage_job.add_ability("Arcane Shield");
mage_job.add_passive("Spell Echo");
mage_job.add_reaction("Mana Shield");
mage_job.set_movement("Manafeet");
scenario.add_script_job(mage_job);
let necromancer_job = new_script_job("Necromancer");
necromancer_job.set_base_stats(unit_stats(6, 8, 14, 10, 12, 10, 10, 10, 10, 4));
necromancer_job.add_equipment_slot("MainHand");
necromancer_job.add_equipment_slot("Body");
necromancer_job.add_equipment_slot("Accessory");
necromancer_job.set_passive_slots(1);
necromancer_job.set_reaction_slots(1);
necromancer_job.set_secondary_job_slots(1);
necromancer_job.add_ability("Raise Skeleton");
necromancer_job.add_ability("Harvest Skeleton");
necromancer_job.add_ability("Soul Drain");
necromancer_job.add_ability("Bone Armor");
necromancer_job.add_passive("Death's Embrace");
necromancer_job.add_reaction("Vengeful Spirit");
necromancer_job.set_movement("Shadow Step");
scenario.add_script_job(necromancer_job);
let skeleton_job = new_script_job("Skeleton_Minion");
skeleton_job.set_base_stats(unit_stats(10, 12, 2, 2, 2, 8, 8, 8, 10, 6));
skeleton_job.add_equipment_slot("MainHand");
skeleton_job.add_equipment_slot("OffHand");
skeleton_job.add_equipment_slot("Body");
skeleton_job.set_passive_slots(1);
skeleton_job.set_reaction_slots(1);
skeleton_job.set_secondary_job_slots(1);
skeleton_job.add_ability("Bony Strike");
skeleton_job.add_ability("Shield Bash");
skeleton_job.add_ability("Screech");
skeleton_job.add_passive("Undead Resilience");
skeleton_job.add_reaction("Bone Splinter");
skeleton_job.set_movement("Rattle Dash");
scenario.add_script_job(skeleton_job);
let tactical_grid = new_grid_map();
tactical_grid.set_horizontal_bounds(hex(0, 0), 50);
tactical_grid.set_layer_bounds(0, 31);
for q in -6..7 {
    for r in -6..7 {
        tactical_grid.set_tile(cell(hex(q, r), 0), "grass");
    }
}
tactical_grid.set_tile(cell(hex(0, 0), 1), "rock");
tactical_grid.set_tile(cell(hex(1, -1), 1), "dirt");
scenario.set_grid(tactical_grid);
scenario.set_projectile_speed_min(10.0);
scenario.set_projectile_speed_max(25.0);
scenario.set_projectile_speed_step(5.0);
scenario.set_projectile_angle_min(5.0);
scenario.set_projectile_angle_max(85.0);
scenario.set_projectile_angle_step(2.0);
scenario.set_projectile_gravity(9.81);
scenario.set_projectile_time_step(0.05);
scenario.set_projectile_max_steps(100);
scenario.set_projectile_ground_cutoff(-5.0);
scenario.set_projectile_collider(ball_collider(0.05));
scenario.add_unit(1, 1, "Caveman", cell(hex(0, 0), 0));
scenario.add_unit(2, 1, "Mage", cell(hex(1, -1), 0));
scenario.add_unit(5, 1, "Necromancer", cell(hex(-1, 1), 0));
scenario.set_unit_mana(5, 0);
scenario.add_unit(3, 2, "Caveman", cell(hex(3, -3), 0));
scenario.add_unit(4, 2, "Mage", cell(hex(4, -3), 0));
scenario.add_unit(6, 2, "Skeleton_Minion", cell(hex(3, -2), 0));
scenario.set_unit_controller(1, "Player");
scenario.set_unit_controller(2, "Npc");
scenario.set_unit_controller(5, "Npc");
scenario.set_unit_controller(3, "Npc");
scenario.set_unit_controller(4, "Npc");
scenario.set_unit_controller(6, "Npc");
scenario.add_secondary_job(1, "Mage");
let sim = new_simulation(scenario, mcts);
let agents = sim.list_agents();
let default_transition = transition(500, 16.0, "SineInOut");
