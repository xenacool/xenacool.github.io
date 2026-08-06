// Note: All of the spriteparts are side profile
use pystral_core::domain::{PainterCommand, SpritePart, Bone, Joint};
use pystral_core::log::{Event, PropertyValue};
use pystral_core::history::HistoryManager;
use hexx::{Hex, ColumnMeshBuilder, HexLayout};
use pystral_compiler::ik::{IkSystem, Rig, Exp, length_eq};
use std::collections::HashMap;
use glam::Vec2;

const WIDTH: f32 = 32.0;

fn color_pair(c: [f32; 4]) -> ([f32; 4], [f32; 4]) {
    let mut mirrored = c;
    mirrored[0] = (c[0] + 0.2).min(1.0);
    mirrored[1] = (c[1] - 0.1).max(0.0);
    (c, mirrored)
}

pub fn create_spider_rig() -> Rig {
    let mut eqs = Vec::new();
    let mut vars = Vec::new();
    let mut targets = Vec::new();

    // Thorax (Base)
    let tx = Exp::var("thorax_x");
    let ty = Exp::var("thorax_y");
    let tz = Exp::var("thorax_z");
    vars.push("thorax".to_string());

    let ttx = Exp::var("target_thorax_x");
    let tty = Exp::var("target_thorax_y");
    let ttz = Exp::var("target_thorax_z");
    targets.push("target_thorax".to_string());
    eqs.push(Exp::sub(tx.clone(), ttx.clone()));
    eqs.push(Exp::sub(ty.clone(), tty.clone()));
    eqs.push(Exp::sub(tz.clone(), ttz.clone()));

    // Abdomen
    let ax = Exp::var("abdomen_x");
    let ay = Exp::var("abdomen_y");
    let az = Exp::var("abdomen_z");
    vars.push("abdomen".to_string());
    eqs.push(length_eq(&tx, &ty, &tz, &ax, &ay, &az, 0.4)); // Distance thorax-abdomen

    // Legs
    for side in &["l", "r"] {
        for i in 1..=4 {
            let prefix = format!("{}_{}", side, i);
            
            let hx = Exp::var(format!("{}_hip_x", prefix));
            let hy = Exp::var(format!("{}_hip_y", prefix));
            let hz = Exp::var(format!("{}_hip_z", prefix));
            vars.push(format!("{}_hip", prefix));
            eqs.push(length_eq(&tx, &ty, &tz, &hx, &hy, &hz, 0.15));

            let kx = Exp::var(format!("{}_knee_x", prefix));
            let ky = Exp::var(format!("{}_knee_y", prefix));
            let kz = Exp::var(format!("{}_knee_z", prefix));
            vars.push(format!("{}_knee", prefix));
            eqs.push(length_eq(&hx, &hy, &hz, &kx, &ky, &kz, 0.3));

            let fx = Exp::var(format!("{}_foot_x", prefix));
            let fy = Exp::var(format!("{}_foot_y", prefix));
            let fz = Exp::var(format!("{}_foot_z", prefix));
            vars.push(format!("{}_foot", prefix));
            
            let tfx = Exp::var(format!("target_{}_foot_x", prefix));
            let tfy = Exp::var(format!("target_{}_foot_y", prefix));
            let tfz = Exp::var(format!("target_{}_foot_z", prefix));
            targets.push(format!("target_{}_foot", prefix));
            
            eqs.push(Exp::sub(fx.clone(), tfx.clone()));
            eqs.push(Exp::sub(fy.clone(), tfy.clone()));
            eqs.push(Exp::sub(fz.clone(), tfz.clone()));
            eqs.push(length_eq(&kx, &ky, &kz, &fx, &fy, &fz, 0.4));
        }
    }

    let mut solver_vars = Vec::new();
    for v in &vars {
        solver_vars.push(format!("{}_x", v));
        solver_vars.push(format!("{}_y", v));
        solver_vars.push(format!("{}_z", v));
    }
    let solver_vars_ref: Vec<&str> = solver_vars.iter().map(|s| s.as_str()).collect();

    let compiled = pystral_compiler::ik::Compiler::compile(&eqs).expect("spider rig compile");
    let solver = pystral_compiler::ik::NewtonRaphsonSolver::new_with_variables(compiled, &solver_vars_ref).expect("expected solver");

    let rig = Rig {
        solver,
        variable_names: vars,
        target_names: targets,
    };

    // Assertion on bounding box fitting in hexx's mesh top
    // We assume the spider is centered at (0,0) in its local space.
    // The max extent of the spider in the XY plane is roughly the body size + leg lengths.
    // Thorax-hip (0.15) + hip-knee (0.3) + knee-foot (0.4) = 0.85.
    let max_radius = 0.85f32;
    
    let layout = HexLayout::default();
    let mesh_info = ColumnMeshBuilder::new(&layout, 1.0).build();
    
    // Get the hex's bounding box in the XY plane
    let mut hex_min = Vec2::splat(f32::INFINITY);
    let mut hex_max = Vec2::splat(f32::NEG_INFINITY);
    for v in &mesh_info.vertices {
        // hexx ColumnMeshBuilder: x, y (height), z
        let pos = Vec2::new(v[0], v[2]);
        hex_min = hex_min.min(pos);
        hex_max = hex_max.max(pos);
    }
    
    let spider_min = Vec2::splat(-max_radius);
    let spider_max = Vec2::splat(max_radius);
    
    assert!(spider_min.x >= hex_min.x && spider_max.x <= hex_max.x, "Spider bounding box X out of hex bounds");
    assert!(spider_min.y >= hex_min.y && spider_max.y <= hex_max.y, "Spider bounding box Y out of hex bounds");

    rig
}

