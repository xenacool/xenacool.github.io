// Initial render entities, map, cameras, and projectile presentation.
let prompt_id = 999;
history.spawn_entity(prompt_id, "prompt", hex(0, 0), sprite_init_properties());
history.set(prompt_id, "z", 0.0);
history.set(prompt_id, "scale", 1.0);
history.set(prompt_id, "rotation_z", 0.0);
history.set(prompt_id, "rotation_y", 0.0);
history.set(prompt_id, "cam_offset_x", 0.0);
history.set(prompt_id, "cam_offset_y", 0.0);
history.set(prompt_id, "cam_offset_z", 0.0);
history.set(prompt_id, "material", "grass");

let unit_kinds = ["", "Caveman", "Mage", "Necromancer", "Skeleton_Minion"];
for id in agents {
    let kind = unit_kinds[id];
    let pos = sim.get_agent_position(id);
    history.spawn_entity(id, kind, pos, character_init_properties());
    history.set(id, "asset", kind);
    history.set(id, "scale", 0.6);
    history.set(id, "z", 1.8);
    history.set(id, "layer", 0);
    history.set(id, "rotation_z", 0.0);
    history.set(id, "rotation_y", 0.0);
    history.set(id, "cam_offset_x", 0.0);
    history.set(id, "cam_offset_y", 0.0);
    history.set(id, "cam_offset_z", 0.0);
    history.set(id, "material", "grass");
}

let demo_map = new_hex_map();
{
    for q in -6..7 {
        for r in -6..7 {
            let h = hex(q, r);
            let base_height = ((q + r) % 4) * 0.05;
            let is_wall = q == 0;
            let is_hole = is_wall && (r >= -1 && r <= 1);
            if is_wall && !is_hole {
                for i in 0..10 { demo_map.add_tile(new_hex_tile(h, i * 0.5, 0.5, "rock")); }
            } else if is_wall && is_hole {
                for i in 0..2 { demo_map.add_tile(new_hex_tile(h, i * 0.5, 0.5, "rock")); }
                for i in 8..10 { demo_map.add_tile(new_hex_tile(h, i * 0.5, 0.5, "rock")); }
                demo_map.add_tile(new_hex_tile(h, 1.0, 0.1, "dirt"));
            } else {
                demo_map.add_tile(new_hex_tile(h, 0.0, 0.1 + base_height, "grass"));
            }
        }
    }
}

let world_id = history.spawn_entity(0, "world", hex(0, 0), ["map", "lighting"]);
history.set(world_id, "map", demo_map);
history.set(world_id, "lighting", default_lighting());
history.set(world_id, "z", 0.0);
history.set(world_id, "scale", 1.0);
history.set(world_id, "rotation_z", 0.0);
history.set(world_id, "rotation_y", 0.0);
history.set(world_id, "cam_offset_x", 0.0);
history.set(world_id, "cam_offset_y", 0.0);
history.set(world_id, "cam_offset_z", 0.0);
history.set(world_id, "material", "grass");

// Visible procedural rock props. Their geometry comes from the Rhai-defined
// Rock spritestack in assets/primitives.rhai.
let rock_hexes = [
    hex(-4, -4),
    hex(4, -4),
    hex(-4, 4),
    hex(4, 4),
    hex(0, -5),
];
let rock_id = 30;
for rock_hex in rock_hexes {
    history.spawn_entity(rock_id, "rock", rock_hex, sprite_init_properties());
    history.set(rock_id, "asset", "Rock");
    history.set(rock_id, "scale", 0.65);
    history.set(rock_id, "z", 0.45);
    history.set(rock_id, "rotation_z", 0.0);
    history.set(rock_id, "rotation_y", 0.0);
    history.set(rock_id, "cam_offset_x", 0.0);
    history.set(rock_id, "cam_offset_y", 0.0);
    history.set(rock_id, "cam_offset_z", 0.0);
    history.set(rock_id, "material", "rock");
    rock_id += 1;
}

