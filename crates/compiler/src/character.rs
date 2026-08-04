// Note: All of the spriteparts are side profile
use pystral_core::domain::{PainterCommand, SpritePart, Bone, Joint};
use pystral_core::log::{Event, PropertyValue};
use pystral_core::history::HistoryManager;
use hexx::Hex;
use crate::demo::animation::generate_ik_tracks;
use crate::ik::{IkSystem, Rig, Exp, length_eq};
use std::collections::HashMap;

fn color_pair(c: [f32; 4]) -> ([f32; 4], [f32; 4]) {
    let mut mirrored = c;
    mirrored[0] = (c[0] + 0.2).min(1.0);
    mirrored[1] = (c[1] - 0.1).max(0.0);
    (c, mirrored)
}

pub fn create_rat_rig() -> Rig {
    let mut eqs = Vec::new();
    let mut vars = Vec::new();
    let mut targets = Vec::new();

    // Chest
    let chx = Exp::var("chest_x");
    let chy = Exp::var("chest_y");
    let chz = Exp::var("chest_z");
    vars.push("chest".to_string());

    // Neck and Head
    let nx = Exp::var("neck_x");
    let ny = Exp::var("neck_y");
    let nz = Exp::var("neck_z");
    vars.push("neck".to_string());
    eqs.push(length_eq(&chx, &chy, &chz, &nx, &ny, &nz, 0.1));

    let hx = Exp::var("head_x");
    let hy = Exp::var("head_y");
    let hz = Exp::var("head_z");
    let thx = Exp::var("target_head_x");
    let thy = Exp::var("target_head_y");
    let thz = Exp::var("target_head_z");
    vars.push("head".to_string());
    targets.push("target_head".to_string());
    eqs.push(Exp::sub(hx.clone(), thx.clone()));
    eqs.push(Exp::sub(hy.clone(), thy.clone()));
    eqs.push(Exp::sub(hz.clone(), thz.clone()));
    eqs.push(length_eq(&hx, &hy, &hz, &nx, &ny, &nz, 0.15));

    // Spine
    let s1x = Exp::var("spine_1_x");
    let s1y = Exp::var("spine_1_y");
    let s1z = Exp::var("spine_1_z");
    vars.push("spine_1".to_string());
    eqs.push(length_eq(&chx, &chy, &chz, &s1x, &s1y, &s1z, 0.6));

    let hpx = Exp::var("pelvis_x");
    let hpy = Exp::var("pelvis_y");
    let hpz = Exp::var("pelvis_z");
    vars.push("pelvis".to_string());
    eqs.push(length_eq(&s1x, &s1y, &s1z, &hpx, &hpy, &hpz, 0.6));

    // Tail
    let t1x = Exp::var("tail_1_x");
    let t1y = Exp::var("tail_1_y");
    let t1z = Exp::var("tail_1_z");
    vars.push("tail_1".to_string());
    eqs.push(length_eq(&hpx, &hpy, &hpz, &t1x, &t1y, &t1z, 0.3));

    let t2x = Exp::var("tail_2_x");
    let t2y = Exp::var("tail_2_y");
    let t2z = Exp::var("tail_2_z");
    vars.push("tail_2".to_string());
    eqs.push(length_eq(&t1x, &t1y, &t1z, &t2x, &t2y, &t2z, 0.3));

    let t3x = Exp::var("tail_3_x");
    let t3y = Exp::var("tail_3_y");
    let t3z = Exp::var("tail_3_z");
    vars.push("tail_3".to_string());
    eqs.push(length_eq(&t2x, &t2y, &t2z, &t3x, &t3y, &t3z, 0.3));

    let t4x = Exp::var("tail_4_x");
    let t4y = Exp::var("tail_4_y");
    let t4z = Exp::var("tail_4_z");
    let ttx = Exp::var("target_tail_x");
    let tty = Exp::var("target_tail_y");
    let ttz = Exp::var("target_tail_z");
    vars.push("tail_4".to_string());
    targets.push("target_tail".to_string());
    eqs.push(Exp::sub(t4x.clone(), ttx.clone()));
    eqs.push(Exp::sub(t4y.clone(), tty.clone()));
    eqs.push(Exp::sub(t4z.clone(), ttz.clone()));
    eqs.push(length_eq(&t3x, &t3y, &t3z, &t4x, &t4y, &t4z, 0.3));

    // Front Limbs (Short)
    let lsx = Exp::var("l_shoulder_x");
    let lsy = Exp::var("l_shoulder_y");
    let lsz = Exp::var("l_shoulder_z");
    vars.push("l_shoulder".to_string());
    eqs.push(length_eq(&chx, &chy, &chz, &lsx, &lsy, &lsz, 0.1));

    let lhax = Exp::var("l_hand_x");
    let lhay = Exp::var("l_hand_y");
    let lhaz = Exp::var("l_hand_z");
    let tlhx = Exp::var("target_l_hand_x");
    let tlhy = Exp::var("target_l_hand_y");
    let tlhz = Exp::var("target_l_hand_z");
    vars.push("l_hand".to_string());
    targets.push("target_l_hand".to_string());
    eqs.push(Exp::sub(lhax.clone(), tlhx.clone()));
    eqs.push(Exp::sub(lhay.clone(), tlhy.clone()));
    eqs.push(Exp::sub(lhaz.clone(), tlhz.clone()));
    eqs.push(length_eq(&lsx, &lsy, &lsz, &lhax, &lhay, &lhaz, 0.15));

    let rsx = Exp::var("r_shoulder_x");
    let rsy = Exp::var("r_shoulder_y");
    let rsz = Exp::var("r_shoulder_z");
    vars.push("r_shoulder".to_string());
    eqs.push(length_eq(&chx, &chy, &chz, &rsx, &rsy, &rsz, 0.1));

    let rhax = Exp::var("r_hand_x");
    let rhay = Exp::var("r_hand_y");
    let rhaz = Exp::var("r_hand_z");
    let trhx = Exp::var("target_r_hand_x");
    let trhy = Exp::var("target_r_hand_y");
    let trhz = Exp::var("target_r_hand_z");
    vars.push("r_hand".to_string());
    targets.push("target_r_hand".to_string());
    eqs.push(Exp::sub(rhax.clone(), trhx.clone()));
    eqs.push(Exp::sub(rhay.clone(), trhy.clone()));
    eqs.push(Exp::sub(rhaz.clone(), trhz.clone()));
    eqs.push(length_eq(&rsx, &rsy, &rsz, &rhax, &rhay, &rhaz, 0.15));

    // Back Limbs (Short)
    let lhx = Exp::var("l_hip_x");
    let lhy = Exp::var("l_hip_y");
    let lhz = Exp::var("l_hip_z");
    vars.push("l_hip".to_string());
    eqs.push(length_eq(&hpx, &hpy, &hpz, &lhx, &lhy, &lhz, 0.1));

    let lfx = Exp::var("l_foot_x");
    let lfy = Exp::var("l_foot_y");
    let lfz = Exp::var("l_foot_z");
    let tlfx = Exp::var("target_l_foot_x");
    let tlfy = Exp::var("target_l_foot_y");
    let tlfz = Exp::var("target_l_foot_z");
    vars.push("l_foot".to_string());
    targets.push("target_l_foot".to_string());
    eqs.push(Exp::sub(lfx.clone(), tlfx.clone()));
    eqs.push(Exp::sub(lfy.clone(), tlfy.clone()));
    eqs.push(Exp::sub(lfz.clone(), tlfz.clone()));
    eqs.push(length_eq(&lhx, &lhy, &lhz, &lfx, &lfy, &lfz, 0.15));

    let rhx = Exp::var("r_hip_x");
    let rhy = Exp::var("r_hip_y");
    let rhz = Exp::var("r_hip_z");
    vars.push("r_hip".to_string());
    eqs.push(length_eq(&hpx, &hpy, &hpz, &rhx, &rhy, &rhz, 0.1));

    let rfx = Exp::var("r_foot_x");
    let rfy = Exp::var("r_foot_y");
    let rfz = Exp::var("r_foot_z");
    let trfx = Exp::var("target_r_foot_x");
    let trfy = Exp::var("target_r_foot_y");
    let trfz = Exp::var("target_r_foot_z");
    vars.push("r_foot".to_string());
    targets.push("target_r_foot".to_string());
    eqs.push(Exp::sub(rfx.clone(), trfx.clone()));
    eqs.push(Exp::sub(rfy.clone(), trfy.clone()));
    eqs.push(Exp::sub(rfz.clone(), trfz.clone()));
    eqs.push(length_eq(&rhx, &rhy, &rhz, &rfx, &rfy, &rfz, 0.15));

    let mut solver_vars = Vec::new();
    for v in &vars {
        solver_vars.push(format!("{}_x", v));
        solver_vars.push(format!("{}_y", v));
        solver_vars.push(format!("{}_z", v));
    }
    let solver_vars_ref: Vec<&str> = solver_vars.iter().map(|s| s.as_str()).collect();

    let compiled = crate::ik::Compiler::compile(&eqs).expect("rig compile");
    let solver = crate::ik::NewtonRaphsonSolver::new_with_variables(compiled, &solver_vars_ref).expect("expected solver");

    Rig {
        solver,
        variable_names: vars,
        target_names: targets,
    }
}

