use crate::RopeHolder;
use bevy::app::{App, FixedUpdate, Plugin, Startup};
use bevy::ecs::schedule::ScheduleLabel;
use bevy::input::mouse::MouseMotion;
use bevy::math::{Vec2, Vec3};
use bevy::prelude::{
    Camera, Commands, Component, Entity, EventReader, FixedPreUpdate, FloatExt, GlobalTransform,
    IntoSystemConfigs, Query, Res, ResMut, Resource, Single, Sprite, Srgba, SystemSet, Time,
    Transform, Update, Window, With, Without, World,
};
use bevy::utils::HashMap;
use std::sync::{Arc, Mutex};

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
            friction: 0.1,
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

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct CollisionSetup;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct PhysicsSet;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedPreUpdate, (reset_forces, reset_collisions));
        app.add_systems(
            Startup,
            (
                // display_collision_tree.after(build_collision_tree),
                (build_collision_tree).in_set(CollisionSetup),
            ),
        );

        // app.add_systems(Update, adjust_power_system);
        app.add_systems(
            FixedUpdate,
            (
                apply_gravity.before(run_sub_steps),
                run_sub_steps.in_set(PhysicsSet),
            ),
        );
        app.insert_resource(CollisionWorld { kd_tree: None });

        app.add_systems(
            SubStepSchedule,
            (
                // apply_constraints,
                update_verlet_position,
                stick_constraints,
                static_collision_system, // collision_system,
                mouse_constraint_system.before(stick_constraints),
                constant_friction_system.after(stick_constraints),
            ),
        );
    }
}

fn run_sub_steps(world: &mut World) {
    for i in 0..8 {
        world.run_schedule(SubStepSchedule);
    }
}

#[derive(Resource)]
pub struct CollisionWorld {
    pub kd_tree: Option<Arc<Mutex<KDNode>>>,
}

#[derive(Clone)]
pub struct AABB {
    pub pos: Vec2,
    pub size: Vec2,
}

pub fn line_line_intersection(a1: Vec2, a2: Vec2, b1: Vec2, b2: Vec2, a_inf: bool) -> (bool, f32) {
    let d0 = (a2 - a1);
    let d1 = (b2 - b1);

    let a = d0.dot(d0);
    let b = d0.dot(d1);
    let c = d1.dot(d1);

    let d = (d0.dot(a1 - b1));
    let e = (d1.dot(a1 - b1));

    let num = a * c - b * b;
    if num.abs() < 1e-6 {
        return (false, f32::INFINITY);
    }

    let s = (b * e - c * d) / (num);
    let t = (a * e - b * d) / (num);
    if (s >= 0.0 && (s <= 1.0 || a_inf) && t >= 0.0 && t <= 1.0) {
        return (true, s);
    }
    return (false, f32::INFINITY);
}
impl AABB {
    fn intersects(&self, other: &AABB) -> bool {
        let min_x_a = self.pos.x;
        let min_y_a = self.pos.y;
        let min_x_b = other.pos.x;
        let min_y_b = other.pos.y;

        let max_x_a = self.pos.x + self.size.x;
        let max_y_a = self.pos.y + self.size.y;
        let max_x_b = other.pos.x + other.size.x;
        let max_y_b = other.pos.y + other.size.y;

        return (min_x_a <= max_x_b
            && max_x_a >= min_x_b
            && min_y_a <= max_y_b
            && max_y_a >= min_y_b);
    }
    fn intersect_ray(&self, ray: &Ray) -> (bool, f32) {
        let min_x = self.pos.x;
        let min_y = self.pos.y;
        let max_x = self.pos.x + self.size.x;
        let max_y = self.pos.y + self.size.y;

        let cor_a = Vec2::new(min_x, min_y);
        let cor_b = Vec2::new(min_x, max_y);
        let cor_c = Vec2::new(max_x, max_y);
        let cor_d = Vec2::new(max_x, min_y);

        let (a_hit, a_dist) =
            line_line_intersection(ray.origin, ray.origin + ray.direction, cor_a, cor_b, true);

        let (b_hit, b_dist) =
            line_line_intersection(ray.origin, ray.origin + ray.direction, cor_b, cor_c, true);

        let (c_hit, c_dist) =
            line_line_intersection(ray.origin, ray.origin + ray.direction, cor_c, cor_d, true);

        let (d_hit, d_dist) =
            line_line_intersection(ray.origin, ray.origin + ray.direction, cor_d, cor_a, true);

        return (
            a_hit || b_hit || c_hit || d_hit,
            a_dist.min(b_dist).min(c_dist).min(d_dist),
        );
    }
}
pub struct KDNode {
    bounding_box: AABB,
    left_node: Option<Arc<Mutex<KDNode>>>,
    right_node: Option<Arc<Mutex<KDNode>>>,
    objects: Vec<Entity>,
}

