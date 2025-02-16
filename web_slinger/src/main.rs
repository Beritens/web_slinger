mod collider_import;
mod physics;

use crate::collider_import::CollisionImportPlugin;
use crate::physics::{
    Collider, PhysicsPlugin, Shape, StaticCollider, Stick, SubStepSchedule, VerletObject,
};
use bevy::app::{FixedUpdate, Startup};
use bevy::color::Color;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::{
    App, ButtonInput, Camera, Camera2d, ClearColor, Commands, Component, Entity, GlobalTransform,
    MouseButton, PluginGroup, Query, Res, Single, Transform, Update, Vec2, Vec3, Window, With,
    Without, World,
};
use bevy::sprite::Sprite;
use bevy::utils::default;
use bevy::window::{CompositeAlphaMode, PrimaryWindow, WindowLevel, WindowPlugin};
use bevy::DefaultPlugins;
use bevy_wasm_window_resize::WindowResizePlugin;
use std::process::id;

fn main() {
    println!("Web_Slinger activated");
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            #[cfg(target_arch = "wasm32")]
            canvas: Some("#bevy".into()),
            ..default()
        }),
        ..default()
    }));
    app.add_systems(Startup, setup);
    app.add_plugins(WindowResizePlugin);
    app.add_plugins(CollisionImportPlugin);
    app.add_plugins(PhysicsPlugin);

    #[cfg(target_arch = "wasm32")]
    app.insert_resource(ClearColor(Color::NONE));
    app.add_systems(FixedUpdate, (follow_mouse_system));
    app.add_systems(Update, (shoot_rope_system, spawn_rope_system));
    app.run();
}

#[derive(Component)]
struct Player;

#[derive(Component)]
struct RopeHolder {
    last_pos: Vec2,
    hand: Entity,
}

fn setup(mut commands: Commands) {
    commands.spawn((Camera2d));
    let mut last_ent: Option<Entity> = None;

    let p = Vec2::new(-50.0, 0.0);
    let hand = commands.spawn((
        Transform::from_xyz(-50.0, 0.0, 0.0),
        RopeShooter {
            delete_old: true,
            connections: vec![],
        },
        Collider {
            shape: Shape::Circle { radius: 2.0 },
            layer: 2,
            layer_mask: 1,
        },
        VerletObject {
            fixed: false,
            position_old: p,
            position_current: p,
            acceleration: Vec2::ZERO,
            friction: 0.8,
            ..default()
        },
        Sprite::from_color(Color::BLACK, Vec2::splat(8.0)),
    ));
    let hand_ent = hand.id();

    commands.spawn((
        Transform::from_xyz(0.0, 0.0, 0.0),
        Collider {
            shape: Shape::Circle { radius: 8.0 },
            layer: 2,
            layer_mask: 1,
        },
        RopeHolder {
            hand: hand_ent,
            last_pos: Vec2::new(0.0, 0.0),
        },
        VerletObject {
            fixed: false,
            position_old: p,
            position_current: p,
            acceleration: Vec2::ZERO,
            ..default()
        },
        Sprite::from_color(Color::BLACK, Vec2::splat(16.0)),
    ));
    // let first_ent_id = first_ent.id();
    // last_ent = Some(first_ent_id);
    // for i in 1..=100 {
    //     let pos: Vec2 = Vec2::new(-50.0 + (i as f32) * 4., 0.);
    //     let new = commands.spawn((
    //         Transform::from_xyz(0.0, 0.0, 0.0),
    //         Collider {
    //             radius: 2.0,
    //             layer: 2,
    //             layer_mask: 1,
    //         },
    //         VerletObject {
    //             fixed: false,
    //             position_old: pos,
    //             position_current: pos,
    //             acceleration: Vec2::ZERO,
    //         },
    //         Sprite::from_color(Color::WHITE, Vec2::splat(4.0)),
    //     ));
    //     let new_ent = new.id();
    //     if let Some(last) = last_ent {
    //         commands.spawn(
    //             (Stick {
    //                 ent1: new_ent,
    //                 ent2: last,
    //                 length: 4.0,
    //                 ratio: if i <= 1 { 1.0 } else { 0.5 },
    //             }),
    //         );
    //     }
    //     last_ent = Some(new_ent);
    // }

    commands.spawn((
        VerletObject {
            position_current: Vec2::new(50.0, -300.0),
            position_old: Default::default(),
            acceleration: Default::default(),
            fixed: true,
            ..default()
        },
        Sprite::from_color(Color::BLACK, Vec2::splat(50.0)),
        Transform::from_xyz(0.0, 0.0, -5.0),
        StaticCollider,
        Collider {
            shape: Shape::Box {
                width: 25.0,
                height: 25.0,
            },
            layer: 1,
            layer_mask: 3,
        },
    ));

    commands.spawn((
        VerletObject {
            position_current: Vec2::new(-150.0, 90.0),
            position_old: Default::default(),
            acceleration: Default::default(),
            fixed: true,
            ..default()
        },
        Sprite::from_color(Color::BLACK, Vec2::splat(50.0)),
        Transform::from_xyz(0.0, 0.0, -5.0),
        StaticCollider,
        Collider {
            shape: Shape::Circle { radius: 25.0 },
            layer: 1,
            layer_mask: 3,
        },
    ));
}

