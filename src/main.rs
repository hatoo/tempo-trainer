use std::collections::VecDeque;

use bevy::utils::{Duration, Instant};

use bevy::{
    color::palettes::basic::*,
    diagnostic::{DiagnosticsStore, EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin},
    prelude::*,
    render::{camera::ScalingMode, mesh::CircleMeshBuilder},
};

const TICK_AUDIO_PATH: &str = "sounds/c5.ogg";
const TAP_AUDIO_PATH: &str = "sounds/c4.ogg";

const CIRCLE_SIZE: f32 = 400.0;
const BINS: usize = 16;

const BAR_HEIGHT_MULTIPLIER: f32 = 4000.0;

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

#[derive(Resource, Default)]
struct Mute {
    tick_mute: bool,
    tap_mute: bool,
}

#[derive(Resource)]
struct HideClock(bool);

#[derive(Component)]
struct Clock;

#[derive(Resource)]
struct AudioHandles {
    tick: Handle<AudioSource>,
    tap: Handle<AudioSource>,
}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "tempo-trainer".to_string(),
                    canvas: Some("#screen".to_string()),
                    fit_canvas_to_parent: true,
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
        .insert_resource(Mute::default())
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
                button_system,
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

const NORMAL_BUTTON: Color = Color::srgb(0.15, 0.15, 0.15);
const PRESSED_BUTTON: Color = Color::srgb(0.35, 0.75, 0.35);

#[derive(Component, Clone, Copy)]
enum ButtonKind {
    BpmUp1,
    BpmDown1,
    BpmUp10,
    BpmDown10,
    DivisionUp1,
    DivisionDown1,
    TapMute,
    TickMute,
    HideClock,
}

