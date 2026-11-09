// Auto-generated wrapper for bevy_composable_dragging/blend_square
#[path = "../../../../libs/bevy_composable_dragging/examples/common/mod.rs"]
mod common;

use bevy::prelude::*;
use bevy_composable_dragging::ops::{BetweenOptions, Interpolation, between_with};
use bevy_composable_dragging::{DraggablePlugin, View, ViewNode};

use common::screenshot::AutoScreenshotPlugin;
use common::script::ScriptedDragPlugin;

/// Half the edge of the square, in world units.
const HALF: f32 = 2.0;
/// Above one the blend favors the nearest corner.
const SHARPNESS: f32 = 1.5;

/// The corner the knob rests in.
#[derive(Clone, PartialEq)]
enum Corner {
    Coral,
    Gold,
    Mint,
    Sky,
}

const CORNERS: [Corner; 4] = [Corner::Coral, Corner::Gold, Corner::Mint, Corner::Sky];

impl Corner {
    fn position(&self) -> Vec2 {
        match self {
            Corner::Coral => Vec2::new(-HALF, -HALF),
            Corner::Gold => Vec2::new(HALF, -HALF),
            Corner::Mint => Vec2::new(HALF, HALF),
            Corner::Sky => Vec2::new(-HALF, HALF),
        }
    }

    fn color(&self) -> Color {
        match self {
            Corner::Coral => Color::srgb(0.95, 0.4, 0.4),
            Corner::Gold => Color::srgb(0.95, 0.8, 0.3),
            Corner::Mint => Color::srgb(0.35, 0.85, 0.6),
            Corner::Sky => Color::srgb(0.35, 0.6, 0.95),
        }
    }

    fn id(&self) -> &'static str {
        match self {
            Corner::Coral => "coral",
            Corner::Gold => "gold",
            Corner::Mint => "mint",
            Corner::Sky => "sky",
        }
    }
}

fn view(corner: &Corner) -> View<Corner> {
    let mut view = View::new().with(
        ViewNode::rectangle("plate", Vec2::splat(2.0 * HALF + 0.8))
            .with_color(Color::srgb(0.17, 0.19, 0.24)),
    );
    for other in CORNERS {
        view.push(
            ViewNode::circle(format!("mark-{}", other.id()), 0.35)
                .at_xy(other.position())
                .with_z(0.05)
                .with_color(other.color()),
        );
    }
    view.with(
        ViewNode::circle("knob", 1.0)
            .at_xy(corner.position())
            .with_z(0.1)
            .with_color(corner.color())
            .draggable(|| {
                between_with(
                    CORNERS,
                    BetweenOptions::new()
                        .with_interpolation(Interpolation::NaturalNeighbor)
                        .with_sharpness(SHARPNESS),
                )
            }),
    )
}


/// Run this example. Called by the WASM runner.
pub fn run(app: &mut App) {
    app
        .add_plugins(DraggablePlugin::new(Corner::Coral, view))
        .add_plugins(ScriptedDragPlugin::straight(
            Corner::Coral.position(),
            Vec2::new(0.6, 0.9),
        ))
        .add_systems(Startup, |mut commands: Commands| {
            common::spawn_cameras(&mut commands, 7.0);
        })
        ;
}