fn setup_camera_props(id, angle, dist, height, tx, ty, tz) {
    this.set(id, "angle", angle);
    this.set(id, "distance", dist);
    this.set(id, "height", height);
    this.set(id, "target_x", tx);
    this.set(id, "target_y", ty);
    this.set(id, "target_z", tz);
    this.set(id, "z", 0.0);
    this.set(id, "scale", 1.0);
    this.set(id, "rotation_z", 0.0);
    this.set(id, "rotation_y", 0.0);
    this.set(id, "cam_offset_x", 0.0);
    this.set(id, "cam_offset_y", 0.0);
    this.set(id, "cam_offset_z", 0.0);
    this.set(id, "material", "grass");
}
let cam1 = history.spawn_entity(100, "camera", hex(0, 0), camera_init_properties());
let cam2 = history.spawn_entity(101, "camera", hex(0, 0), camera_init_properties());
let cam3 = history.spawn_entity(102, "camera", hex(0, 0), camera_init_properties());
let cam4 = history.spawn_entity(103, "camera", hex(0, 0), camera_init_properties());
history.configure_transition(cam1, default_transition);
history.configure_transition(cam2, default_transition);
history.configure_transition(cam3, default_transition);
history.configure_transition(cam4, default_transition);
history.setup_camera_props(cam1, 0.0, 20.0, 12.0, 0.0, 0.0, 0.0);
history.setup_camera_props(cam2, 1.570795, 25.0, 15.0, 0.0, 0.0, 0.0);
history.setup_camera_props(cam3, 3.14159, 20.0, 12.0, 0.0, 0.0, 0.0);
history.setup_camera_props(cam4, 4.712385, 25.0, 15.0, 0.0, 0.0, 0.0);
history.set(cam1, "neighbor_right", cam2);
history.set(cam2, "neighbor_right", cam3);
history.set(cam3, "neighbor_right", cam4);
history.set(cam4, "neighbor_right", cam1);
history.set(cam1, "neighbor_left", cam4);
history.set(cam2, "neighbor_left", cam1);
history.set(cam3, "neighbor_left", cam2);
history.set(cam4, "neighbor_left", cam3);

let ts = new_trajectory_system();
let arrow_fsm = new_fsm();
let flight = new_animation_state("flight");
let arrow_request = new_trajectory_request(vec3(-4.0, 1.0, 0.0), vec3(-1.0, 1.0, 0.0));
arrow_request.set_speed_range(10.0, 25.0, 5.0);
arrow_request.set_angle_range(5.0, 85.0, 2.0);
arrow_request.set_gravity(9.81);
arrow_request.set_time_step(0.05);
arrow_request.set_max_steps(100);
arrow_request.set_ground_cutoff(-5.0);
arrow_request.set_collider(ball_collider(0.05));
let arrow_tracks = generate_arrow_tracks(ts, arrow_request, demo_map);
for track in arrow_tracks { flight.add_track(track); }
arrow_fsm.add_state("flight", flight);
history.define_fsm("arrow_fsm", arrow_fsm);
history.define_material("arrow_mat", material(1.0, 1.0, 0.0));
let arrow_id = 20;
history.spawn_entity(arrow_id, "arrow", hex(0, 0), sprite_init_properties());
history.set(arrow_id, "asset", "Arrow");
history.set(arrow_id, "fsm", "arrow_fsm");
history.set(arrow_id, "material", "arrow_mat");
history.set(arrow_id, "z", 0.0);
history.set(arrow_id, "scale", 1.0);
history.set(arrow_id, "rotation_z", 0.0);
history.set(arrow_id, "rotation_y", 0.0);
history.set(arrow_id, "cam_offset_x", 0.0);
history.set(arrow_id, "cam_offset_y", 0.0);
history.set(arrow_id, "cam_offset_z", 0.0);
history.set_animation_state(arrow_id, "flight");
