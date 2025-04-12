use crate::{Delta, Division, LastTick, TapDeltas};
use bevy::{platform_support::time::Instant, prelude::*, render::mesh::CircleMeshBuilder};

const CIRCLE_SIZE: f32 = 400.0;

pub struct ClockPlugin;

#[derive(Component)]
struct Clock;

#[derive(Component)]
struct ClockMarker;

#[derive(Resource)]
pub struct HideClock(pub bool);

#[derive(Resource)]
struct ClockResource {
    mesh_legend: Handle<Mesh>,
    material_legend: Handle<ColorMaterial>,
    mesh_delta: Handle<Mesh>,
    material_delta: Handle<ColorMaterial>,
    mesh_precision: Handle<Mesh>,
    material_precision: Handle<ColorMaterial>,
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.insert_resource(ClockResource {
        mesh_legend: meshes.add(Mesh::from(Circle { radius: 16.0 })),
        material_legend: materials.add(Color::linear_rgb(0.1, 0.3, 0.1)),
        mesh_delta: meshes.add(Mesh::from(Circle { radius: 12.0 })),
        material_delta: materials.add(Color::linear_rgb(0.1, 0.1, 0.3)),
        mesh_precision: meshes.add(Mesh::from(Rectangle {
            half_size: Vec2::new(0.5, 0.5),
        })),
        material_precision: materials.add(Color::linear_rgb(0.0, 0.0, 0.0)),
    });

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
                Mesh2d(meshes.add(CircleMeshBuilder {
                    circle: Circle::new(CIRCLE_SIZE + 4.0),
                    resolution: 128,
                })),
                MeshMaterial2d(materials.add(Color::linear_rgb(0.1, 0.1, 0.1))),
                Transform::from_xyz(0.0, 0.0, -1.0),
            ));

            commands.spawn((
                ClockMarker,
                Mesh2d(meshes.add(Mesh::from(Circle::new(CIRCLE_SIZE / 8.0)))),
                MeshMaterial2d(materials.add(Color::BLACK)),
                Transform::from_xyz(0.0, 0.0, 1.0),
            ));
        });
}

fn update_clock_marker(
    last_tick: Res<LastTick>,
    timer: Res<Time<Fixed>>,
    mut query: Query<&mut Transform, With<ClockMarker>>,
) {
    let now = Instant::now();
    let time_step = timer.timestep();
    let delta = (now - last_tick.0).as_secs_f64() / time_step.as_secs_f64();

    let angle = 2.0 * std::f32::consts::PI * delta as f32;

    for mut transform in &mut query {
        transform.translation =
            Vec3::new(angle.sin() * CIRCLE_SIZE, angle.cos() * CIRCLE_SIZE, 1.0);
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

fn update_clock_legend(
    mut commands: Commands,
    query: Query<Entity, With<ClockLegend>>,
    parent: Query<Entity, With<Clock>>,
    division: Res<Division>,
    clock_resource: Res<ClockResource>,
    timer: Res<Time<Fixed>>,
) {
    if division.is_changed() || timer.is_changed() {
        for e in query.iter() {
            commands.entity(e).despawn();
        }

        let division = division.0;
        let tick = timer.timestep().as_secs_f32();

        for parent in &parent {
            commands.entity(parent).with_children(|commands| {
                for i in 0..division {
                    let angle = 2.0 * std::f32::consts::PI * (i as f32 / division as f32);
                    let x = angle.sin() * CIRCLE_SIZE;
                    let y = angle.cos() * CIRCLE_SIZE;

                    commands.spawn((
                        ClockLegend,
                        Mesh2d(clock_resource.mesh_legend.clone()),
                        MeshMaterial2d(clock_resource.material_legend.clone()),
                        Transform::from_xyz(x, y, 3.0),
                    ));

                    let t = tick / division as f32 * i as f32;

                    for delta in [
                        -1.0 / 60.0,
                        1.0 / 60.0,
                        -1.5 / 60.0,
                        1.5 / 60.0,
                        -2.0 / 60.0,
                        2.0 / 60.0,
                    ] {
                        let theta = (t + delta) / tick * 2.0 * std::f32::consts::PI;

                        let mut transform = Transform::from_scale(Vec3::new(
                            6.0,
                            96.0 * (-delta.abs() * 60.0).exp(),
                            1.0,
                        ))
                        .with_translation(Vec3::new(
                            0.0,
                            CIRCLE_SIZE,
                            8.0,
                        ));
                        transform.rotate_around(Vec3::ZERO, Quat::from_rotation_z(theta));

                        commands.spawn((
                            ClockLegend,
                            Mesh2d(clock_resource.mesh_precision.clone()),
                            MeshMaterial2d(clock_resource.material_precision.clone()),
                            transform,
                        ));
                    }
                }
            });
        }
    }
}

#[derive(Component)]
struct ClockDelta;

fn set_clock_delta(
    mut commands: Commands,
    query: Query<Entity, With<ClockDelta>>,
    tap_deltas: Res<TapDeltas>,
    parent: Query<Entity, With<Clock>>,
    clock_resource: Res<ClockResource>,
) {
    if tap_deltas.is_changed() {
        for e in query.iter() {
            commands.entity(e).despawn();
        }

        for parent in &parent {
            commands.entity(parent).with_children(|commands| {
                for Delta { theta, .. } in tap_deltas.0.iter() {
                    let x = theta.sin() as f32 * CIRCLE_SIZE;
                    let y = theta.cos() as f32 * CIRCLE_SIZE;

                    commands.spawn((
                        ClockDelta,
                        Mesh2d(clock_resource.mesh_delta.clone()),
                        MeshMaterial2d(clock_resource.material_delta.clone()),
                        Transform::from_xyz(x, y, 4.0),
                    ));
                }
            });
        }
    }
}

impl Plugin for ClockPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(HideClock(false))
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                (
                    update_clock_marker,
                    update_clock_legend,
                    set_clock_delta,
                    hide_clock,
                ),
            );
    }
}
