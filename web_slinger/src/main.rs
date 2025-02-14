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
use std::process::id;

#[derive(ScheduleLabel, Debug, Hash, PartialEq, Eq, Clone)]
struct SubStepSchedule;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    app.add_systems(Startup, setup);
    app.add_systems(Update, (shoot_rope_system, spawn_rope_system));
    app.add_systems(FixedUpdate, (follow_mouse_system, run_sub_steps));
    app.add_systems(
        SubStepSchedule,
        (
            apply_gravity,
            apply_constraints,
            update_verlet_position,
            stick_constraints,
            static_collision_system, // collision_system,
            mouse_constraint_system,
        ),
    );
    app.run();
}

fn run_sub_steps(world: &mut World) {
    for i in 0..8 {
        world.run_schedule(SubStepSchedule);
    }
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
            radius: 2.0,
            layer: 2,
            layer_mask: 1,
        },
        VerletObject {
            fixed: false,
            position_old: p,
            position_current: p,
            acceleration: Vec2::ZERO,
            friction: 0.5,
            ..default()
        },
        Sprite::from_color(Color::WHITE, Vec2::splat(8.0)),
    ));
    let hand_ent = hand.id();

    commands.spawn((
        Transform::from_xyz(0.0, 0.0, 0.0),
        Collider {
            radius: 8.0,
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
        Sprite::from_color(Color::WHITE, Vec2::splat(16.0)),
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
            position_current: Vec2::new(50.0, 25.0),
            position_old: Default::default(),
            acceleration: Default::default(),
            fixed: true,
            ..default()
        },
        Sprite::from_color(Color::BLACK, Vec2::splat(50.0)),
        Transform::from_xyz(0.0, 0.0, -5.0),
        StaticCollider,
        Collider {
            radius: 25.0,
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
            radius: 25.0,
            layer: 1,
            layer_mask: 3,
        },
    ));
}

trait Verlet {
    fn accelerate(&mut self, acc: Vec2);
}
#[derive(Component)]
struct VerletObject {
    position_current: Vec2,
    position_old: Vec2,
    acceleration: Vec2,
    fixed: bool,
    drag: f32,
    friction: f32,
}

impl Default for VerletObject {
    fn default() -> Self {
        return VerletObject {
            position_current: Vec2::ZERO,
            position_old: Vec2::ZERO,
            acceleration: Vec2::ZERO,
            fixed: false,
            drag: 0.001,
            friction: 0.01,
        };
    }
}

impl Verlet for VerletObject {
    fn accelerate(&mut self, acc: Vec2) {
        self.acceleration += acc;
    }
}

#[derive(Component)]
struct Stick {
    ent1: Entity,
    ent2: Entity,
    length: f32,
}

#[derive(Component)]
struct MouseFollower;

#[derive(Component)]
struct StaticCollider;

#[derive(Component)]
struct Collider {
    radius: f32,
    layer: u32,
    layer_mask: u32,
}

fn update_verlet_position(mut verlet_query: Query<(&mut VerletObject, &mut Transform)>) {
    for (mut verlet_object, mut transform) in verlet_query.iter_mut() {
        if verlet_object.fixed {
            transform.translation = Vec3::new(
                verlet_object.position_current.x,
                verlet_object.position_current.y,
                transform.translation.z,
            );
            continue;
        }
        let vel = (verlet_object.position_current - verlet_object.position_old)
            * (1.0 - verlet_object.drag);
        verlet_object.position_old = verlet_object.position_current;
        verlet_object.position_current =
            verlet_object.position_old + vel + verlet_object.acceleration;
        verlet_object.acceleration = Vec2::ZERO;
        transform.translation = Vec3::new(
            verlet_object.position_current.x,
            verlet_object.position_current.y,
            transform.translation.z,
        );
    }
}

fn apply_gravity(mut verlet_query: Query<(&mut VerletObject)>) {
    for (mut verlet_object) in verlet_query.iter_mut() {
        verlet_object.accelerate(-Vec2::Y * 0.01);
    }
}

fn apply_constraints(mut verlet_query: Query<&mut VerletObject>) {
    const origin: Vec2 = Vec2::ZERO;
    const radius: f32 = 350.0;
    for (mut verlet_object) in verlet_query.iter_mut() {
        if (verlet_object.position_current.y < -400.0) {
            let normal = Vec2::Y;

            apply_friction(normal, &mut verlet_object);
            verlet_object.position_current.y = -400.0;
        }
        // let dirr = verlet_object.position_current - origin;
        // if (dirr.length() > radius) {
        //     verlet_object.position_current = origin + dirr.normalize() * radius;
        // }
    }
}

fn apply_friction(normal: Vec2, verlet_object: &mut VerletObject) -> Vec2 {
    let vel = verlet_object.position_current - verlet_object.position_old;
    let vel_n = normal * normal.dot(vel);
    let vel_t = vel - vel_n;
    verlet_object.position_current -= vel_t * verlet_object.friction;
    return vel_t;
}