#[derive(Component)]
struct MouseFollower;

fn follow_mouse_system(
    camera_query: Single<(&Camera, &GlobalTransform)>,
    windows: Query<&Window>,
    mut verlet_query: Query<(&mut VerletObject), With<MouseFollower>>,
) {
    let (camera, camera_transform) = *camera_query;

    let Ok(window) = windows.get_single() else {
        return;
    };

    let Some(cursor_position) = window.cursor_position() else {
        return;
    };

    // Calculate a world position based on the cursor's position.
    let Ok(point) = camera.viewport_to_world_2d(camera_transform, cursor_position) else {
        return;
    };
    for (mut verlet_object) in verlet_query.iter_mut() {
        verlet_object.position_current = point;
    }
}

#[derive(Component)]
struct RopeSpawner {
    start: Vec2,
    end: Vec2,
    attached_start: Option<Entity>,
    start_length: f32,
    attached_end: Option<Entity>,
    end_length: f32,
    end_fixed: bool,
    shooter: Entity,
}

fn spawn_rope_system(
    mut commands: Commands,
    spawner_query: Query<(&RopeSpawner, Entity)>,
    mut shooter_query: Query<&mut RopeShooter>,
) {
    for (rope_spawner, entity) in spawner_query.iter() {
        let diff = rope_spawner.end - rope_spawner.start;
        let count = (diff.length() / 8.0) as i32;

        let mut last_ent = rope_spawner.attached_start;
        let mut last_pos = rope_spawner.start;

        for i in 1..=count {
            let percent = i as f32 / count as f32;
            let pos = rope_spawner.start.clone().lerp(rope_spawner.end, percent);
            let new = commands.spawn((
                Transform::from_xyz(0.0, 0.0, 0.0),
                Collider {
                    shape: Shape::Circle { radius: 4.0 },
                    layer: 1,
                    layer_mask: 1,
                },
                VerletObject {
                    fixed: i == count && rope_spawner.end_fixed,
                    position_old: pos,
                    position_current: pos,
                    acceleration: Vec2::ZERO,
                    ..default()
                },
                Sprite::from_color(Color::BLACK, Vec2::splat(4.0)),
            ));
            let new_ent = new.id();
            if let Some(last) = last_ent {
                let stick = commands.spawn(
                    (Stick {
                        ent1: new_ent,
                        ent2: last,
                        length: (pos - last_pos).length() * 0.9,
                    }),
                );
                let stick_ent = stick.id();
                if (i == 1) {
                    if let Ok(mut shooter) = shooter_query.get_mut(rope_spawner.shooter) {
                        shooter.connections.push(stick_ent);
                    }
                }
            }
            last_ent = Some(new_ent);
            last_pos = pos;
        }
        if let (Some(end_entity), Some(last_ent)) = (rope_spawner.attached_end, last_ent) {
            commands.spawn(
                (Stick {
                    ent1: end_entity,
                    ent2: last_ent,
                    length: rope_spawner.end_length,
                }),
            );
        }
        commands.entity(entity).despawn();
    }
}

#[derive(Component)]
struct RopeShooter {
    delete_old: bool,
    connections: Vec<Entity>,
}

fn shoot_rope_system(
    mut commands: Commands,
    camera_query: Single<(&Camera, &GlobalTransform)>,
    windows: Query<&Window>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut player_query: Query<(&VerletObject, &mut RopeShooter, Entity)>,
) {
    let mut clear = false;
    let mut shoot = false;
    if (buttons.just_released(MouseButton::Left)) {
        clear = true;
    }
    if (buttons.just_pressed(MouseButton::Left)) {
        clear = true;
        shoot = true;
    }
    if (!clear && !shoot) {
        return;
    }
    let (camera, camera_transform) = *camera_query;

    let Ok(window) = windows.get_single() else {
        return;
    };

    let cursor_position = if let Some(pos) = window.cursor_position() {
        pos
    } else if !shoot {
        Vec2::ZERO
    } else {
        return;
    };

    // Calculate a world position based on the cursor's position.
    let Ok(point) = camera.viewport_to_world_2d(camera_transform, cursor_position) else {
        return;
    };
    for (verlet_object, mut shooter, entity) in player_query.iter_mut() {
        if (clear) {
            for con in shooter.connections.iter() {
                commands.entity(*con).despawn();
            }
            shooter.connections.clear();
        }
        if (shoot) {
            commands.spawn(
                (RopeSpawner {
                    start: verlet_object.position_current,
                    end: point,
                    attached_start: Some(entity),
                    start_length: 0.5,
                    attached_end: None,
                    end_length: 0.0,
                    end_fixed: true,
                    shooter: entity,
                }),
            );
        }
    }
}
