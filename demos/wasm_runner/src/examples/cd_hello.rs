// Auto-generated wrapper for bevy_composable_dragging/hello
#[path = "../../../../libs/bevy_composable_dragging/examples/common/mod.rs"]
mod common;

use bevy::prelude::*;

use common::screenshot::AutoScreenshotPlugin;


/// Run this example. Called by the WASM runner.
pub fn run(app: &mut App) {
    app
        .add_systems(Startup, setup)
        ;
}


fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    common::spawn_cameras(&mut commands, 6.0);

    let track = meshes.add(Rectangle::new(4.0, 0.3));
    let knob = meshes.add(Circle::new(0.5));
    let track_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.25, 0.27, 0.32),
        unlit: true,
        ..default()
    });
    let knob_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.6, 0.2),
        unlit: true,
        ..default()
    });

    commands.spawn((
        Mesh3d(track),
        MeshMaterial3d(track_material),
        Transform::default(),
    ));
    commands.spawn((
        Mesh3d(knob),
        MeshMaterial3d(knob_material),
        Transform::from_xyz(-2.0, 0.0, 0.1),
    ));
}
