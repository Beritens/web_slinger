use crate::collider_import::Colored;
use crate::physics::{PhysicsSet, TrackCollision};
use crate::Player;
use bevy::app::{App, FixedUpdate, Plugin};
use bevy::color::{Alpha, Color};
use bevy::prelude::{
    in_state, AppExtStates, IntoSystemConfigs, Luminance, NextState, Query, ResMut, Resource,
    Sprite, States, With,
};

pub struct ColorPickerPlugin;

impl Plugin for ColorPickerPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<ColorPickState>();
        app.insert_resource(GlobalColor {
            color: Color::linear_rgb(0.0, 1.0, 0.0),
            background_color: Color::linear_rgb(0.0, 0.0, 0.0),
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
pub enum ColorPickState {
    #[default]
    Picking,
    Picked,
}

#[derive(Resource)]
pub struct GlobalColor {
    pub color: Color,
    pub background_color: Color,
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
                    sprite.color = colored.color.with_alpha(sprite.color.alpha());
                }
                color_res.color = colored.color;
                let value = color_res.color.luminance();
                if (value > 0.3) {
                    color_res.background_color = Color::BLACK.with_alpha(0.5);
                } else {
                    color_res.background_color = Color::WHITE.with_alpha(0.5);
                }
                pick_state.set(ColorPickState::Picked);
            }
        }
    }
}
