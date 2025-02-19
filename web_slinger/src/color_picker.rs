use crate::collider_import::Colored;
use crate::physics::{PhysicsSet, TrackCollision};
use crate::Player;
use bevy::app::{App, FixedPreUpdate, FixedUpdate, Plugin, Startup};
use bevy::color::Color;
use bevy::prelude::{
    in_state, AppExtStates, Component, IntoSystemConfigs, NextState, Query, ResMut, Resource,
    Sprite, States, With,
};

pub struct ColorPickerPlugin;

impl Plugin for ColorPickerPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<ColorPickState>();
        app.insert_resource(GlobalColor {
            color: Color::linear_rgb(0.0, 1.0, 0.0),
        });
        app.add_systems(
            FixedUpdate,
            change_color
                .after(PhysicsSet)
                .run_if(in_state(ColorPickState::Picking)),
        );
    }
}
#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
enum ColorPickState {
    #[default]
    Picking,
    Picked,
}

#[derive(Resource)]
pub struct GlobalColor {
    pub color: Color,
}
fn change_color(
    player_query: Query<(&TrackCollision), With<Player>>,
    colored_query: Query<&Colored>,
    mut sprite_query: Query<&mut Sprite>,
    mut color_res: ResMut<GlobalColor>,
    mut pick_state: ResMut<NextState<ColorPickState>>,
) {
    for (track_collision) in player_query.iter() {
        if let Some(collision) = track_collision.collisions.keys().next() {
            if let Ok(colored) = colored_query.get(*collision) {
                for mut sprite in sprite_query.iter_mut() {
                    sprite.color = colored.color;
                }
                color_res.color = colored.color;
                pick_state.set(ColorPickState::Picked);
            }
        }
    }
}