fn stick_constraints(stick_query: Query<(&Stick)>, mut verlet_query: Query<&mut VerletObject>) {
    for (mut stick) in stick_query.iter() {
        if let Ok([mut obj1, mut obj2]) = verlet_query.get_many_mut([stick.ent1, stick.ent2]) {
            let diff = obj2.position_current - obj1.position_current;
            let err = diff.length() - stick.length;

            let ma = if obj1.fixed { 0.0 } else { 1.0 };
            let mb = if obj2.fixed { 0.0 } else { 1.0 };
            if (ma + mb <= 0.0) {
                continue;
            }
            obj1.position_current += diff.normalize() * err * (ma / (ma + mb));
            obj2.position_current -= diff.normalize() * err * (mb / (ma + mb));
        }
    }
}

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

fn static_collision_system(
    mut collider_query: Query<(&Collider, &mut VerletObject), Without<StaticCollider>>,
    static_collider_query: Query<(&Collider, &VerletObject), With<StaticCollider>>,
) {
    for (collider_a, mut verlet_object_a) in collider_query.iter_mut() {
        for (collider_b, verlet_object_b) in static_collider_query.iter() {
            let diff = verlet_object_b.position_current - verlet_object_a.position_current;
            let max = collider_a.radius + collider_b.radius;
            let norm = diff.normalize();
            let err = diff.length() - max;
            if (err < 0.0) {
                apply_friction(norm, &mut verlet_object_a);

                verlet_object_a.position_current += norm * err;
            }
        }
    }
}
fn collision_system(mut collider_query: Query<(&Collider, &mut VerletObject)>) {
    let mut combinations = collider_query.iter_combinations_mut();
    while let Some([(collider_a, mut verlet_object_a), (collider_b, mut verlet_object_b)]) =
        combinations.fetch_next()
    {
        if (collider_a.layer_mask & collider_b.layer == 0
            || collider_b.layer_mask & collider_a.layer == 0)
        {
            continue;
        }
        let diff = verlet_object_b.position_current - verlet_object_a.position_current;
        let max = collider_a.radius + collider_b.radius;
        let norm = diff.normalize();
        let err = diff.length() - max;
        if (err < 0.0) {
            let ma = if verlet_object_a.fixed { 0.0 } else { 1.0 };
            let mb = if verlet_object_b.fixed { 0.0 } else { 1.0 };
            if (ma + mb <= 0.0) {
                continue;
            }
            verlet_object_a.position_current += norm * err * ma / (ma + mb);
            verlet_object_b.position_current -= norm * err * mb / (ma + mb);
        }
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
                    radius: 4.0,
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
                Sprite::from_color(Color::WHITE, Vec2::splat(4.0)),
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

fn mouse_constraint_system(
    camera_query: Single<(&Camera, &GlobalTransform)>,
    windows: Query<&Window>,
    mut player_query: Query<(&mut RopeHolder, Entity)>,
    mut verlet_object_query: Query<&mut VerletObject>,
) {
    let (camera, camera_transform) = *camera_query;

    let Ok(window) = windows.get_single() else {
        return;
    };

    let cursor_position = window.cursor_position();

    // Calculate a world position based on the cursor's position.
    let point =
        cursor_position.and_then(|pos| camera.viewport_to_world_2d(camera_transform, pos).ok());

    for (mut rope_holer, entity) in player_query.iter_mut() {
        if let Ok([mut obj1, mut obj2]) =
            verlet_object_query.get_many_mut([entity, rope_holer.hand])
        {
            let mouse_pos = point.unwrap_or(rope_holer.last_pos);
            let diff_obj2 = (mouse_pos - obj2.position_current);
            // let target_position = obj1.position_current - diff_obj2;
            // let diff_to_target = target_position - obj1.position_current;
            let length = diff_obj2.length().min(64.0);
            let ideal_pos = obj1.position_current + diff_obj2.normalize() * length;
            // let ideal_pos = obj1.position_current - Vec2::Y * 50.0;
            let diff = ideal_pos - obj2.position_current;
            if (diff.length() == 0.0) {
                continue;
            }
            let diff_norm = diff.clone().normalize();

            obj2.position_current += diff_norm * diff.length().min(0.5) * 0.95;

            obj1.position_current -= diff_norm * diff.length().min(0.5) * 0.05;

            let hand_diff = obj2.position_current - obj1.position_current;
            let hand_diff_norm = hand_diff.clone().normalize();
            let err = 64.0 - hand_diff.length();
            if (err < 0.0) {
                obj1.position_current -= hand_diff_norm * err * 0.05;
                obj2.position_current += hand_diff_norm * err * 0.95;
            }
            rope_holer.last_pos = mouse_pos;

            if (obj1.position_current.is_nan()) {
                println!("nan");
            }
            // obj2.position_current += diff / 2.0;
        }
    }
}