#[derive(Clone)]
struct ColliderObj {
    entity: Entity,
    bounding_box: AABB,
}
fn kd_tree(objects: &mut Vec<ColliderObj>, depth: usize) -> Option<Arc<Mutex<KDNode>>> {
    if (objects.len() <= 0) {
        return None;
    }

    if (objects.len() == 1) {
        let obj = objects.pop().unwrap();
        return Some(Arc::new(Mutex::new(KDNode {
            bounding_box: obj.bounding_box,
            left_node: None,
            right_node: None,
            objects: vec![obj.entity],
        })));
    }

    let axis = depth % 2;

    objects.sort_by(|a, b| {
        if axis == 0 {
            a.bounding_box
                .pos
                .x
                .partial_cmp(&b.bounding_box.pos.x)
                .unwrap()
        } else {
            a.bounding_box
                .pos
                .y
                .partial_cmp(&b.bounding_box.pos.y)
                .unwrap()
        }
    });

    let mid = objects.len() / 2;

    let mut right_objects = objects.split_off(mid);
    let left_objects = objects;

    let left_node = kd_tree(left_objects, depth + 1);
    let right_node = kd_tree(&mut right_objects, depth + 1);
    let bounding_box: AABB;
    if let (Some(l), Some(r)) = (left_node.as_ref(), right_node.as_ref()) {
        let bl = l.lock().unwrap().bounding_box.clone();
        let br = r.lock().unwrap().bounding_box.clone();
        bounding_box = combine_bounding_boxes(bl, br);
    } else if let Some(l) = left_node.as_ref() {
        bounding_box = l.lock().unwrap().bounding_box.clone();
    } else if let Some(r) = right_node.as_ref() {
        bounding_box = r.lock().unwrap().bounding_box.clone();
    } else {
        bounding_box = AABB {
            pos: Default::default(),
            size: Default::default(),
        };
    }

    Some(Arc::new(Mutex::new(KDNode {
        bounding_box,
        left_node,
        right_node,
        objects: vec![],
    })))

    // let bounding_box = combine_bounding_boxes(left_node, right_node);
    // let node = Node {
    //     left_node: left_node,
    //     right_node: right_node,
    //     bounding_box: bounding_box,
    //     objects: vec![],
    // };

    // sort stuff
    // choose mid point
    // call kd_tree() with one half and other dimension
    // call kd_tree() with other half and other dimension
    // combine bounding boxes to get bounding box of current node
}
fn combine_bounding_boxes(left: AABB, right: AABB) -> AABB {
    // You would want to merge the AABBs from both sides (left and right) here

    // Combine AABBs to cover both regions
    AABB {
        pos: Vec2 {
            x: left.pos.x.min(right.pos.x),
            y: left.pos.y.min(right.pos.y),
        },
        size: Vec2 {
            x: (left.pos.x + left.size.x).max(right.pos.x + right.size.x)
                - (left.pos.x).min(right.pos.x),
            y: (left.pos.y + left.size.y).max(right.pos.y + right.size.y)
                - (left.pos.y).min(right.pos.y),
        },
    }
}
fn build_collision_tree(
    collider_query: Query<(&Collider, &VerletObject, Entity), With<StaticCollider>>,
    mut collision_world_resource: ResMut<CollisionWorld>,
) {
    let mut objects: Vec<ColliderObj> = vec![];
    for (collider, verlet_obj, entity) in collider_query.iter() {
        let bounds: AABB;
        match collider.shape {
            Shape::Circle { radius } => {
                return;
            }
            Shape::Box { width, height } => {
                bounds = AABB {
                    pos: verlet_obj.position_current - Vec2::new(width, height),
                    size: Vec2::new(width * 2.0, height * 2.0),
                };
            }
        }
        let obj = ColliderObj {
            bounding_box: bounds,
            entity,
        };
        objects.push(obj);
    }

    let tree: Option<Arc<Mutex<KDNode>>> = kd_tree(&mut objects, 0);

    collision_world_resource.kd_tree = tree.clone();
    //go over each collider
    //get bounding box of collider
    //call kd_tree() with all collider-bounding box pairs
}

