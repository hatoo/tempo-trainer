use std::time::Duration;

use bevy::{prelude::*, render::camera::ScalingMode};

const CLICK_AUDIO_PATH: &str = "sounds/c5.ogg";
const TAP_AUDIO_PATH: &str = "sounds/c4.ogg";

#[derive(Component)]
struct BpmText;

#[derive(Resource)]
struct LastTap(Duration);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(Time::<Fixed>::from_duration(from_bpm(90.0)))
        .insert_resource(LastTap(Duration::from_secs(0)))
        .add_systems(Startup, setup)
        .add_systems(FixedUpdate, metronome)
        .add_systems(Update, (tap, bpm_control, set_bpm_text))
        .run();
}

fn bpm(time: &Time<Fixed>) -> f32 {
    60.0 / time.timestep().as_secs_f32()
}

fn from_bpm(bpm: f32) -> Duration {
    Duration::from_secs_f32(60.0 / bpm)
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,

    timer: Res<Time<Fixed>>,
) {
    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical {
                viewport_height: 1200.0,
            },
            ..OrthographicProjection::default_2d()
        }),
    ));

    let circle_mesh = meshes.add(Mesh::from(Circle::new(100.0)));
    let circle_material = materials.add(Color::linear_rgb(1.0, 1.0, 0.2));
    commands.spawn((
        Mesh2d(circle_mesh),
        MeshMaterial2d(circle_material),
        Transform::from_xyz(0.0, 200.0, 0.0),
    ));

    commands.spawn((
        BpmText,
        Text::new(format!("BPM: {:.2}", bpm(&timer))),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..Default::default()
        },
    ));
}

fn tap(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) {
    if keyboard_input.get_just_pressed().count() > 0 {
        let audio = asset_server.load(TAP_AUDIO_PATH);

        commands.spawn(AudioPlayer::new(audio));

        commands.insert_resource(LastTap(time.elapsed()));
    }
}

fn metronome(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(AudioPlayer::new(asset_server.load(CLICK_AUDIO_PATH)));
}

fn bpm_control(mut timer: ResMut<Time<Fixed>>, keyboard_input: Res<ButtonInput<KeyCode>>) {
    if keyboard_input.just_pressed(KeyCode::ArrowUp) {
        let next_bpm = bpm(&timer).round() as u32 + 1;
        timer.set_timestep(from_bpm(next_bpm as f32));
    }

    if keyboard_input.just_pressed(KeyCode::ArrowRight) {
        let next_bpm = bpm(&timer).round() as u32 + 5;
        timer.set_timestep(from_bpm(next_bpm as f32));
    }

    if keyboard_input.just_pressed(KeyCode::ArrowDown) {
        let current_bpm = bpm(&timer).round() as u32;

        if current_bpm >= 1 {
            let next_bpm = current_bpm - 1;
            timer.set_timestep(from_bpm(next_bpm as f32));
        }
    }

    if keyboard_input.just_pressed(KeyCode::ArrowLeft) {
        let current_bpm = bpm(&timer).round() as u32;

        if current_bpm >= 5 {
            let next_bpm = current_bpm - 5;
            timer.set_timestep(from_bpm(next_bpm as f32));
        }
    }
}

fn set_bpm_text(timer: Res<Time<Fixed>>, mut query: Query<&mut Text, With<BpmText>>) {
    if timer.is_changed() {
        query.single_mut().0 = format!("BPM: {:.2}", bpm(&timer));
    }
}
