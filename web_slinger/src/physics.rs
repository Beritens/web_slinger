use crate::RopeHolder;
use bevy::app::{App, FixedUpdate, Plugin};
use bevy::ecs::schedule::ScheduleLabel;
use bevy::math::{Vec2, Vec3};
use bevy::prelude::{
    Camera, Component, Entity, FixedPreUpdate, GlobalTransform, IntoSystemConfigs, Query, Single,
    Transform, Window, With, Without, World,
};

#[derive(ScheduleLabel, Debug, Hash, PartialEq, Eq, Clone)]
pub struct SubStepSchedule;

pub struct PhysicsPlugin;

trait Verlet {
    fn accelerate(&mut self, acc: Vec2);
}
#[derive(Component)]
pub struct VerletObject {
    pub position_current: Vec2,
    pub position_old: Vec2,
    pub acceleration: Vec2,
    pub fixed: bool,
    pub drag: f32,
    pub friction: f32,
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
pub struct Stick {
    pub ent1: Entity,
    pub ent2: Entity,
    pub length: f32,
}

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedPreUpdate, (reset_forces));
        app.add_systems(
            FixedUpdate,
            (apply_gravity.before(run_sub_steps), run_sub_steps),
        );

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
    }
}

fn run_sub_steps(world: &mut World) {
    for i in 0..8 {
        world.run_schedule(SubStepSchedule);
    }
}

fn static_collision_system(
    mut collider_query: Query<(&Collider, &mut VerletObject), Without<StaticCollider>>,
    static_collider_query: Query<(&Collider, &VerletObject), With<StaticCollider>>,
) {
    for (collider_a, mut verlet_object_a) in collider_query.iter_mut() {
        for (collider_b, verlet_object_b) in static_collider_query.iter() {
            let (collides, err, norm) =
                calc_collision(&verlet_object_a, &verlet_object_b, collider_a, collider_b);
            // let diff = verlet_object_b.position_current - verlet_object_a.position_current;
            // let max = collider_a.radius + collider_b.radius;
            // let norm = diff.normalize();
            // let err = diff.length() - max;
            if (collides) {
                apply_friction(norm, &mut verlet_object_a);

                verlet_object_a.position_current += err;
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
        let (collides, err, norm) =
            calc_collision(&verlet_object_a, &verlet_object_b, collider_a, collider_b);
        if (collides) {
            let ma = if verlet_object_a.fixed { 0.0 } else { 1.0 };
            let mb = if verlet_object_b.fixed { 0.0 } else { 1.0 };
            if (ma + mb <= 0.0) {
                continue;
            }
            verlet_object_a.position_current += err * ma / (ma + mb);
            verlet_object_b.position_current -= err * mb / (ma + mb);
        }
    }
}

#[derive(Debug)]
pub enum Shape {
    Circle { radius: f32 },
    Box { width: f32, height: f32 },
}
#[derive(Component)]
pub struct StaticCollider;

#[derive(Component)]
pub struct Collider {
    // pub radius: f32,
    pub shape: Shape,
    pub layer: u32,
    pub layer_mask: u32,
}

//returns doesCollide, error-vector, normal-vector

fn circle_circle_collision(pos_a: Vec2, pos_b: Vec2, r_a: f32, r_b: f32) -> (bool, Vec2, Vec2) {
    let diff = pos_b - pos_a;
    let dist = diff.length();
    if (dist > r_a + r_b) {
        return (false, Vec2::ZERO, Vec2::ZERO);
    }
    let diff_norm = diff.clone().normalize();
    let err = diff_norm * (dist - r_a - r_b);
    return (true, err, -diff_norm);
}

fn circle_box_collision(
    pos_a: Vec2,
    pos_b: Vec2,
    radius: f32,
    width: f32,
    height: f32,
) -> (bool, Vec2, Vec2) {
    //get closest corner

    let mut check_axis: Vec<Vec2> = vec![];
    let left = pos_b.x - width;
    let right = pos_b.x + width;
    let closest_x;
    if (pos_a.x - right).abs() < (pos_a.x - left).abs() {
        closest_x = right;
        check_axis.push(Vec2::X);
    } else {
        closest_x = left;
        check_axis.push(Vec2::NEG_X);
    };

    let top = pos_b.y + height;
    let bottom = pos_b.y - height;
    let closest_y;
    if (pos_a.y - top).abs() < (pos_a.y - bottom).abs() {
        closest_y = top;
        check_axis.push(Vec2::Y);
    } else {
        closest_y = bottom;
        check_axis.push(Vec2::NEG_Y);
    };
    let closest_point = Vec2::new(closest_x, closest_y);

    check_axis.push((pos_a - closest_point).normalize());

    let mut depth: f32 = 100000.0;
    let mut norm: Vec2 = Vec2::ZERO;

    let offsets = [
        Vec2::new(width, height),
        Vec2::new(-width, height),
        Vec2::new(width, -height),
        Vec2::new(-width, -height),
    ];

    for axis in check_axis {
        let proj_a = axis.dot(pos_a) - radius;
        let proj_b = offsets
            .iter()
            .map(|&offset| axis.dot(pos_b + offset))
            .fold(f32::NEG_INFINITY, f32::max);

        let axis_depth = proj_b - proj_a;
        if (axis_depth < depth) {
            depth = axis_depth;
            norm = axis;
        }
    }

    return (depth > 0.0, norm * depth, norm);
    //get axises
    //loop over them
    //get points on axis
    //check depth
}
fn calc_collision(
    a_obj: &VerletObject,
    b_obj: &VerletObject,
    a_col: &Collider,
    b_col: &Collider,
) -> (bool, Vec2, Vec2) {
    match (&a_col.shape, &b_col.shape) {
        (Shape::Circle { radius: ra }, Shape::Circle { radius: rb }) => {
            return circle_circle_collision(
                a_obj.position_current,
                b_obj.position_current,
                *ra,
                *rb,
            );
        }
        (
            Shape::Circle { radius: radius },
            Shape::Box {
                width: width,
                height: height,
            },
        ) => {
            return circle_box_collision(
                a_obj.position_current,
                b_obj.position_current,
                *radius,
                *width,
                *height,
            );
        }
        (
            Shape::Box {
                width: width,
                height: height,
            },
            Shape::Circle { radius: radius },
        ) => {
            return circle_box_collision(
                b_obj.position_current,
                a_obj.position_current,
                *radius,
                *width,
                *height,
            );
        }
        _ => {
            return (false, Vec2::ZERO, Vec2::ZERO);
        }
    }
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

fn reset_forces(mut verlet_query: Query<(&mut VerletObject)>) {
    for (mut verlet_object) in verlet_query.iter_mut() {
        verlet_object.acceleration = Vec2::ZERO;
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