pub fn setup_character(history: &mut HistoryManager) {
    let mut ik_system = IkSystem::new();
    ik_system.add_rig("knight", create_rat_rig()); // Reusing rat rig for now as it matches humanoid enough for this demo

    let idle_tracks = generate_ik_tracks(&mut ik_system, 20, 100.0, |i, phase| {
        let mut targets = HashMap::new();
        if i == 0 {
            targets.insert("target_head".to_string(), crate::ik::Vec3 { x: 1.2, y: 0.0, z: 0.3 });
            targets.insert("target_tail".to_string(), crate::ik::Vec3 { x: -1.5, y: 0.0, z: 0.2 });
            targets.insert("target_l_hand".to_string(), crate::ik::Vec3 { x: 0.8, y: -0.2, z: 0.0 });
            targets.insert("target_r_hand".to_string(), crate::ik::Vec3 { x: 0.9, y: 0.2, z: 0.0 });
            targets.insert("target_l_foot".to_string(), crate::ik::Vec3 { x: -0.6, y: -0.2, z: 0.0 });
            targets.insert("target_r_foot".to_string(), crate::ik::Vec3 { x: -0.5, y: 0.2, z: 0.0 });
        } else {
            targets.insert("target_head".to_string(), crate::ik::Vec3 { x: 1.2 + (phase.cos() * 0.05) as f32, y: (phase.sin() * 0.05) as f32, z: 0.3 + (phase.sin() * 0.05) as f32 });
            targets.insert("target_tail".to_string(), crate::ik::Vec3 { x: -1.5, y: (phase.cos() * 0.1) as f32, z: 0.2 + (phase.sin() * 0.15) as f32 });
            targets.insert("target_l_hand".to_string(), crate::ik::Vec3 { x: 0.8 + (phase.cos() * 0.02) as f32, y: -0.2, z: (phase.sin().abs() * 0.05) as f32 });
            targets.insert("target_r_hand".to_string(), crate::ik::Vec3 { x: 0.9 + (phase.sin() * 0.02) as f32, y: 0.2, z: (phase.cos().abs() * 0.05) as f32 });
            targets.insert("target_l_foot".to_string(), crate::ik::Vec3 { x: -0.6 + (phase.sin() * 0.02) as f32, y: -0.2, z: (phase.cos().abs() * 0.02) as f32 });
            targets.insert("target_r_foot".to_string(), crate::ik::Vec3 { x: -0.5 + (phase.cos() * 0.02) as f32, y: 0.2, z: (phase.sin().abs() * 0.02) as f32 });
        }
        targets
    });

    history.push_and_apply(Event::SpawnEntity {
        id: 1,
        kind: "sprite".to_string(),
        hex: Hex::new(3, -2),
    });

    let _rat_color = [0.4, 0.4, 0.4, 1.0];
    let _belly_color = [0.6, 0.6, 0.6, 1.0];
    let _tail_color = [0.8, 0.6, 0.6, 1.0];
    let _joint_color = [0.3, 0.3, 0.3, 1.0];

    let mut initial_props = vec![
        ("scale".to_string(), PropertyValue::Float(2.0)),
        ("z".to_string(), PropertyValue::Float(1.0)),
        ("rotation_z".to_string(), PropertyValue::Float(0.0)),
        ("cam_offset_x".to_string(), PropertyValue::Float(0.0)),
        ("cam_offset_y".to_string(), PropertyValue::Float(0.0)),
        ("cam_offset_z".to_string(), PropertyValue::Float(0.0)),
        ("material".to_string(), PropertyValue::String("rock".to_string())),
        ("sprite_parts".to_string(), PropertyValue::SpriteParts(vec![
            SpritePart { x_prop: "l_foot_x".into(), y_prop: "l_foot_y".into(), z_prop: "l_foot_z".into(), rotation_prop: None, color: [1.0, 1.0, 1.0], scale: 1.0, painter_commands: Vec::new(), spritestack: Some(("primitives".into(), "CubeGray".into())) },
            SpritePart { x_prop: "r_foot_x".into(), y_prop: "r_foot_y".into(), z_prop: "r_foot_z".into(), rotation_prop: None, color: [1.0, 1.0, 1.0], scale: 1.0, painter_commands: Vec::new(), spritestack: Some(("primitives".into(), "CubeGray".into())) },
            SpritePart { x_prop: "l_hand_x".into(), y_prop: "l_hand_y".into(), z_prop: "l_hand_z".into(), rotation_prop: None, color: [1.0, 1.0, 1.0], scale: 1.0, painter_commands: Vec::new(), spritestack: Some(("primitives".into(), "CubeRed".into())) },
            SpritePart { x_prop: "r_hand_x".into(), y_prop: "r_hand_y".into(), z_prop: "r_hand_z".into(), rotation_prop: None, color: [1.0, 1.0, 1.0], scale: 1.0, painter_commands: Vec::new(), spritestack: Some(("primitives".into(), "CubeRed".into())) },
            SpritePart { x_prop: "pelvis_x".into(), y_prop: "pelvis_y".into(), z_prop: "pelvis_z".into(), rotation_prop: None, color: [1.0, 1.0, 1.0], scale: 1.0, painter_commands: Vec::new(), spritestack: Some(("primitives".into(), "CubeBlue".into())) },
            SpritePart { x_prop: "head_x".into(), y_prop: "head_y".into(), z_prop: "head_z".into(), rotation_prop: None, color: [1.0, 1.0, 1.0], scale: 1.0, painter_commands: Vec::new(), spritestack: Some(("primitives".into(), "CubeGreen".into())) },
        ])),
    ];

    // Initialize all joint properties to avoid "not found" errors before FSM kicks in
    let joints = vec![
        "chest", "neck", "head", "spine_1", "pelvis",
        "tail_1", "tail_2", "tail_3", "tail_4",
        "l_shoulder", "r_shoulder", "l_hand", "r_hand",
        "l_hip", "r_hip", "l_foot", "r_foot"
    ];
    for j in joints {
        initial_props.push((format!("{}_x", j), PropertyValue::Float(0.0)));
        initial_props.push((format!("{}_y", j), PropertyValue::Float(0.0)));
        initial_props.push((format!("{}_z", j), PropertyValue::Float(0.0)));
    }

    for (prop, val) in initial_props {
        history.push_and_apply(Event::UpdateProperty {
            id: 1,
            property: prop,
            value: val,
        });
    }

    let mut states = HashMap::new();
    let mut idle_state_tracks = Vec::new();
    for (prop, keyframes) in idle_tracks {
        idle_state_tracks.push(pystral_core::animation::PropertyTrack {
            property: prop,
            keyframes,
            loop_behavior: pystral_core::animation::LoopBehavior::Loop,
        });
    }
    states.insert("idle".to_string(), pystral_core::animation::AnimationState { name: "idle".to_string(), tracks: idle_state_tracks });

    history.push_and_apply(Event::DefineFSM {
        name: "character_fsm".to_string(),
        definition: pystral_core::animation::InactiveFSMDefinition {
            states,
        },
    });

    history.push_and_apply(Event::UpdateProperty {
        id: 1,
        property: "fsm".to_string(),
        value: PropertyValue::String("character_fsm".to_string()),
    });

    history.push_and_apply(Event::SetAnimationState {
        id: 1,
        state: "idle".to_string(),
    });

    history.push_and_apply(Event::UpdateProperty {
        id: 1,
        property: "skeleton".to_string(),
        value: PropertyValue::Skeleton(pystral_core::domain::Skeleton {
            bones: vec![
                Bone { start: Joint::Property("neck".to_string()), end: Joint::Property("head".to_string()), painter_commands: make_bone_commands(12.0) },
                Bone { start: Joint::Property("chest".to_string()), end: Joint::Property("neck".to_string()), painter_commands: make_bone_commands(15.0) },
                Bone { start: Joint::Property("chest".to_string()), end: Joint::Property("spine_1".to_string()), painter_commands: make_bone_commands(20.0) },
                Bone { start: Joint::Property("spine_1".to_string()), end: Joint::Property("pelvis".to_string()), painter_commands: make_bone_commands(20.0) },
                Bone { start: Joint::Property("pelvis".to_string()), end: Joint::Property("tail_1".to_string()), painter_commands: make_bone_commands(10.0) },
                Bone { start: Joint::Property("tail_1".to_string()), end: Joint::Property("tail_2".to_string()), painter_commands: make_bone_commands(8.0) },
                Bone { start: Joint::Property("tail_2".to_string()), end: Joint::Property("tail_3".to_string()), painter_commands: make_bone_commands(6.0) },
                Bone { start: Joint::Property("tail_3".to_string()), end: Joint::Property("tail_4".to_string()), painter_commands: make_bone_commands(4.0) },
                Bone { start: Joint::Property("chest".to_string()), end: Joint::Property("l_shoulder".to_string()), painter_commands: make_bone_commands(10.0) },
                Bone { start: Joint::Property("l_shoulder".to_string()), end: Joint::Property("l_hand".to_string()), painter_commands: make_bone_commands(8.0) },
                Bone { start: Joint::Property("chest".to_string()), end: Joint::Property("r_shoulder".to_string()), painter_commands: make_bone_commands(10.0) },
                Bone { start: Joint::Property("r_shoulder".to_string()), end: Joint::Property("r_hand".to_string()), painter_commands: make_bone_commands(8.0) },
                Bone { start: Joint::Property("pelvis".to_string()), end: Joint::Property("l_hip".to_string()), painter_commands: make_bone_commands(12.0) },
                Bone { start: Joint::Property("l_hip".to_string()), end: Joint::Property("l_foot".to_string()), painter_commands: make_bone_commands(10.0) },
                Bone { start: Joint::Property("pelvis".to_string()), end: Joint::Property("r_hip".to_string()), painter_commands: make_bone_commands(12.0) },
                Bone { start: Joint::Property("r_hip".to_string()), end: Joint::Property("r_foot".to_string()), painter_commands: make_bone_commands(10.0) },
            ]
        }),
    });
}