impl ButtonKind {
    fn label(&self) -> &str {
        match self {
            ButtonKind::BpmUp1 => "BPM+1",
            ButtonKind::BpmDown1 => "BPM-1",
            ButtonKind::BpmUp10 => "BPM+10",
            ButtonKind::BpmDown10 => "BPM-10",
            ButtonKind::DivisionUp1 => "Div+",
            ButtonKind::DivisionDown1 => "Div-",
            ButtonKind::TapMute => "Tap Mute",
            ButtonKind::TickMute => "Tick Mute",
            ButtonKind::HideClock => "Hide Clock",
        }
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    asset_server: Res<AssetServer>,
) {
    commands.insert_resource(AudioHandles {
        tick: asset_server.load(TICK_AUDIO_PATH),
        tap: asset_server.load(TAP_AUDIO_PATH),
    });

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

    commands.spawn(
        Node {
            position_type: PositionType::Absolute,
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            top: Val::Px(12.0),
            left: Val::Px(0.0),
            ..Default::default()
        },
    ).with_children(|commands| {
        commands.spawn((
            StatusText,
            Text::new(""),
            Node {
                margin: UiRect {
                    left: Val::Px(12.0),
                    ..Default::default()
                },
                ..Default::default()
            },
        ));

        commands.spawn((
            Text::new(
                "up/down: BPM +-1\nleft/right: BPM +-10\n[/]: Division +-1\nn: Tap Mute\nm: Tick Mute\n,: Hide Clock",
            ),
            Node {
                margin: UiRect {
                    left: Val::Px(12.0),
                    ..Default::default()
                },
                ..Default::default()
            },
        ));
    });

    // Bar chart

    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_self: JustifySelf::Center,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|commands| {
            commands
                .spawn(Node {
                    display: Display::Flex,
                    justify_self: JustifySelf::Center,
                    flex_direction: FlexDirection::Row,
                    width: Val::Percent(80.0),
                    height: Val::Percent(100.0),
                    ..Default::default()
                })
                .with_children(|commands| {
                    for (f, height, label) in [
                        (0.0, 4.0, "0"),
                        (1.0, 3.0, "1/60"),
                        (1.5, 2.0, "1.5/60"),
                        (2.0, 1.0, "2/60"),
                    ] {
                        commands.spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                width: Val::Percent(100.0),
                                height: Val::Px(f / 60.0 * BAR_HEIGHT_MULTIPLIER - height / 2.0),
                                bottom: Val::Percent(50.0),
                                border: UiRect {
                                    top: Val::Px(height),
                                    ..default()
                                },
                                ..default()
                            },
                            BorderColor(Color::BLACK),
                        ));
                        commands.spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                width: Val::Percent(100.0),
                                height: Val::Px(f / 60.0 * BAR_HEIGHT_MULTIPLIER - height / 2.0),
                                top: Val::Percent(50.0),
                                border: UiRect {
                                    bottom: Val::Px(height),
                                    ..default()
                                },
                                ..default()
                            },
                            BorderColor(Color::BLACK),
                        ));

                        commands
                            .spawn((
                                Node {
                                    position_type: PositionType::Absolute,
                                    left: Val::Px(-12.0),
                                    height: Val::Px(f / 60.0 * BAR_HEIGHT_MULTIPLIER),
                                    width: Val::Percent(100.0),
                                    bottom: Val::Percent(50.0),
                                    ..default()
                                },
                                // BackgroundColor(Color::linear_rgba(0.0, 1.0, 0.0, 0.3)),
                            ))
                            .with_children(|commands| {
                                commands.spawn((
                                    Node {
                                        position_type: PositionType::Absolute,
                                        right: Val::Percent(100.0),
                                        bottom: Val::Percent(100.0),
                                        ..default()
                                    },
                                    Text::new(label),
                                    TextFont {
                                        font_size: 10.3,
                                        ..Default::default()
                                    },
                                ));
                            });
                    }

                    for i in 0..BINS {
                        commands
                            .spawn(Node {
                                margin: UiRect {
                                    left: Val::Px(4.0),
                                    right: Val::Px(4.0),
                                    ..default()
                                },
                                flex_grow: 1.0,
                                flex_basis: Val::Px(0.0),
                                justify_content: JustifyContent::Center,
                                justify_self: JustifySelf::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            })
                            .with_children(|commands| {
                                commands
                                    .spawn((
                                        Node {
                                            width: Val::Percent(100.0),
                                            height: Val::Percent(100.0),
                                            justify_content: JustifyContent::Center,
                                            justify_self: JustifySelf::Center,
                                            align_items: AlignItems::Center,
                                            ..default()
                                        },
                                        // BackgroundColor(Color::linear_rgba(0.0, 1.0, 0.0, 0.3)),
                                    ))
                                    .with_children(|commands| {
                                        commands.spawn((
                                            BinBar,
                                            BinIndex(i),
                                            Visibility::Visible,
                                            Node {
                                                position_type: PositionType::Absolute,
                                                width: Val::Percent(100.0),
                                                height: Val::Px(100.0),
                                                top: Val::Percent(50.0),
                                                bottom: Val::DEFAULT,
                                                justify_content: JustifyContent::Center,
                                                align_items: AlignItems::Center,
                                                ..default()
                                            },
                                            BackgroundColor(Color::linear_rgb(0.0, 0.0, 1.0)),
                                        ));
                                        commands.spawn((
                                            BinIndex(i),
                                            Text::new("1.23"),
                                            TextFont {
                                                font_size: 10.3,
                                                ..Default::default()
                                            },
                                        ));
                                    });
                            });
                    }
                });
        });

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

    // UI Buttons

    let mut node = commands.spawn(Node {
        position_type: PositionType::Absolute,
        width: Val::Percent(100.),
        bottom: Val::Px(4.0),
        display: Display::Flex,
        flex_direction: FlexDirection::Row,
        flex_wrap: FlexWrap::Wrap,
        ..default()
    });
    let mut add_button = move |kind: ButtonKind| {
        node.with_children(|parent| {
            parent
                .spawn((
                    Button,
                    kind,
                    Node {
                        // width: Val::Px(105.0),
                        // height: Val::Px(48.0),
                        border: UiRect::all(Val::Px(2.0)),
                        // horizontally center child text
                        justify_content: JustifyContent::Center,
                        // vertically center child text
                        align_items: AlignItems::Center,
                        margin: UiRect {
                            left: Val::Px(2.0),
                            right: Val::Px(2.0),
                            ..Default::default()
                        },
                        ..default()
                    },
                    BorderColor(Color::BLACK),
                    BorderRadius::all(Val::Px(4.0)),
                    BackgroundColor(NORMAL_BUTTON),
                ))
                .with_child((
                    Text::new(kind.label()),
                    TextColor(Color::srgb(0.9, 0.9, 0.9)),
                ));
        });
    };
    add_button(ButtonKind::BpmDown10);
    add_button(ButtonKind::BpmDown1);
    add_button(ButtonKind::BpmUp1);
    add_button(ButtonKind::BpmUp10);

    add_button(ButtonKind::DivisionDown1);
    add_button(ButtonKind::DivisionUp1);

    add_button(ButtonKind::TapMute);
    add_button(ButtonKind::TickMute);
    add_button(ButtonKind::HideClock);
}

