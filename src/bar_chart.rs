use bevy::prelude::*;

use crate::{Delta, TapDeltas};

pub const BINS: usize = 16;
const BAR_HEIGHT_MULTIPLIER: f32 = 4000.0;

pub struct BarChartPlugin;
impl Plugin for BarChartPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(HideBarChart(false))
            .add_systems(Startup, setup)
            .add_systems(Update, (update_bins, hide_bar_chart));
    }
}

#[derive(Component)]
struct BarChart;
#[derive(Resource)]
pub struct HideBarChart(pub bool);
#[derive(Component)]
struct BinIndex(usize);

#[derive(Component)]
struct BinBar;

fn setup(mut commands: Commands) {
    commands
        .spawn((
            BarChart,
            Visibility::Visible,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_self: JustifySelf::Center,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
        ))
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
                                height: Val::Px(f / 60.0 * BAR_HEIGHT_MULTIPLIER + height / 2.0),
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
                                height: Val::Px(f / 60.0 * BAR_HEIGHT_MULTIPLIER + height / 2.0),
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
                                            Visibility::Inherited,
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
}

fn update_bins(
    mut query_bar: Query<
        (&BinIndex, &mut Node, &mut BackgroundColor, &mut Visibility),
        With<BinBar>,
    >,
    mut query_text: Query<(&BinIndex, &mut Text)>,
    tap_deltas: Res<TapDeltas>,
) {
    if tap_deltas.is_changed() {
        for (BinIndex(index), mut node, mut color, mut visibility) in &mut query_bar {
            if let Some(Delta { delta, .. }) = tap_deltas.0.get(*index) {
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

                *visibility = Visibility::Inherited;
            } else {
                *visibility = Visibility::Hidden;
            }
        }

        for (BinIndex(index), mut text) in &mut query_text {
            if let Some(Delta {
                delta, division, ..
            }) = tap_deltas.0.get(*index)
            {
                text.0 = format!("[{}]{:+.1}", division, delta * 1000.0);
            } else {
                text.0 = "".to_string();
            }
        }
    }
}

fn hide_bar_chart(
    mut bar_chart: Query<&mut Visibility, With<BarChart>>,
    hide_bar_chart: Res<HideBarChart>,
) {
    if hide_bar_chart.is_changed() {
        for mut visibility in &mut bar_chart {
            if hide_bar_chart.0 {
                *visibility = Visibility::Hidden;
            } else {
                *visibility = Visibility::Visible;
            }
        }
    }
}
