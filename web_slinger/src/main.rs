mod collider_import;
mod color_picker;
mod physics;
mod rope_shooting;
mod timer;

use crate::collider_import::CollisionImportPlugin;
use crate::color_picker::{ColorPickerPlugin, GlobalColor};
use crate::physics::{
    raycast, Collider, Collision, CollisionWorld, ConstantFriction, PhysicsPlugin, Position, Ray,
    Shape, StaticCollider, Stick, SubStepSchedule, TrackCollision, VerletObject,
};
use crate::rope_shooting::{RopeShooter, RopeShootingPlugin};
use crate::timer::{StateValue, TimerPlugin};
use bevy::app::{FixedUpdate, Startup};
use bevy::color::Color;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::input::mouse::MouseMotion;
use bevy::prelude::{
    Alpha, App, Button, ButtonInput, Camera, Camera2d, Changed, ClearColor, Commands, Component,
    Entity, EventReader, GlobalTransform, IntoSystemConfigs, KeyCode, MouseButton, PluginGroup,
    Query, Res, ResMut, Resource, Single, Transform, Update, Vec2, Vec3, Vec3Swizzles, Visibility,
    Window, With, Without, World,
};
use bevy::sprite::Sprite;
use bevy::utils::default;
use bevy::utils::hashbrown::HashMap;
use bevy::window::{
    CompositeAlphaMode, CursorGrabMode, CursorOptions, PrimaryWindow, WindowFocused, WindowLevel,
    WindowPlugin,
};
use bevy::winit::cursor;
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
            cursor_options: CursorOptions {
                visible: true,
                grab_mode: CursorGrabMode::None,
                ..Default::default()
            },
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
    app.add_systems(
        Update,
        (
            mouse_motion_test,
            cursor_indicator_system,
            update_cursor_mode,
            relock_cursor,
            handle_window_focus,
        ),
    );
    app.add_plugins(WindowResizePlugin);
    app.add_plugins(TimerPlugin);
    app.add_plugins(CollisionImportPlugin);
    app.add_plugins(PhysicsPlugin);
    app.add_plugins(RopeShootingPlugin);

    #[cfg(target_arch = "wasm32")]
    app.insert_resource(ClearColor(Color::NONE));
    app.add_systems(
        Update,
        (
            update_scroll_pos.before(align_camera_origin),
            align_camera_origin,
        ),
    );
    app.insert_resource(ScrollPosition { x: 0, y: 0 });
    app.insert_resource(CursorModeRes {
        locked: StateValue::new(true),
    });
    app.run();
}

#[derive(Component)]
struct Player;

#[derive(Component)]
struct RopeHolder {
    hand: Entity,
    mouse: Entity,
    indicator: Entity,
    power: f32,
}

#[derive(Component)]
struct ScrollStatic;

#[derive(Component)]
struct UnscrolledPosition {
    pos: Vec2,
}

