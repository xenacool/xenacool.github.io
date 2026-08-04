// Shared presentation property builders used by the scenario modules.
history.define_material("rock", material(0.5, 0.5, 0.5));
history.define_material("grass", material(0.2, 0.6, 0.2));
history.define_material("dirt", material(0.4, 0.3, 0.2));

fn sprite_init_properties() {
    let properties = [
        "scale", "z", "layer", "rotation_x", "rotation_y", "rotation_z",
        "cam_offset_x", "cam_offset_y", "cam_offset_z",
        "x_offset", "y_offset", "z_offset", "material", "spritestack"
    ];
    properties
}

fn character_init_properties() {
    let properties = sprite_init_properties();
    let joints = [
        "chest", "neck", "head", "l_shoulder", "r_shoulder", "pelvis",
        "l_hip", "r_hip", "l_elbow", "l_hand", "r_elbow", "r_hand",
        "l_knee", "l_foot", "r_knee", "r_foot", "spine_1", "tail_1",
        "tail_2", "tail_3", "tail_4"
    ];
    for joint in joints {
        properties.push(joint + "_x");
        properties.push(joint + "_y");
        properties.push(joint + "_z");
    }
    properties
}

fn camera_init_properties() {
    ["angle", "distance", "height", "target_x", "target_y", "target_z"]
}
