use crate::physics::{Collider, CollisionSetup, Shape, StaticCollider, VerletObject};
use bevy::app::{App, FixedUpdate, Plugin, Startup};
use bevy::prelude::{Color, Commands, IntoSystemConfigs, Transform, Vec2};
use bevy::reflect::erased_serde::__private::serde::{Deserialize, Serialize};
use bevy::sprite::Sprite;
use bevy::utils::default;
use serde_wasm_bindgen::from_value;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

pub struct CollisionImportPlugin;

impl Plugin for CollisionImportPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, get_colliders_system.before(CollisionSetup));
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn get_colliders_system(mut commands: Commands) {
    for i in 0..=10 {
        for j in 0..=10 {
            let pos = Vec2::new(100.0 + (i as f32 * 40.0), -500.0 + (j as f32 * 40.0));
            commands.spawn((
                StaticCollider,
                Collider {
                    layer: 1,
                    layer_mask: 1,
                    shape: Shape::Box {
                        width: 15.0,
                        height: 15.0,
                    },
                },
                VerletObject {
                    fixed: true,
                    position_current: pos,
                    ..default()
                },
                Sprite::from_color(Color::BLACK, Vec2::new(30.0, 30.0)),
                Transform::from_xyz(pos.x, pos.y, 1.0),
            ));
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn get_colliders_system(mut commands: Commands) {
    let colliders = get_colliders_rust();

    for collider in colliders {
        let mid_x = (collider.right + collider.left) / 2.0;
        let mid_y = (-collider.top + -collider.bottom) / 2.0;
        let width = (collider.right - collider.left).abs() / 2.0;
        let height = (-collider.top + collider.bottom).abs() / 2.0;

        let pos = Vec2::new(mid_x, mid_y);
        commands.spawn((
            StaticCollider,
            Collider {
                layer: 1,
                layer_mask: 1,
                shape: Shape::Box {
                    width: width,
                    height: height,
                },
            },
            VerletObject {
                fixed: true,
                position_current: pos,
                ..default()
            },
            // Sprite::from_color(Color::BLACK, Vec2::new(width * 2.0, height * 2.0)),
            Transform::from_xyz(pos.x, pos.y, 1.0),
        ));
    }
}
#[derive(Serialize, Deserialize, Debug)]
struct TestCollider {
    top: f32,
    bottom: f32,
    right: f32,
    left: f32,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    fn get_colliders() -> JsValue;

    // Use `js_namespace` here to bind `console.log(..)` instead of just
    // `log(..)`
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);

    // The `console.log` is quite polymorphic, so we can bind it with multiple
    // signatures. Note that we need to use `js_name` to ensure we always call
    // `log` in JS.
    #[wasm_bindgen(js_namespace = console, js_name = log)]
    fn log_u32(a: u32);

    // Multiple arguments too!
    #[wasm_bindgen(js_namespace = console, js_name = log)]
    fn log_many(a: &str, b: &str);
}

#[cfg(target_arch = "wasm32")]
pub fn get_colliders_rust() -> Vec<TestCollider> {
    let colliders_js: JsValue = get_colliders();
    if let Some(string) = colliders_js.as_string() {
        log(string.as_str());
    }
    from_value::<Vec<TestCollider>>(colliders_js).unwrap_or_else(|e| {
        log(&format!("Deserialization error: {:?}", e));
        vec![]
    })
}
