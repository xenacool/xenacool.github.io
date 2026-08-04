// Shared presentation property builders used by the scenario modules.
history.define_material("rock", material(0.5, 0.5, 0.5));
history.define_material("grass", material(0.2, 0.6, 0.2));
history.define_material("dirt", material(0.4, 0.3, 0.2));

fn sprite_init_properties() {
    let properties = [
        "scale", "layer", "rotation_x", "rotation_y", "rotation_z", "facing",
        "cam_offset_x", "cam_offset_y", "cam_offset_z",
        "x_offset", "y_offset", "z_offset", "slice_index", "material", "spritestack"
    ];
    properties
}

fn character_init_properties() {
    [
        "scale", "layer", "rotation_x", "rotation_y", "rotation_z", "facing",
        "cam_offset_x", "cam_offset_y", "cam_offset_z",
        "x_offset", "y_offset", "z_offset", "slice_index", "material", "spritestack"
    ]
}

fn camera_init_properties() {
    ["angle", "distance", "height", "target_x", "target_y", "target_z"]
}
