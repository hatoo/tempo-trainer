use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

use bevy::{
    diagnostic::{DiagnosticsStore, EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin},
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

#[derive(Resource)]
struct Mute(bool);

#[derive(Resource)]
struct HideClock(bool);

#[derive(Component)]
struct Clock;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "tempo-trainer".to_string(),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            FrameTimeDiagnosticsPlugin,
            EntityCountDiagnosticsPlugin,
        ))
        .insert_resource(Time::<Fixed>::from_duration(from_bpm(90.0)))
        .insert_resource(LastTick(Instant::now()))
        .insert_resource(Division(1))
        .insert_resource(TapDeltas(VecDeque::new()))
        .insert_resource(Mute(false))
        .insert_resource(HideClock(false))
        .add_systems(Startup, setup)
        .add_systems(FixedUpdate, metronome)
        .add_systems(
            Update,
            (
                tap,
                control,
                clock,
                set_status_text,
                set_bins,
                set_clock_legend,
                diagnostics_text_update_system,
                hide_clock,
            ),
        )
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

    commands
        .spawn((Clock, Transform::default(), Visibility::Hidden))
        .with_children(|commands| {
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
        });

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
        Text::new(
            "up/down: BPM +-1\nleft/right: BPM +-10\n[/]: Division +-1\nm: Mute\n,: Hide Clock",
        ),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(240.0),
            ..Default::default()
        },
    ));

    commands.spawn(BarChart::new(&mut meshes, &mut materials));
    for i in 0..BINS {
        commands.spawn(Bin::new(i, &mut meshes, &mut materials));
        commands.spawn(BinText::new(i));
    }

    commands.spawn((
        DiagnosticsText,
        Text::new(""),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(4.0),
            right: Val::Px(4.0),
            ..default()
        },
    ));
}

#[allow(clippy::too_many_arguments)]
fn tap(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    last_tick: Res<LastTick>,
    timer: Res<Time<Fixed>>,
    division: Res<Division>,
    mut tap_deltas: ResMut<TapDeltas>,
    mute: Res<Mute>,
) {
    if keyboard_input.get_just_pressed().count() > 0 {
        if !mute.0 {
            commands.spawn((
                AudioPlayer::new(asset_server.load(TAP_AUDIO_PATH)),
                PlaybackSettings::DESPAWN,
            ));
        }

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
    }
}

fn metronome(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut last_tick: ResMut<LastTick>,
    mute: Res<Mute>,
) {
    if !mute.0 {
        commands.spawn((
            AudioPlayer::new(asset_server.load(CLICK_AUDIO_PATH)),
            PlaybackSettings::DESPAWN,
        ));
    }
    last_tick.0 = Instant::now();
}

fn control(
    mut timer: ResMut<Time<Fixed>>,
    mut division: ResMut<Division>,
    mut mute: ResMut<Mute>,
    mut hide_clock: ResMut<HideClock>,
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

    if keyboard_input.just_pressed(KeyCode::BracketLeft) && division.0 > 1 {
        division.0 -= 1;
    }

    if keyboard_input.just_pressed(KeyCode::BracketRight) {
        division.0 += 1;
    }

    if keyboard_input.just_pressed(KeyCode::KeyM) {
        mute.0 = !mute.0;
    }

    if keyboard_input.just_pressed(KeyCode::Comma) {
        hide_clock.0 = !hide_clock.0;
    }
}

