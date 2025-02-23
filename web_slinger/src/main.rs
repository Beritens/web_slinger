mod collider_import;
mod color_picker;
mod physics;
mod rope_shooting;

use crate::collider_import::CollisionImportPlugin;
use crate::color_picker::{ColorPickerPlugin, GlobalColor};
use crate::physics::{
    raycast, Collider, Collision, CollisionWorld, ConstantFriction, PhysicsPlugin, Ray, Shape,
    StaticCollider, Stick, SubStepSchedule, TrackCollision, VerletObject,
};
use crate::rope_shooting::{RopeShooter, RopeShootingPlugin};
use bevy::app::{FixedUpdate, Startup};
use bevy::color::Color;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::{
    App, ButtonInput, Camera, Camera2d, Changed, ClearColor, Commands, Component, Entity,
    GlobalTransform, IntoSystemConfigs, MouseButton, PluginGroup, Query, Res, ResMut, Resource,
    Single, Transform, Update, Vec2, Vec3, Window, With, Without, World,
};
use bevy::sprite::Sprite;
use bevy::utils::default;
use bevy::utils::hashbrown::HashMap;
use bevy::window::{CompositeAlphaMode, PrimaryWindow, WindowLevel, WindowPlugin};
use bevy::DefaultPlugins;
use bevy_wasm_window_resize::WindowResizePlugin;
use std::process::id;
use std::slice::Windows;
use std::sync::{Arc, Mutex};
use wasm_bindgen::prelude::wasm_bindgen;

fn main() {
    println!("Web_Slinger activated");
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            #[cfg(target_arch = "wasm32")]
            canvas: Some("#bevy".into()),
            #[cfg(target_arch = "wasm32")]
            prevent_default_event_handling: false,
            ..default()
        }),
        ..default()
    }));
    app.add_plugins(ColorPickerPlugin);
    app.add_systems(Startup, setup);
    app.add_plugins(WindowResizePlugin);
    app.add_plugins(CollisionImportPlugin);
    app.add_plugins(PhysicsPlugin);
    app.add_plugins(RopeShootingPlugin);

    #[cfg(target_arch = "wasm32")]
    app.insert_resource(ClearColor(Color::NONE));
    app.add_systems(FixedUpdate, (follow_mouse_system));
    app.add_systems(
        Update,
        (
            update_scroll_pos.before(align_camera_origin),
            align_camera_origin,
        ),
    );
    app.insert_resource(ScrollPosition { x: 0, y: 0 });
    app.run();
}

#[derive(Component)]
struct Player;

#[derive(Component)]
struct RopeHolder {
    last_pos: Vec2,
    hand: Entity,
    power: f32,
}

fn align_camera_origin(
    windows: Query<&Window>,
    scroll_position: Res<ScrollPosition>,
    mut transforms: Query<&mut Transform, With<Camera>>,
) {
    let Ok(window) = windows.get_single() else {
        return;
    };
    let Ok(mut transform) = transforms.get_single_mut() else {
        return;
    };

    transform.translation.x = window.width() / 2.0 + scroll_position.x as f32;
    transform.translation.y = -window.height() / 2.0 - scroll_position.y as f32;
}

fn setup(mut commands: Commands, global_color: Res<GlobalColor>) {
    let mut camera = Camera2d;

    commands.spawn((Camera2d));
    let mut last_ent: Option<Entity> = None;

    let p = Vec2::new(800.0, -50.0);
    let hand = commands.spawn((
        Transform::from_xyz(p.x, p.y, 0.0),
        RopeShooter {
            delete_old: true,
            connections: vec![],
        },
        Collider {
            shape: Shape::Circle { radius: 4.0 },
            layer: 2,
            layer_mask: 1,
        },
        TrackCollision {
            collisions: HashMap::new(),
            last: HashMap::new(),
        },
        ConstantFriction,
        VerletObject {
            fixed: false,
            position_old: p,
            position_current: p,
            acceleration: Vec2::ZERO,
            friction: 0.8,
            ..default()
        },
        Sprite::from_color(global_color.color, Vec2::splat(8.0)),
    ));
    let hand_ent = hand.id();

    commands.spawn((
        Transform::from_xyz(p.x, p.y, 0.0),
        Player,
        TrackCollision {
            collisions: Default::default(),
            last: Default::default(),
        },
        Collider {
            shape: Shape::Circle { radius: 8.0 },
            layer: 2,
            layer_mask: 1,
        },
        RopeHolder {
            power: 0.4,
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
        Sprite::from_color(global_color.color, Vec2::splat(16.0)),
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

    // commands.spawn((
    //     VerletObject {
    //         position_current: Vec2::new(250.0, -400.0),
    //         position_old: Default::default(),
    //         acceleration: Default::default(),
    //         fixed: true,
    //         ..default()
    //     },
    //     Sprite::from_color(Color::BLACK, Vec2::splat(50.0)),
    //     Transform::from_xyz(0.0, 0.0, -5.0),
    //     StaticCollider,
    //     Collider {
    //         shape: Shape::Box {
    //             width: 25.0,
    //             height: 25.0,
    //         },
    //         layer: 1,
    //         layer_mask: 3,
    //     },
    // ));
    //
    // commands.spawn((
    //     VerletObject {
    //         position_current: Vec2::new(400.0, -300.0),
    //         position_old: Default::default(),
    //         acceleration: Default::default(),
    //         fixed: true,
    //         ..default()
    //     },
    //     Sprite::from_color(Color::BLACK, Vec2::splat(50.0)),
    //     Transform::from_xyz(0.0, 0.0, -5.0),
    //     StaticCollider,
    //     Collider {
    //         shape: Shape::Circle { radius: 25.0 },
    //         layer: 1,
    //         layer_mask: 3,
    //     },
    // ));
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

#[derive(Resource)]
struct ScrollPosition {
    x: i32,
    y: i32,
}

static SCROLL_Y: once_cell::sync::Lazy<Arc<Mutex<i32>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(0)));
static SCROLL_X: once_cell::sync::Lazy<Arc<Mutex<i32>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(0)));
#[wasm_bindgen]
pub fn set_scroll_pos(scroll_y: i32, scroll_x: i32) {
    if let Ok(mut y) = SCROLL_Y.lock() {
        *y = scroll_y;
    }
    if let Ok(mut x) = SCROLL_X.lock() {
        *x = scroll_x;
    }
}

fn update_scroll_pos(mut pos_res: ResMut<ScrollPosition>) {
    if let Ok(x) = SCROLL_X.lock() {
        pos_res.x = x.clone();
    }
    if let Ok(y) = SCROLL_Y.lock() {
        pos_res.y = y.clone();
    }
}
