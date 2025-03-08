use tempo_trainer::GamePlugin;

use bevy::{prelude::*, window::PresentMode};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "tempo-trainer".to_string(),
                    // Workaround for hi-dpi phones
                    #[cfg(target_arch = "wasm32")]
                    resolution: bevy::window::WindowResolution::new(800.0, 600.0),
                    fit_canvas_to_parent: true,
                    present_mode: PresentMode::AutoNoVsync,
                    ..Default::default()
                }),
                ..Default::default()
            }),
            GamePlugin,
        ))
        .run();
}