fn set_status_text(
    timer: Res<Time<Fixed>>,
    division: Res<Division>,
    mute: Res<Mute>,
    mut query: Query<&mut Text, With<StatusText>>,
) {
    if timer.is_changed() || division.is_changed() || mute.is_changed() {
        query.single_mut().0 = format!(
            "BPM: {}\n1 / {}\nMute: {}",
            bpm(&timer).round() as u32,
            division.0,
            mute.0
        );
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
    visibility: Visibility,
}

impl Bin {
    fn new(
        index: usize,
        meshes: &mut ResMut<Assets<Mesh>>,
        materials: &mut ResMut<Assets<ColorMaterial>>,
    ) -> Self {
        Self {
            index: BinIndex(index),
            rect: Mesh2d(meshes.add(Mesh::from(Rectangle {
                half_size: Vec2::new(0.5, 0.5),
            }))),
            material: MeshMaterial2d(materials.add(Color::linear_rgb(0.0, 0.0, 1.0))),
            transform: Transform::from_xyz((index as f32 - (BINS / 2) as f32) * 100.0, 0.0, 4.0),
            bin_bar: BinBar,
            visibility: Visibility::Hidden,
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
    mut query_bar: Query<
        (
            &BinIndex,
            &mut MeshMaterial2d<ColorMaterial>,
            &mut Transform,
            &mut Visibility,
        ),
        With<BinBar>,
    >,
    mut query_text: Query<(&BinIndex, &mut Text2d)>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    tap_deltas: Res<TapDeltas>,
) {
    if tap_deltas.is_changed() {
        for (BinIndex(index), mut material, mut transform, mut visibility) in &mut query_bar {
            if let Some(delta) = tap_deltas.0.get(*index) {
                let height = *delta as f32 * 4000.0;
                transform.translation.y = height / 2.0;
                transform.scale = Vec3::new(88.0, height, 1.0);

                let color = if *delta > 0.0 {
                    Color::linear_rgba(1.0, 0.0, 0.0, 0.6)
                } else {
                    Color::linear_rgba(0.0, 0.0, 1.0, 0.6)
                };

                // TODO: reuse material handle
                material.0 = materials.add(color);
                *visibility = Visibility::Visible;
            } else {
                *visibility = Visibility::Hidden;
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

fn hide_clock(mut clock: Query<&mut Visibility, With<Clock>>, hide_clock: Res<HideClock>) {
    if hide_clock.is_changed() {
        for mut visibility in &mut clock {
            if hide_clock.0 {
                *visibility = Visibility::Hidden;
            } else {
                *visibility = Visibility::Visible;
            }
        }
    }
}

#[derive(Component)]
struct ClockLegend;

fn set_clock_legend(
    mut commands: Commands,
    query: Query<Entity, With<ClockLegend>>,
    parent: Query<Entity, With<Clock>>,
    division: Res<Division>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    if division.is_changed() {
        for e in query.iter() {
            commands.entity(e).despawn_recursive();
        }

        let parent = parent.single();
        let division = division.0;

        // TODO: reuse mesh and material handles
        let mesh = Mesh2d(meshes.add(Mesh::from(Circle { radius: 16.0 })));
        let material = MeshMaterial2d(materials.add(Color::linear_rgb(0.1, 0.3, 0.1)));

        commands.entity(parent).with_children(|commands| {
            for bundle in (0..division).map(|i| {
                let angle = 2.0 * std::f32::consts::PI * (i as f32 / division as f32);
                let x = angle.sin() * CIRCLE_SIZE;
                let y = angle.cos() * CIRCLE_SIZE;

                (
                    ClockLegend,
                    mesh.clone(),
                    material.clone(),
                    Transform::from_xyz(x, y, 3.0),
                )
            }) {
                commands.spawn(bundle);
            }
        });
    }
}

#[derive(Component)]
struct DiagnosticsText;

fn diagnostics_text_update_system(
    diagnostics: Res<DiagnosticsStore>,
    mut query: Query<&mut Text, With<DiagnosticsText>>,
) {
    let fps = if let Some(fps) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS) {
        if let Some(value) = fps.smoothed() {
            format!("{value:.2}")
        } else {
            "N/A".to_string()
        }
    } else {
        "N/A".to_string()
    };

    let entity_count =
        if let Some(entity_count) = diagnostics.get(&EntityCountDiagnosticsPlugin::ENTITY_COUNT) {
            if let Some(value) = entity_count.value() {
                format!("{value:.0}")
            } else {
                "N/A".to_string()
            }
        } else {
            "N/A".to_string()
        };

    if diagnostics.is_changed() {
        for mut span in &mut query {
            **span = format!("entity_count: {entity_count} FPS: {fps}");
        }
    }
}
