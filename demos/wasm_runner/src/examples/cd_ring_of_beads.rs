// Auto-generated wrapper for bevy_composable_dragging/ring_of_beads
#[path = "../../../../libs/bevy_composable_dragging/examples/common/mod.rs"]
mod common;

use std::f32::consts::{FRAC_PI_2, TAU};

use bevy::prelude::*;
use bevy_composable_dragging::ops::closest_states;
use bevy_composable_dragging::{DraggablePlugin, View, ViewNode};

use common::move_item;
use common::screenshot::AutoScreenshotPlugin;
use common::script::ScriptedDragPlugin;

const BEADS: usize = 8;
const RADIUS: f32 = 2.0;

/// The beads around the ring, clockwise from the top.
#[derive(Clone)]
struct Ring(Vec<usize>);

fn slot_position(slot: usize) -> Vec2 {
    Vec2::from_angle(FRAC_PI_2 - slot as f32 * TAU / BEADS as f32) * RADIUS
}

fn view(ring: &Ring) -> View<Ring> {
    let mut view = View::new();
    for slot in 0..BEADS {
        view.push(
            ViewNode::circle(format!("slot-{slot}"), 0.2)
                .at_xy(slot_position(slot))
                .with_color(Color::srgb(0.35, 0.37, 0.42)),
        );
    }
    for (slot, &bead) in ring.0.iter().enumerate() {
        let order = ring.0.clone();
        let hue = bead as f32 * 360.0 / BEADS as f32;
        view.push(
            ViewNode::circle(format!("bead-{bead}"), 0.8)
                .at_xy(slot_position(slot))
                .with_z(0.1)
                .with_color(Color::hsl(hue, 0.7, 0.55))
                .draggable(move || {
                    closest_states((0..BEADS).map(|to| Ring(move_item(&order, slot, to))))
                        .with_floating()
                }),
        );
    }
    view
}


/// Run this example. Called by the WASM runner.
pub fn run(app: &mut App) {
    app
        .add_plugins(DraggablePlugin::new(Ring((0..BEADS).collect()), view))
        .add_plugins(ScriptedDragPlugin::straight(
            slot_position(0),
            slot_position(3) * 1.25,
        ))
        .add_systems(Startup, |mut commands: Commands| {
            common::spawn_cameras(&mut commands, 6.0);
        })
        ;
}