pub fn make_arrow_commands(color: [f32; 4]) -> Vec<PainterCommand> {
    let cp = color_pair(color);
    vec![
        PainterCommand::SetColor(cp.0, cp.1),
        PainterCommand::SetStrokeWidth(6.0),
        // Shaft
        PainterCommand::MoveTo(40.0, 128.0, 0.0),
        PainterCommand::LineTo(200.0, 128.0, 0.0),
        PainterCommand::Stroke,
        // Head
        PainterCommand::MoveTo(200.0, 100.0, 0.0),
        PainterCommand::LineTo(240.0, 128.0, 0.0),
        PainterCommand::LineTo(200.0, 156.0, 0.0),
        PainterCommand::Close,
        PainterCommand::Fill,
        // Tail
        PainterCommand::MoveTo(40.0, 100.0, 0.0),
        PainterCommand::LineTo(40.0, 156.0, 0.0),
        PainterCommand::Stroke,
    ]
}

pub fn make_rat_head_commands(color: [f32; 4]) -> Vec<PainterCommand> {
    let cp = color_pair(color);
    vec![
        PainterCommand::SetColor(cp.0, cp.1),
        PainterCommand::SetStrokeWidth(4.0),
        // Main head shape
        PainterCommand::MoveTo(220.0, 128.0, 0.0), // Nose
        PainterCommand::QuadTo(160.0, 80.0, 0.0, 80.0, 100.0, 0.0), // Forehead to back
        PainterCommand::QuadTo(60.0, 160.0, 0.0, 120.0, 180.0, 0.0), // Back to jaw
        PainterCommand::LineTo(220.0, 128.0, 0.0), // Jaw to nose
        PainterCommand::Fill,
        PainterCommand::Stroke,
        // Ear
        PainterCommand::MoveTo(110.0, 95.0, 5.0),
        PainterCommand::QuadTo(90.0, 40.0, 5.0, 70.0, 90.0, 5.0),
        PainterCommand::Stroke,
        // Eye (black dot)
        PainterCommand::SetColor([0.0, 0.0, 0.0, 1.0], [0.1, 0.1, 0.1, 1.0]),
        PainterCommand::MoveTo(175.0, 120.0, 2.0),
        PainterCommand::LineTo(176.0, 120.0, 2.0),
        PainterCommand::SetStrokeWidth(6.0),
        PainterCommand::Stroke,
    ]
}

