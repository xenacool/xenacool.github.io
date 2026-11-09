//! Single WASM runner for all Bevy examples.
//! Reads ?example=shapes from URL and dispatches to the matching example.

use bevy::prelude::*;
use bevy::render::RenderPlugin;

mod examples;

fn get_example_name() -> String {
    web_sys::window()
        .and_then(|w| w.location().ok())
        .and_then(|l| l.search().ok())
        .and_then(|s| s.strip_prefix('?'))
        .and_then(|params| {
            params.split('&').find_map(|p| {
                p.split_once('=').and_then(|(k, v)| (k == "example").then_some(v))
            })
        })
        .unwrap_or("shapes")
        .to_string()
}

fn main() {
    let example_name = get_example_name();
    
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: format!("Bevy Demo: {}", example_name),
                        resolution: (1280.0, 720.0).into(),
                        canvas: Some("#bevy-canvas".into()),
                        ..default()
                    }),
                    ..default()
                })
                .set(RenderPlugin {
                    synchronous_pipeline_compilation: true,
                    ..default()
                }),
        )
        .add_plugins(bevy_gradmesh::GradMeshPlugin)
        .add_systems(Startup, move |mut app: App| {
            examples::dispatch(&example_name, &mut app);
        })
        .run();
}
