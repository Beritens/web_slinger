use bevy::app::{FixedUpdate, Startup};
use bevy::color::Color;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::{
    App, Camera, Camera2d, Commands, Component, Entity, GlobalTransform, Query, Single, Transform,
    Vec2, Vec3, Window, With, World,
};
use bevy::sprite::Sprite;
use bevy::window::PrimaryWindow;
use bevy::DefaultPlugins;

#[derive(ScheduleLabel, Debug, Hash, PartialEq, Eq, Clone)]
struct SubStepSchedule;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    app.add_systems(Startup, setup);
    app.add_systems(FixedUpdate, (follow_mouse_system, run_sub_steps));
    app.add_systems(
        SubStepSchedule,
        (
            apply_gravity,
            apply_constraints,
            update_verlet_position,
            stick_constraints,
            collision_system,
        ),
    );
    app.run();
}

fn run_sub_steps(world: &mut World) {
    for i in 0..8 {
        world.run_schedule(SubStepSchedule);
    }
}

fn setup(mut commands: Commands) {
    commands.spawn((Camera2d));
    let mut last_ent: Option<Entity> = None;

    let p = Vec2::new(-50.0, 0.0);
    let first_ent = commands.spawn((
        Transform::from_xyz(-50.0, 0.0, 0.0),
        Collider {
            radius: 2.0,
            layer: 2,
            layer_mask: 1,
        },
        VerletObject {
            fixed: true,
            position_old: p,
            position_current: p,
            acceleration: Vec2::ZERO,
        },
        Sprite::from_color(Color::WHITE, Vec2::splat(8.0)),
        MouseFollower,
    ));
    let first_ent_id = first_ent.id();
    last_ent = Some(first_ent_id);
    for i in 1..=100 {
        let pos: Vec2 = Vec2::new(-50.0 + (i as f32) * 4., 0.);
        let new = commands.spawn((
            Transform::from_xyz(0.0, 0.0, 0.0),
            Collider {
                radius: 2.0,
                layer: 2,
                layer_mask: 1,
            },
            VerletObject {
                fixed: false,
                position_old: pos,
                position_current: pos,
                acceleration: Vec2::ZERO,
            },
            Sprite::from_color(Color::WHITE, Vec2::splat(4.0)),
        ));
        let new_ent = new.id();
        if let Some(last) = last_ent {
            commands.spawn(
                (Stick {
                    ent1: new_ent,
                    ent2: last,
                    length: 4.0,
                    ratio: if i <= 1 { 1.0 } else { 0.5 },
                }),
            );
        }
        last_ent = Some(new_ent);
    }

    commands.spawn((
        VerletObject {
            position_current: Vec2::new(50.0, 25.0),
            position_old: Default::default(),
            acceleration: Default::default(),
            fixed: true,
        },
        Sprite::from_color(Color::BLACK, Vec2::splat(50.0)),
        Transform::from_xyz(0.0, 0.0, -5.0),
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
        },
        Sprite::from_color(Color::BLACK, Vec2::splat(50.0)),
        Transform::from_xyz(0.0, 0.0, -5.0),
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
    ratio: f32,
}

#[derive(Component)]
struct MouseFollower;

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
        let vel = verlet_object.position_current - verlet_object.position_old;
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
        let dirr = verlet_object.position_current - origin;
        if (dirr.length() > radius) {
            verlet_object.position_current = origin + dirr.normalize() * radius;
        }
    }
}

fn stick_constraints(stick_query: Query<(&Stick)>, mut verlet_query: Query<&mut VerletObject>) {
    for (mut stick) in stick_query.iter() {
        if let Ok([mut obj1, mut obj2]) = verlet_query.get_many_mut([stick.ent1, stick.ent2]) {
            let diff = obj2.position_current - obj1.position_current;
            let err = diff.length() - stick.length;
            obj1.position_current += diff.normalize() * err * stick.ratio;
            obj2.position_current -= diff.normalize() * err * (1.0 - stick.ratio);
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
    // for (collider_a, mut verlet_object_a) in collider_query.iter_mut() {
    //     for (collider_b, mut verlet_object_b) in collider_query.iter_mut() {}
    // }
}