pub fn make_rat_chest_commands(color: [f32; 4]) -> Vec<PainterCommand> {
    let cp = color_pair(color);
    vec![
        PainterCommand::SetColor(cp.0, cp.1),
        PainterCommand::SetStrokeWidth(6.0),
        PainterCommand::MoveTo(60.0, 128.0, 0.0),
        PainterCommand::QuadTo(128.0, 60.0, 0.0, 200.0, 100.0, 0.0), // Back
        PainterCommand::QuadTo(220.0, 160.0, 0.0, 128.0, 190.0, 0.0), // Belly
        PainterCommand::QuadTo(40.0, 160.0, 0.0, 60.0, 128.0, 0.0),
        PainterCommand::Fill,
        PainterCommand::Stroke,
    ]
}

pub fn make_rat_spine_commands(color: [f32; 4]) -> Vec<PainterCommand> {
    let cp = color_pair(color);
    vec![
        PainterCommand::SetColor(cp.0, cp.1),
        PainterCommand::SetStrokeWidth(8.0),
        PainterCommand::MoveTo(40.0, 160.0, 0.0),
        PainterCommand::QuadTo(128.0, 80.0, 0.0, 216.0, 160.0, 0.0),
        PainterCommand::Fill,
        PainterCommand::Stroke,
    ]
}

