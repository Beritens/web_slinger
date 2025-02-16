use bevy::app::{App, FixedUpdate, Plugin};
use bevy::prelude::Commands;
use bevy::reflect::erased_serde::__private::serde::{Deserialize, Serialize};
use serde_wasm_bindgen::from_value;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

pub struct CollisionImportPlugin;

impl Plugin for CollisionImportPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, get_colliders_system);
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn get_colliders_system() {}

#[cfg(target_arch = "wasm32")]
fn get_colliders_system(mut commands: Commands) {
    let colliders = get_colliders_rust();
    log(colliders.len().to_string().as_str());
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