fn align_camera_origin(
    windows: Query<&Window>,
    scroll_position: Res<ScrollPosition>,
    mut transforms: Query<&mut Transform, With<ScrollStatic>>,
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

    commands.spawn((Camera2d, ScrollStatic));
    let mut last_ent: Option<Entity> = None;

    let p = Vec2::new(800.0, -50.0);
    let hand = commands.spawn((
        Transform::from_xyz(p.x, p.y, 0.0),
        RopeShooter {
            delete_old: true,
            connections: vec![],
        },
        Collider {
            trigger: false,
            shape: Shape::Circle { radius: 4.0 },
            layer: 2,
            layer_mask: 1,
        },
        TrackCollision {
            triggers: Default::default(),
            collisions: HashMap::new(),
            last: HashMap::new(),
            last_triggers: Default::default(),
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

    let mouse = commands.spawn((
        UnscrolledPosition { pos: p },
        Position { pos: p },
        MouseFollower,
        ScrollStatic,
    ));
    let mouse_id = mouse.id();

    let indicator = commands.spawn((
        CursorIndicator,
        Transform::from_xyz(p.x, p.y, 0.0),
        Sprite::from_color(Color::srgba(1.0, 1.0, 1.0, 0.5), Vec2::splat(12.0)),
        // Visibility::Hidden,
    ));
    let indicator_id = indicator.id();

    commands.spawn((
        Transform::from_xyz(p.x, p.y, 0.0),
        Player,
        TrackCollision {
            collisions: Default::default(),
            last: Default::default(),
            triggers: Default::default(),
            last_triggers: Default::default(),
        },
        Collider {
            trigger: false,
            shape: Shape::Circle { radius: 8.0 },
            layer: 2,
            layer_mask: 1,
        },
        RopeHolder {
            power: 0.4,
            hand: hand_ent,
            mouse: mouse_id,
            indicator: indicator_id,
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

#[derive(Component)]
struct CursorIndicator;

// fn follow_mouse_system(
//     camera_query: Single<(&Camera, &GlobalTransform)>,
//     windows: Query<&Window>,
//     mut verlet_query: Query<(&mut VerletObject), With<MouseFollower>>,
// ) {
//     let (camera, camera_transform) = *camera_query;
//
//     let Ok(window) = windows.get_single() else {
//         return;
//     };
//
//     let Some(cursor_position) = window.cursor_position() else {
//         return;
//     };
//
//     // Calculate a world position based on the cursor's position.
//     let Ok(point) = camera.viewport_to_world_2d(camera_transform, cursor_position) else {
//         return;
//     };
//     for (mut verlet_object) in verlet_query.iter_mut() {
//         verlet_object.position_current = point;
//     }
// }

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

fn mouse_motion_test(
    scroll_position: Res<ScrollPosition>,
    mut evr_motion: EventReader<MouseMotion>,
    mut mouse_follower_query: Query<(&mut Position, &mut UnscrolledPosition), With<MouseFollower>>,
) {
    let mut delta = Vec2::ZERO;
    for ev in evr_motion.read() {
        delta.x += ev.delta.x;
        delta.y -= ev.delta.y;
    }
    for (mut pos, mut scroll) in mouse_follower_query.iter_mut() {
        scroll.pos += delta;
        pos.pos = Vec2::new(scroll_position.x as f32, -scroll_position.y as f32) + scroll.pos;
    }
}

fn cursor_indicator_system(
    mouse_follower_query: Query<&Position, With<MouseFollower>>,
    player_query: Query<(&Transform, &RopeHolder)>,
    mut cursor_query: Query<(&mut Transform), (With<CursorIndicator>, Without<RopeHolder>)>,
) {
    for (trans, rope_holder) in player_query.iter() {
        if let Ok((mut cursor)) = cursor_query.get_mut(rope_holder.indicator) {
            if let Ok(mouse_follower) = mouse_follower_query.get(rope_holder.mouse) {
                // cursor.translation.x = mouse_follower.pos.x;
                // cursor.translation.y = mouse_follower.pos.y;
                cursor.translation.x = (mouse_follower.pos.x + trans.translation.x) / 2.0;
                cursor.translation.y = (mouse_follower.pos.y + trans.translation.y) / 2.0;
                // cursor.translation.y = mouse_follower.pos.y;
            }
        }
    }
}

fn update_cursor_mode(
    mut cursor_mode: ResMut<CursorModeRes>,
    mut q_windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    let mut primary_window = q_windows.single_mut();
    if (cursor_mode.locked.dirty) {
        let grab_mode = if cursor_mode.locked.value {
            CursorGrabMode::Locked
        } else {
            CursorGrabMode::None
        };
        primary_window.cursor_options.grab_mode = grab_mode;
        primary_window.cursor_options.visible = !cursor_mode.locked.value;
        cursor_mode.locked.clean();
    }
}
fn relock_cursor(
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut cursor_mode_res: ResMut<CursorModeRes>,
) {
    let mut window = windows.single_mut();

    // If the cursor is not locked and the player clicks, relock it
    if mouse_button_input.just_pressed(MouseButton::Left) {
        focus(&mut cursor_mode_res);
    }
    if mouse_button_input.just_pressed(MouseButton::Right)
        || keyboard_input.just_pressed(KeyCode::Escape)
    {
        unfocus(&mut window);
    }
}
fn handle_window_focus(
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    mut focus_events: EventReader<WindowFocused>,
    mut cursor_mode_res: ResMut<CursorModeRes>,
) {
    let mut window = windows.single_mut();

    for event in focus_events.read() {
        if event.focused {
            focus(&mut cursor_mode_res);
        } else {
            unfocus(&mut window);
        }
    }
}
fn focus(cursor_mode_res: &mut ResMut<CursorModeRes>) {
    cursor_mode_res.locked.dirty = true;
}
fn unfocus(window: &mut Window) {
    window.cursor_options.grab_mode = CursorGrabMode::None;
    window.cursor_options.visible = true;
}

#[derive(Resource)]
pub struct CursorModeRes {
    pub locked: StateValue<bool>,
}