pub fn setup_spider(history: &mut HistoryManager) {
    let mut ik_system = IkSystem::new();
    ik_system.add_rig("spider", create_spider_rig());

    let idle_tracks = crate::demo::animation::generate_ik_tracks(&mut ik_system, "spider", 40, 100.0, |_i, phase| {
        let mut targets = HashMap::new();
        let breathing = (phase.sin() * 0.05) as f32;
        
        targets.insert("target_thorax".to_string(), pystral_compiler::ik::Vec3 { x: 0.0, y: 0.0, z: 0.4 + breathing });

        for side in &["l", "r"] {
            for j in 1..=4 {
                let prefix = format!("{}_{}", side, j);
                let angle = match (side, j) {
                    (&"l", 1) => 150.0f32,
                    (&"l", 2) => 170.0f32,
                    (&"l", 3) => 190.0f32,
                    (&"l", 4) => 210.0f32,
                    (&"r", 1) => 30.0f32,
                    (&"r", 2) => 10.0f32,
                    (&"r", 3) => 350.0f32,
                    (&"r", 4) => 330.0f32,
                    _ => 0.0,
                }.to_radians();
                
                let r = 0.7 + (breathing * 0.2); // Legs move slightly with breathing
                targets.insert(format!("target_{}_foot", prefix), pystral_compiler::ik::Vec3 { 
                    x: angle.cos() * r, 
                    y: angle.sin() * r, 
                    z: 0.0 
                });
            }
        }
        
        // Add thorax movement for breathing
        // The IK system currently doesn't support targets for all variables easily if not defined as targets,
        // but we can influence it by changing initial guesses or adding a thorax target.
        // For now, let's just move the feet targets.
        
        targets
    });

    history.push_and_apply(Event::SpawnEntity {
        id: 1,
        kind: "sprite".to_string(),
        hex: Hex::new(3, -1),
    });

    let mut initial_props = vec![
        ("scale".to_string(), PropertyValue::Float(1.5)),
        ("z".to_string(), PropertyValue::Float(1.0)),
        ("material".to_string(), PropertyValue::String("rock".to_string())),
    ];

    let mut joints = vec!["thorax".to_string(), "abdomen".to_string()];
    for side in &["l", "r"] {
        for i in 1..=4 {
            let prefix = format!("{}_{}", side, i);
            joints.push(format!("{}_hip", prefix));
            joints.push(format!("{}_knee", prefix));
            joints.push(format!("{}_foot", prefix));
        }
    }
    
    for j in joints {
        initial_props.push((format!("{}_x", j), PropertyValue::Float(0.0)));
        initial_props.push((format!("{}_y", j), PropertyValue::Float(0.0)));
        initial_props.push((format!("{}_z", j), PropertyValue::Float(0.0)));
    }

    let mut sprite_parts = vec![
        SpritePart { x_prop: "thorax_x".into(), y_prop: "thorax_y".into(), z_prop: "thorax_z".into(), rotation_prop: None, color: [1.0, 1.0, 1.0], scale: 1.0, painter_commands: Vec::new(), spritestack: Some(("primitives".into(), "SpiderThorax".into())) },
        SpritePart { x_prop: "abdomen_x".into(), y_prop: "abdomen_y".into(), z_prop: "abdomen_z".into(), rotation_prop: None, color: [1.0, 1.0, 1.0], scale: 1.0, painter_commands: Vec::new(), spritestack: Some(("primitives".into(), "SpiderAbdomen".into())) },
    ];
    
    for side in &["l", "r"] {
        for i in 1..=4 {
            let prefix = format!("{}_{}", side, i);
            sprite_parts.push(SpritePart { x_prop: format!("{}_hip_x", prefix), y_prop: format!("{}_hip_y", prefix), z_prop: format!("{}_hip_z", prefix), rotation_prop: None, color: [1.0, 1.0, 1.0], scale: 1.0, painter_commands: Vec::new(), spritestack: Some(("primitives".into(), "SpiderJoint".into())) });
            sprite_parts.push(SpritePart { x_prop: format!("{}_knee_x", prefix), y_prop: format!("{}_knee_y", prefix), z_prop: format!("{}_knee_z", prefix), rotation_prop: None, color: [1.0, 1.0, 1.0], scale: 1.0, painter_commands: Vec::new(), spritestack: Some(("primitives".into(), "SpiderJoint".into())) });
            sprite_parts.push(SpritePart { x_prop: format!("{}_foot_x", prefix), y_prop: format!("{}_foot_y", prefix), z_prop: format!("{}_foot_z", prefix), rotation_prop: None, color: [1.0, 1.0, 1.0], scale: 1.0, painter_commands: Vec::new(), spritestack: Some(("primitives".into(), "SpiderJoint".into())) });
        }
    }
    
    initial_props.push(("sprite_parts".to_string(), PropertyValue::SpriteParts(sprite_parts)));

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
        name: "spider_fsm".to_string(),
        definition: pystral_core::animation::InactiveFSMDefinition {
            states,
        },
    });

    history.push_and_apply(Event::UpdateProperty {
        id: 1,
        property: "fsm".to_string(),
        value: PropertyValue::String("spider_fsm".to_string()),
    });

    history.push_and_apply(Event::SetAnimationState {
        id: 1,
        state: "idle".to_string(),
    });

    let mut bones = vec![
        Bone { start: Joint::Property("thorax".to_string()), end: Joint::Property("abdomen".to_string()), painter_commands: make_bone_commands(WIDTH) },
    ];
    for side in &["l", "r"] {
        for i in 1..=4 {
            let prefix = format!("{}_{}", side, i);
            bones.push(Bone { start: Joint::Property("thorax".to_string()), end: Joint::Property(format!("{}_hip", prefix)), painter_commands: make_bone_commands(WIDTH) });
            bones.push(Bone { start: Joint::Property(format!("{}_hip", prefix)), end: Joint::Property(format!("{}_knee", prefix)), painter_commands: make_bone_commands(WIDTH) });
            bones.push(Bone { start: Joint::Property(format!("{}_knee", prefix)), end: Joint::Property(format!("{}_foot", prefix)), painter_commands: make_bone_commands(WIDTH) });
        }
    }

    history.push_and_apply(Event::UpdateProperty {
        id: 1,
        property: "skeleton".to_string(),
        value: PropertyValue::Skeleton(pystral_core::domain::Skeleton { bones }),
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
