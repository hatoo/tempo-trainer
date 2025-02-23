use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

use bevy::{
    prelude::*,
    render::{camera::ScalingMode, mesh::CircleMeshBuilder},
};

const CLICK_AUDIO_PATH: &str = "sounds/c5.ogg";
const TAP_AUDIO_PATH: &str = "sounds/c4.ogg";

const CIRCLE_SIZE: f32 = 400.0;
const BINS: usize = 16;

#[derive(Component)]
struct StatusText;

#[derive(Component)]
struct ClockMarker;

#[derive(Resource)]
struct LastTick(Instant);

#[derive(Resource)]
struct Division(u32);

#[derive(Resource)]
struct TapDeltas(VecDeque<f64>);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(Time::<Fixed>::from_duration(from_bpm(90.0)))
        .insert_resource(LastTick(Instant::now()))
        .insert_resource(Division(1))
        .insert_resource(TapDeltas(VecDeque::new()))
        .add_systems(Startup, setup)
        .add_systems(FixedUpdate, metronome)
        .add_systems(Update, (tap, control, clock, set_status_text, set_bins))
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

    commands.spawn((
        Mesh2d(meshes.add(CircleMeshBuilder {
            circle: Circle::new(CIRCLE_SIZE),
            resolution: 128,
        })),
        MeshMaterial2d(materials.add(Color::linear_rgb(0.4, 0.4, 0.4))),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));

    commands.spawn((
        ClockMarker,
        Mesh2d(meshes.add(Mesh::from(Circle::new(CIRCLE_SIZE / 8.0)))),
        MeshMaterial2d(materials.add(Color::BLACK)),
        Transform::from_xyz(0.0, 0.0, 1.0),
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

    commands.spawn((
        Text::new("up/down: BPM +-1\nleft/right: BPM +-10\n[/]: Division +-1"),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(120.0),
            ..Default::default()
        },
    ));

    commands.spawn(BarChart::new(&mut meshes, &mut materials));
    for i in 0..BINS {
        commands.spawn(Bin::new(i, &mut meshes, &mut materials));
        commands.spawn(BinText::new(i));
    }
}

fn tap(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    last_tick: Res<LastTick>,
    timer: Res<Time<Fixed>>,
    division: Res<Division>,
    mut tap_deltas: ResMut<TapDeltas>,
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

        let delta = if delta_last < delta_next {
            delta_last
        } else {
            -delta_next
        };

        tap_deltas.0.push_front(delta);
        while tap_deltas.0.len() > BINS {
            tap_deltas.0.pop_back();
        }
        dbg!(&tap_deltas.0);
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

fn clock(
    last_tick: Res<LastTick>,
    timer: Res<Time<Fixed>>,
    mut query: Query<&mut Transform, With<ClockMarker>>,
) {
    let now = Instant::now();
    let time_step = timer.timestep();
    let delta = (now - last_tick.0).as_secs_f64() / time_step.as_secs_f64();

    let angle = 2.0 * std::f32::consts::PI * delta as f32;

    let mut transform = query.single_mut();
    transform.translation = Vec3::new(angle.sin() * CIRCLE_SIZE, angle.cos() * CIRCLE_SIZE, 1.0);
}

#[derive(Bundle)]
struct BarChart {
    line: (Mesh2d, MeshMaterial2d<ColorMaterial>, Transform),
}

impl BarChart {
    fn new(
        meshes: &mut ResMut<Assets<Mesh>>,
        materials: &mut ResMut<Assets<ColorMaterial>>,
    ) -> Self {
        Self {
            line: (
                Mesh2d(meshes.add(Mesh::from(Rectangle {
                    half_size: Vec2::new(1000.0, 1.0),
                }))),
                MeshMaterial2d(materials.add(Color::BLACK)),
                Transform::from_xyz(0.0, 0.0, 2.0),
            ),
        }
    }
}

#[derive(Component)]
struct BinIndex(usize);

#[derive(Component)]
struct BinBar;

#[derive(Bundle)]
struct Bin {
    index: BinIndex,
    rect: Mesh2d,
    material: MeshMaterial2d<ColorMaterial>,
    transform: Transform,
    bin_bar: BinBar,
}

impl Bin {
    fn new(
        index: usize,
        meshes: &mut ResMut<Assets<Mesh>>,
        materials: &mut ResMut<Assets<ColorMaterial>>,
    ) -> Self {
        let mut transform = Transform::from_xyz(0.0, 0.0, 4.0);
        transform.scale = Vec3::ZERO;
        transform.translation.x = (index as f32 - (BINS / 2) as f32) * 100.0;
        Self {
            index: BinIndex(index),
            rect: Mesh2d(meshes.add(Mesh::from(Rectangle {
                half_size: Vec2::new(0.5, 0.5),
            }))),
            material: MeshMaterial2d(materials.add(Color::linear_rgb(0.0, 0.0, 1.0))),
            transform,
            bin_bar: BinBar,
        }
    }
}

#[derive(Bundle)]
struct BinText {
    index: BinIndex,
    text: Text2d,
    trandform: Transform,
}

impl BinText {
    fn new(index: usize) -> Self {
        Self {
            index: BinIndex(index),
            text: Text2d::new(""),
            trandform: Transform::from_xyz((index as f32 - (BINS / 2) as f32) * 100.0, 0.0, 8.0),
        }
    }
}

fn set_bins(
    mut query_bar: Query<(&BinIndex, &mut Transform), With<BinBar>>,
    mut query_text: Query<(&BinIndex, &mut Text2d)>,
    tap_deltas: Res<TapDeltas>,
) {
    if tap_deltas.is_changed() {
        for (BinIndex(index), mut transform) in &mut query_bar {
            if let Some(delta) = tap_deltas.0.get(*index) {
                let height = *delta as f32 * 4000.0;
                transform.translation.y = height / 2.0;
                transform.scale = Vec3::new(88.0, height, 1.0);
            } else {
                transform.scale = Vec3::ZERO;
            }
        }

        for (BinIndex(index), mut text) in &mut query_text {
            if let Some(delta) = tap_deltas.0.get(*index) {
                text.0 = format!("{:.1}", delta * 1000.0);
            } else {
                text.0 = "".to_string();
            }
        }
    }
}
