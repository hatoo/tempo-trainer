use std::time::{Duration, Instant};

use bevy::{prelude::*, render::camera::ScalingMode};

const CLICK_AUDIO_PATH: &str = "sounds/c5.ogg";
const TAP_AUDIO_PATH: &str = "sounds/c4.ogg";

#[derive(Component)]
struct StatusText;

#[derive(Resource)]
struct LastTick(Instant);

#[derive(Resource)]
struct Division(u32);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(Time::<Fixed>::from_duration(from_bpm(90.0)))
        .insert_resource(LastTick(Instant::now()))
        .insert_resource(Division(1))
        .add_systems(Startup, setup)
        .add_systems(FixedUpdate, metronome)
        .add_systems(Update, (tap, control, set_status_text))
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
        StatusText,
        Text::new(""),
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
    last_tick: Res<LastTick>,
    timer: Res<Time<Fixed>>,
    division: Res<Division>,
) {
    if keyboard_input.get_just_pressed().count() > 0 {
        commands.spawn(AudioPlayer::new(asset_server.load(TAP_AUDIO_PATH)));

        let now = Instant::now();
        let time_step = timer.timestep();
        let time_step_div = time_step / division.0;

        let last_tick = last_tick.0;
        let next_tick = last_tick + time_step;

        let delta_last = (now - last_tick).as_secs_f64() % time_step_div.as_secs_f64();
        let delta_next = (next_tick - now).as_secs_f64() % time_step_div.as_secs_f64();

        let delta_ms = if delta_last < delta_next {
            delta_last * 1000.0
        } else {
            -(delta_next * 1000.0)
        };

        dbg!(delta_ms);
    }
}

fn metronome(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut last_tick: ResMut<LastTick>,
) {
    commands.spawn(AudioPlayer::new(asset_server.load(CLICK_AUDIO_PATH)));
    last_tick.0 = Instant::now();
}

fn control(
    mut timer: ResMut<Time<Fixed>>,
    mut division: ResMut<Division>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    if keyboard_input.just_pressed(KeyCode::ArrowUp) {
        let next_bpm = bpm(&timer).round() as u32 + 1;
        timer.set_timestep(from_bpm(next_bpm as f32));
    }

    if keyboard_input.just_pressed(KeyCode::ArrowRight) {
        let next_bpm = bpm(&timer).round() as u32 + 10;
        timer.set_timestep(from_bpm(next_bpm as f32));
    }

    if keyboard_input.just_pressed(KeyCode::ArrowDown) {
        let current_bpm = bpm(&timer).round() as u32;

        if current_bpm > 1 {
            let next_bpm = current_bpm - 1;
            timer.set_timestep(from_bpm(next_bpm as f32));
        }
    }

    if keyboard_input.just_pressed(KeyCode::ArrowLeft) {
        let current_bpm = bpm(&timer).round() as u32;

        let next_bpm = if current_bpm > 10 {
            current_bpm - 10
        } else {
            1
        };

        timer.set_timestep(from_bpm(next_bpm as f32));
    }

    if keyboard_input.just_pressed(KeyCode::BracketLeft) {
        if division.0 > 1 {
            division.0 -= 1;
        }
    }

    if keyboard_input.just_pressed(KeyCode::BracketRight) {
        division.0 += 1;
    }
}

fn set_status_text(
    timer: Res<Time<Fixed>>,
    division: Res<Division>,
    mut query: Query<&mut Text, With<StatusText>>,
) {
    if timer.is_changed() || division.is_changed() {
        query.single_mut().0 = format!("BPM: {}\n1 / {}", bpm(&timer) as u32, division.0);
    }
}