fn display_sub_tree(node: &KDNode, commands: &mut Commands, depth: usize) {
    if (depth > 5) {
        return;
    }
    if (depth == 5) {
        let color = Srgba {
            red: 0.4,
            green: 1.0,
            blue: 1.0,
            alpha: 0.01,
        };
        commands.spawn((
            Transform::from_xyz(
                node.bounding_box.pos.x + node.bounding_box.size.x / 2.0,
                node.bounding_box.pos.y + node.bounding_box.size.y / 2.0,
                5.0 + depth as f32 / 2.0,
            ),
            Sprite::from_color(
                color,
                Vec2::new(node.bounding_box.size.x, node.bounding_box.size.y),
            ),
        ));
    }

    if let Some(ref node) = node.left_node {
        let kd_node = node.lock().unwrap();
        display_sub_tree(&kd_node, commands, depth + 1);
    }

    if let Some(ref node) = node.right_node {
        let kd_node = node.lock().unwrap();
        display_sub_tree(&kd_node, commands, depth + 1);
    }
}
fn display_collision_tree(mut commands: Commands, col_world: Res<CollisionWorld>) {
    if let Some(ref node) = col_world.kd_tree {
        let kd_node = node.lock().unwrap();
        display_sub_tree(&kd_node, &mut commands, 0);
    }
}

fn find_collision_entities(
    bounding_box: &AABB,
    tree: &Option<Arc<Mutex<KDNode>>>,
    colliders: &mut Vec<Entity>,
) {
    if let Some(node) = tree {
        let node = node.lock().unwrap();
        if (!bounding_box.intersects(&node.bounding_box)) {
            return;
        }
        find_collision_entities(bounding_box, &node.left_node.clone(), colliders);
        find_collision_entities(bounding_box, &node.right_node.clone(), colliders);
        colliders.extend(&node.objects);
    }
}

pub struct Ray {
    pub origin: Vec2,
    pub direction: Vec2,
}

fn find_ray_collision_entities(
    ray: &Ray,
    tree: &Option<Arc<Mutex<KDNode>>>,
    colliders: &mut Vec<(Entity, f32)>,
) {
    if let Some(node) = tree {
        let node = node.lock().unwrap();
        let (possible_hit, dist) = node.bounding_box.intersect_ray(ray);
        if (!possible_hit) {
            return;
        }
        find_ray_collision_entities(ray, &node.left_node.clone(), colliders);
        find_ray_collision_entities(ray, &node.right_node.clone(), colliders);
        for &object in &node.objects {
            colliders.push((object, dist));
        }
    }
}

