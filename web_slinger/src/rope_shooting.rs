use crate::collider_import::CollisionImportPlugin;
use crate::color_picker::GlobalColor;
use crate::physics::{
    raycast, Collider, CollisionSetup, CollisionWorld, Ray, Shape, Stick, VerletObject,
};
use crate::{align_camera_origin, update_scroll_pos, RopeHolder};
use bevy::app::{App, Plugin, Startup, Update};
use bevy::input::ButtonInput;
use bevy::math::Vec2;
use bevy::prelude::{
    default, Camera, Commands, Component, Entity, GlobalTransform, MouseButton, Query, Res, Single,
    Sprite, Transform, Window,
};

pub struct RopeShootingPlugin;
impl Plugin for RopeShootingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (shoot_rope_system, spawn_rope_system));
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
    global_color: Res<GlobalColor>,
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
                    trigger: false,
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
                Sprite::from_color(global_color.color, Vec2::splat(4.0)),
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
pub struct RopeShooter {
    pub delete_old: bool,
    pub connections: Vec<Entity>,
}

#[derive(Component)]
pub struct Hookable;

fn shoot_rope_system(
    mut commands: Commands,
    camera_query: Single<(&Camera, &GlobalTransform)>,
    windows: Query<&Window>,
    buttons: Res<ButtonInput<MouseButton>>,
    player_query: Query<(&VerletObject, &RopeHolder)>,
    mut hand_query: Query<(&VerletObject, &mut RopeShooter)>,
    collision_world: Res<CollisionWorld>,
    collider_query: Query<(&Collider, &VerletObject)>,
    hookable_query: Query<&Hookable>,
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
    for (player_object, rope_shooter) in player_query.iter() {
        let entity = rope_shooter.hand;
        if let Ok((verlet_object, mut shooter)) = hand_query.get_mut(entity) {
            if (clear) {
                for con in shooter.connections.iter() {
                    commands.entity(*con).despawn();
                }
                shooter.connections.clear();
            }
            if (shoot) {
                let ray = Ray {
                    origin: verlet_object.position_current,
                    direction: (point - player_object.position_current).normalize(),
                };
                let hit = raycast(&ray, &collider_query, &collision_world);
                if let Some(hit) = hit {
                    if let Ok(hookedEnt) = hookable_query.get(hit.1) {
                        let pos = ray.origin + hit.0 * ray.direction;
                        commands.spawn(
                            (RopeSpawner {
                                start: verlet_object.position_current,
                                end: pos,
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
        }
    }
}