#[allow(clippy::too_many_arguments)]
fn tap(
    mut commands: Commands,
    audio_handles: Res<AudioHandles>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    touches: Res<Touches>,
    last_tick: Res<LastTick>,
    timer: Res<Time<Fixed>>,
    division: Res<Division>,
    mut tap_deltas: ResMut<TapDeltas>,
    mute: Res<Mute>,
) {
    if keyboard_input.get_just_pressed().count() > 0
        || buttons.get_just_pressed().count() > 0
        || touches.any_just_pressed()
    {
        if !mute.tap_mute {
            commands.spawn((
                AudioPlayer::new(audio_handles.tap.clone()),
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
    audio_handles: Res<AudioHandles>,
    mut last_tick: ResMut<LastTick>,
    mute: Res<Mute>,
) {
    if !mute.tick_mute {
        commands.spawn((
            AudioPlayer::new(audio_handles.tick.clone()),
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

    if keyboard_input.just_pressed(KeyCode::KeyN) {
        mute.tap_mute = !mute.tap_mute;
    }

    if keyboard_input.just_pressed(KeyCode::KeyM) {
        mute.tick_mute = !mute.tick_mute;
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
        for mut text in &mut query {
            text.0 = format!(
                "BPM: {}\n1 / {}\nTick Mute: {}\nTap Mute: {}",
                bpm(&timer).round() as u32,
                division.0,
                mute.tick_mute,
                mute.tap_mute
            );
        }
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

#[derive(Component)]
struct BinIndex(usize);

#[derive(Component)]
struct BinBar;

fn set_bins(
    mut query_bar: Query<
        (&BinIndex, &mut Node, &mut BackgroundColor, &mut Visibility),
        With<BinBar>,
    >,
    mut query_text: Query<(&BinIndex, &mut Text)>,
    tap_deltas: Res<TapDeltas>,
) {
    if tap_deltas.is_changed() {
        for (BinIndex(index), mut node, mut color, mut visibility) in &mut query_bar {
            if let Some(delta) = tap_deltas.0.get(*index) {
                let height = delta.abs() as f32 * BAR_HEIGHT_MULTIPLIER;
                node.height = Val::Px(height);
                node.position_type = PositionType::Absolute;

                if *delta >= 0.0 {
                    color.0 = Color::linear_rgba(1.0, 0.0, 0.0, 0.6);
                    node.top = Val::DEFAULT;
                    node.bottom = Val::Percent(50.0);
                } else {
                    color.0 = Color::linear_rgba(0.0, 0.0, 1.0, 0.6);
                    node.bottom = Val::DEFAULT;
                    node.top = Val::Percent(50.0);
                }

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

        for parent in &parent {
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
}

#[derive(Component)]
struct DiagnosticsText;

fn diagnostics_text_update_system(
    diagnostics: Res<DiagnosticsStore>,
    mut query: Query<&mut Text, With<DiagnosticsText>>,
) {
    if diagnostics.is_changed() {
        let fps = if let Some(fps) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS) {
            if let Some(value) = fps.smoothed() {
                format!("{value:.2}")
            } else {
                "N/A".to_string()
            }
        } else {
            "N/A".to_string()
        };

        let entity_count = if let Some(entity_count) =
            diagnostics.get(&EntityCountDiagnosticsPlugin::ENTITY_COUNT)
        {
            if let Some(value) = entity_count.value() {
                format!("{value:.0}")
            } else {
                "N/A".to_string()
            }
        } else {
            "N/A".to_string()
        };

        for mut span in &mut query {
            **span = format!("entity_count: {entity_count} FPS: {fps}");
        }
    }
}

#[allow(clippy::type_complexity)]
fn button_system(
    mut interaction_query: Query<
        (
            &Interaction,
            &mut BackgroundColor,
            &mut BorderColor,
            &ButtonKind,
        ),
        (Changed<Interaction>, With<Button>),
    >,

    mut timer: ResMut<Time<Fixed>>,
    mut division: ResMut<Division>,
    mut mute: ResMut<Mute>,
    mut hide_clock: ResMut<HideClock>,
) {
    for (interaction, mut color, mut border_color, button_kind) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                *color = PRESSED_BUTTON.into();
                border_color.0 = RED.into();

                match button_kind {
                    ButtonKind::BpmUp1 => {
                        let next_bpm = bpm(&timer).round() as u32 + 1;
                        timer.set_timestep(from_bpm(next_bpm as f32));
                    }
                    ButtonKind::BpmDown1 => {
                        let current_bpm = bpm(&timer).round() as u32;

                        if current_bpm > 1 {
                            let next_bpm = current_bpm - 1;
                            timer.set_timestep(from_bpm(next_bpm as f32));
                        }
                    }
                    ButtonKind::BpmUp10 => {
                        let next_bpm = bpm(&timer).round() as u32 + 10;
                        timer.set_timestep(from_bpm(next_bpm as f32));
                    }
                    ButtonKind::BpmDown10 => {
                        let current_bpm = bpm(&timer).round() as u32;

                        let next_bpm = if current_bpm > 10 {
                            current_bpm - 10
                        } else {
                            1
                        };

                        timer.set_timestep(from_bpm(next_bpm as f32));
                    }
                    ButtonKind::DivisionUp1 => {
                        division.0 += 1;
                    }
                    ButtonKind::DivisionDown1 => {
                        if division.0 > 1 {
                            division.0 -= 1;
                        }
                    }
                    ButtonKind::TapMute => {
                        mute.tap_mute = !mute.tap_mute;
                    }
                    ButtonKind::TickMute => {
                        mute.tick_mute = !mute.tick_mute;
                    }
                    ButtonKind::HideClock => {
                        hide_clock.0 = !hide_clock.0;
                    }
                }
            }
            Interaction::None | Interaction::Hovered => {
                *color = NORMAL_BUTTON.into();
                border_color.0 = Color::BLACK;
            }
        }
    }
}
