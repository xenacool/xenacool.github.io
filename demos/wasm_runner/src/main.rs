//! Shared WASM runner for all Bevy examples (Runtime A — Core).
//! Reads ?example=shapes from URL and dispatches to the matching example.

use bevy::prelude::*;
use bevy::render::RenderPlugin;

// Include auto-generated examples module from build.rs
mod examples {
    include!(concat!(env!("OUT_DIR"), "/examples/mod.rs"));
}

fn get_example_name() -> String {
    let search = web_sys::window()
        .and_then(|w| w.location().search().ok())
        .and_then(|s| s.strip_prefix('?').map(str::to_string))
        .unwrap_or_default();
    search.split('&').find_map(|p| {
        p.split_once('=').and_then(|(k, v)| (k == "example").then_some(v.to_string()))
    }).unwrap_or_else(|| "shapes".to_string())
}

fn main() {
    let example_name = get_example_name();

    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: format!("Bevy Demo: {}", example_name),
                    #[cfg(not(target_arch = "wasm32"))]
                    resolution: (1280, 720).into(),
                    canvas: Some("#bevy-canvas".into()),
                    fit_canvas_to_parent: true,
                    ..default()
                }),
                ..default()
            })
            .set(RenderPlugin {
                synchronous_pipeline_compilation: true,
                ..default()
            }),
    );
    examples::dispatch(&example_name, &mut app);
    app.run();
}