pub fn make_rat_pelvis_commands(color: [f32; 4]) -> Vec<PainterCommand> {
    let cp = color_pair(color);
    vec![
        PainterCommand::SetColor(cp.0, cp.1),
        PainterCommand::SetStrokeWidth(6.0),
        PainterCommand::MoveTo(40.0, 140.0, 0.0),
        PainterCommand::QuadTo(128.0, 60.0, 0.0, 216.0, 140.0, 0.0),
        PainterCommand::QuadTo(200.0, 220.0, 0.0, 128.0, 200.0, 0.0),
        PainterCommand::QuadTo(56.0, 220.0, 0.0, 40.0, 140.0, 0.0),
        PainterCommand::Fill,
        PainterCommand::Stroke,
    ]
}

pub fn make_rat_foot_commands(color: [f32; 4]) -> Vec<PainterCommand> {
    let cp = color_pair(color);
    vec![
        PainterCommand::SetColor(cp.0, cp.1),
        PainterCommand::SetStrokeWidth(4.0),
        PainterCommand::MoveTo(60.0, 160.0, 0.0),
        PainterCommand::LineTo(200.0, 160.0, 0.0), // Sole
        PainterCommand::LineTo(210.0, 140.0, 0.0), // Toe 1
        PainterCommand::MoveTo(200.0, 160.0, 0.0),
        PainterCommand::LineTo(205.0, 145.0, 0.0), // Toe 2
        PainterCommand::Stroke,
    ]
}

