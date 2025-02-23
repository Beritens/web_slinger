use crate::color_picker::GlobalColor;
use crate::physics::{PhysicsSet, TrackCollision};
use crate::Player;
use bevy::app::{App, FixedPreUpdate, FixedUpdate, Plugin, Startup, Update};
use bevy::color::Color;
use bevy::prelude::{
    default, BuildChildren, ChildBuild, Commands, Component, Entity, IntoSystemConfigs,
    JustifyContent, Label, Node, Query, Res, ResMut, Resource, Text, TextColor, TextFont, Time,
    Val, Visibility, With,
};

pub struct TimerPlugin;
impl Plugin for TimerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(TimeTracker {
            time: 0.0,
            active: false,
        });
        app.add_systems(Startup, timer_setup);
        app.add_systems(FixedUpdate, stop_start_tracking.after(PhysicsSet));
        app.add_systems(
            FixedPreUpdate,
            (track_time, display_time.before(track_time)),
        );
    }
}

#[derive(Component)]
pub struct TimerStarter;

#[derive(Component)]
pub struct Finish;
#[derive(Resource)]
pub struct TimeTracker {
    pub time: f32,
    pub active: bool,
}

#[derive(Component)]
pub struct TimeDisplay {
    active: bool,
}

fn timer_setup(mut commands: Commands) {
    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            ..default()
        })
        .with_children(|parent| {
            parent.spawn((
                TimeDisplay { active: false },
                Text::new("0.5s"),
                TextFont { ..default() },
                Label,
                Visibility::Hidden,
                TextColor(Color::WHITE),
            ));
        });
}

fn display_time(
    mut commands: Commands,
    time_tracker: Res<TimeTracker>,
    color: Res<GlobalColor>,
    mut query: Query<(&mut Text, &mut Visibility, &mut TimeDisplay, &mut TextColor)>,
) {
    for (mut text, mut visibility, mut time_display, mut text_color) in query.iter_mut() {
        if (time_display.active != time_tracker.active) {
            *visibility = if time_tracker.active {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }
        text_color.0 = color.color;
        time_display.active = time_tracker.active;
        text.0 = format!("{:.2}", time_tracker.time);
    }
}

fn stop_start_tracking(
    player_tracker_query: Query<(&TrackCollision), With<Player>>,
    mut time_tracker: ResMut<TimeTracker>,
    timer_starter_query: Query<&TimerStarter>,
    finish_query: Query<&Finish>,
) {
    for (collision_tracker) in player_tracker_query.iter() {
        for trigger in &collision_tracker.triggers {
            if let Ok(timer_starter) = timer_starter_query.get(*trigger) {
                time_tracker.active = true;
                time_tracker.time = 0.0;
            }

            if let Ok(finish) = finish_query.get(*trigger) {
                if (time_tracker.active) {

                    //spawn finish screen
                }
                time_tracker.active = false;
                time_tracker.time = 0.0;
            }
        }
    }
}
fn track_time(time: Res<Time>, mut time_tracker: ResMut<TimeTracker>) {
    if (time_tracker.active) {
        time_tracker.time += time.delta_secs();
    }
}