fn static_collision_system(
    mut collider_query: Query<
        (&Collider, &mut VerletObject, Option<&mut TrackCollision>),
        Without<StaticCollider>,
    >,
    kd_tree: Res<CollisionWorld>,
    static_collider_query: Query<(&Collider, &VerletObject, Entity), With<StaticCollider>>,
) {
    for (collider_a, mut verlet_object_a, mut tracker) in collider_query.iter_mut() {
        let bounding_box: AABB = collider_a.get_bounding_box(verlet_object_a.position_current);
        let mut colliders = vec![];
        find_collision_entities(&bounding_box, &kd_tree.kd_tree, &mut colliders);
        for col_ent in colliders {
            if let Ok((collider_b, verlet_object_b, ent)) = static_collider_query.get(col_ent) {
                let (collides, err, norm) =
                    calc_collision(&verlet_object_a, &verlet_object_b, collider_a, collider_b);

                if (collides) {
                    apply_friction(norm, &mut verlet_object_a);

                    verlet_object_a.position_current += err;

                    if let Some(mut tracker_a) = tracker.take() {
                        tracker_a.collisions.insert(ent, Collision { normal: norm });
                    }
                }
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

impl Collider {
    fn intersect_ray(&self, ray: &Ray, pos: Vec2) -> (bool, f32) {
        match self.shape {
            Shape::Box { width, height } => return self.get_bounding_box(pos).intersect_ray(ray),

            Shape::Circle { radius } => {
                return (false, 0.0);
            }
        }
    }
    fn get_bounding_box(&self, pos: Vec2) -> AABB {
        match self.shape {
            Shape::Box { width, height } => {
                return AABB {
                    pos: Vec2::new(pos.x - width, pos.y - height),
                    size: Vec2::new(width * 2.0, height * 2.0),
                };
            }
            Shape::Circle { radius } => {
                return AABB {
                    pos: Vec2::new(pos.x - radius, pos.y - radius),
                    size: Vec2::new(radius * 2.0, radius * 2.0),
                }
            }
        }
    }
}

pub struct Collision {
    pub normal: Vec2,
}
#[derive(Component)]
pub struct TrackCollision {
    pub collisions: HashMap<Entity, Collision>,
    pub last: HashMap<Entity, Collision>,
}

#[derive(Component)]
pub struct ConstantFriction;

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
    let left = pos_b.x - width;
    let right = pos_b.x + width;
    let closest_x;
    let axis1;
    if (pos_a.x - right).abs() < (pos_a.x - left).abs() {
        closest_x = right;
        axis1 = Vec2::X;
    } else {
        closest_x = left;
        axis1 = Vec2::NEG_X;
    };

    let top = pos_b.y + height;
    let bottom = pos_b.y - height;
    let closest_y;
    let axis2;
    if (pos_a.y - top).abs() < (pos_a.y - bottom).abs() {
        axis2 = Vec2::Y;
        closest_y = top;
    } else {
        axis2 = Vec2::NEG_Y;
        closest_y = bottom;
    };
    let closest_point = Vec2::new(closest_x, closest_y);

    let check_axis = [(pos_a - closest_point).normalize(), axis1, axis2];

    let mut depth: f32 = f32::INFINITY;
    let mut norm: Vec2 = Vec2::ZERO;

    let offsets = [
        Vec2::new(width, height),
        Vec2::new(-width, height),
        Vec2::new(width, -height),
        Vec2::new(-width, -height),
    ];

    for axis in check_axis {
        let proj_a = axis.dot(pos_a) - radius;
        let mut proj_b_min = f32::INFINITY;
        let mut proj_b_max = f32::NEG_INFINITY;
        for &offset in &[
            Vec2::new(width, height),
            Vec2::new(-width, height),
            Vec2::new(width, -height),
            Vec2::new(-width, -height),
        ] {
            let projection = axis.dot(pos_b + offset);
            proj_b_min = proj_b_min.min(projection);
            proj_b_max = proj_b_max.max(projection);
        }

        let axis_depth = proj_b_max - proj_a;
        if (axis_depth < depth) {
            depth = axis_depth;
            norm = axis;
        }
        if depth < 0.0 {
            return (false, Vec2::ZERO, Vec2::ZERO); // Early exit if no collision
        }
    }

    return (depth > 0.0, norm * depth, norm);
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
        // verlet_object.acceleration = Vec2::ZERO;
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

fn reset_collisions(mut collision_query: Query<(&mut TrackCollision)>) {
    for (mut col) in collision_query.iter_mut() {
        col.last = std::mem::take(&mut col.collisions);
        // col.collisions.clear();
    }
}

fn apply_constraints(mut verlet_query: Query<&mut VerletObject>) {
    const origin: Vec2 = Vec2::ZERO;
    const radius: f32 = 350.0;
    for (mut verlet_object) in verlet_query.iter_mut() {
        if (verlet_object.position_current.y < -800.0) {
            let normal = Vec2::Y;

            apply_friction(normal, &mut verlet_object);
            verlet_object.position_current.y = -800.0;
        }
        // let dirr = verlet_object.position_current - origin;
        // if (dirr.length() > radius) {
        //     verlet_object.position_current = origin + dirr.normalize() * radius;
        // }
    }
}

fn constant_friction_system(
    mut verlet_query: Query<
        (
            &mut VerletObject,
            &Collider,
            &TrackCollision,
            &mut Transform,
        ),
        With<ConstantFriction>,
    >,
) {
    for (mut verlet_object, _collider, track_collision, mut transform) in verlet_query.iter_mut() {
        //using last collisions to avoid inconsistent friction due to jitter
        for col in &track_collision.last {
            apply_friction(col.1.normal, &mut verlet_object);
        }
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

fn adjust_power_system(
    time: Res<Time>,
    mut evr_motion: EventReader<MouseMotion>,
    mut rope_holder_query: Query<&mut RopeHolder>,
) {
    let mut mouse_move: Vec2 = Vec2::ZERO;
    for ev in evr_motion.read() {
        mouse_move += ev.delta;
    }

    let strength = 0.25.lerp(
        0.5,
        (mouse_move.length() * 0.01 / time.delta_secs()).min(1.0),
    );

    for (mut rope_holder) in rope_holder_query.iter_mut() {
        rope_holder.power = 0.5;
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

            obj2.position_current += diff_norm * (diff.length() / 8.0).min(rope_holer.power) * 0.95;

            obj1.position_current -= diff_norm * (diff.length() / 8.0).min(rope_holer.power) * 0.05;

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

pub fn raycast(
    ray: &Ray,
    collider_query: &Query<(&Collider, &VerletObject)>,
    collision_world: &Res<CollisionWorld>,
) -> Option<(f32, Entity)> {
    let mut objects: Vec<(Entity, f32)> = vec![];
    find_ray_collision_entities(ray, &collision_world.kd_tree, &mut objects);
    if objects.len() == 0 {
        return None;
    }

    objects.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    let mut curr_ent: Option<Entity> = None;
    let mut cur_dist = f32::INFINITY;

    for i in 0..objects.len() {
        let (ent, dist) = objects[i];
        if dist > cur_dist {
            break;
        }
        if let Ok((collider, verlet_obj)) = collider_query.get(ent) {
            let (hit, hit_dist) = collider.intersect_ray(ray, verlet_obj.position_current);
            if (!hit) {
                continue;
            }
            if (hit_dist > cur_dist) {
                continue;
            }
            cur_dist = hit_dist;
            curr_ent = Some(ent);
        }
    }
    if let Some(ent) = curr_ent {
        return Some((cur_dist, ent));
    }
    return None;
}