pub fn make_triangle_commands(color: [f32; 4]) -> Vec<PainterCommand> {
    let cp = color_pair(color);
    vec![
        PainterCommand::SetColor(cp.0, cp.1),
        PainterCommand::MoveTo(128.0, 64.0, 0.0),
        PainterCommand::LineTo(220.0, 160.0, 0.0),
        PainterCommand::LineTo(64.0, 160.0, 0.0),
        PainterCommand::Close,
        PainterCommand::Fill,
    ]
}

pub fn make_circle_commands(color: [f32; 4]) -> Vec<PainterCommand> {
    let cp = color_pair(color);
    vec![
        PainterCommand::SetColor(cp.0, cp.1),
        PainterCommand::MoveTo(176.0, 128.0, 0.0),
        PainterCommand::QuadTo(176.0, 176.0, 0.0, 128.0, 176.0, 0.0),
        PainterCommand::QuadTo(80.0, 176.0, 0.0, 80.0, 128.0, 0.0),
        PainterCommand::QuadTo(80.0, 80.0, 0.0, 128.0, 80.0, 0.0),
        PainterCommand::QuadTo(176.0, 80.0, 0.0, 176.0, 128.0, 0.0),
        PainterCommand::Fill,
    ]
}

pub fn make_rect_commands(color: [f32; 4], x: f32, y: f32, w: f32, h: f32) -> Vec<PainterCommand> {
    let cp = color_pair(color);
    vec![
        PainterCommand::SetColor(cp.0, cp.1),
        PainterCommand::MoveTo(x, y, 0.0),
        PainterCommand::LineTo(x + w, y, 0.0),
        PainterCommand::LineTo(x + w, y + h, 0.0),
        PainterCommand::LineTo(x, y + h, 0.0),
        PainterCommand::Close,
        PainterCommand::Fill,
    ]
}

pub fn make_bone_commands(width: f32) -> Vec<PainterCommand> {
    let cp = color_pair([0.8, 0.8, 0.8, 1.0]);
    vec![
        PainterCommand::SetColor(cp.0, cp.1),
        PainterCommand::SetStrokeWidth(width),
        PainterCommand::MoveTo(128.0, 0.0, 0.0),
        PainterCommand::LineTo(128.0, 256.0, 0.0),
        PainterCommand::Stroke,
    ]
}
