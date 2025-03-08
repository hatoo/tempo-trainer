use bevy::prelude::*;
use bevy::window::{PresentMode, WindowMode};
use bevy::winit::WinitSettings;
use tempo_trainer::GamePlugin;

#[bevy_main]
fn main() {
    App::new()
        .insert_resource(WinitSettings::mobile())
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    resizable: false,
                    mode: WindowMode::BorderlessFullscreen(MonitorSelection::Current),
                    present_mode: PresentMode::AutoVsync,
                    ..default()
                }),
                ..default()
            }),
            GamePlugin,
        ))
        .run();
}
