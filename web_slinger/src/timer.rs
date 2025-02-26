use crate::color_picker::{ColorPickState, GlobalColor};
use crate::physics::{PhysicsSet, TrackCollision};
use crate::{CursorModeRes, Player};
use bevy::app::{App, FixedPreUpdate, FixedUpdate, Plugin, Startup, Update};
use bevy::color::Color;
use bevy::prelude::{
    default, BuildChildren, Button, Changed, ChildBuild, Commands, Component, Entity, Interaction,
    IntoSystemConfigs, JustifyContent, Label, Node, OnEnter, Query, Res, ResMut, Resource, Text,
    TextColor, TextFont, Time, Val, Visibility, Window, With, Without,
};
use bevy::text::cosmic_text::Action;
use bevy::ui::{AlignContent, BackgroundColor, FlexDirection, UiRect};
use bevy::window::{CursorGrabMode, PrimaryWindow};

pub struct TimerPlugin;
impl Plugin for TimerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(TimeTracker {
            time: 0.0,
            active: StateValue::new(false),
            finish_time: 0.0,
            show_finish_screen: StateValue::new(false),
        });
        app.add_systems(Startup, (timer_setup, spawn_finish_screen));
        app.add_systems(FixedUpdate, stop_start_tracking.after(PhysicsSet));
        app.add_systems(
            OnEnter(ColorPickState::Picked),
            (change_background_color, change_text_color),
        );
        app.add_systems(
            FixedPreUpdate,
            (track_time, display_time.before(track_time)),
        );
        app.add_systems(Update, button_system);
    }
}

#[derive(Component)]
pub struct TimerStarter;

#[derive(Component)]
pub struct Finish;
#[derive(Resource)]
pub struct TimeTracker {
    pub time: f32,
    pub active: StateValue<bool>,
    pub finish_time: f32,
    pub show_finish_screen: StateValue<bool>,
}

#[derive(Component)]
struct TimeDisplay;

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
                TimeDisplay,
                Text::new("0.5s"),
                TextFont { ..default() },
                Label,
                Visibility::Hidden,
                TextColorChanger,
                TextColor(Color::WHITE),
            ));
        });
}

fn change_text_color(
    global_color: Res<GlobalColor>,
    mut text_query: Query<(&mut TextColor), With<TextColorChanger>>,
) {
    for (mut text_color) in text_query.iter_mut() {
        text_color.0 = global_color.color;
    }
}

fn change_background_color(
    global_color: Res<GlobalColor>,
    mut background: Query<(&mut BackgroundColor), With<BackgroundColorChanger>>,
) {
    for (mut background_color) in background.iter_mut() {
        background_color.0 = global_color.background_color;
    }
}

fn display_time(
    mut commands: Commands,
    mut time_tracker: ResMut<TimeTracker>,
    mut query: Query<
        (&mut Text, &mut Visibility),
        (
            With<TimeDisplay>,
            Without<FinishDisplay>,
            Without<FinishTime>,
        ),
    >,
    mut finish_query: Query<(&mut Visibility), With<FinishDisplay>>,
    mut finish_time_query: Query<(&mut Text), With<FinishTime>>,
    mut cursor_mode: ResMut<CursorModeRes>,
) {
    for (mut text, mut visibility) in query.iter_mut() {
        if (time_tracker.active.dirty) {
            *visibility = if time_tracker.active.value {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }
        text.0 = format!("{:.2}", time_tracker.time);
    }
    if (time_tracker.show_finish_screen.dirty) {
        for mut finish_visibility in finish_query.iter_mut() {
            *finish_visibility = if time_tracker.show_finish_screen.value {
                cursor_mode.locked.set(false);
                Visibility::Visible
            } else {
                cursor_mode.locked.set(true);
                Visibility::Hidden
            };
        }

        for mut finish_time in finish_time_query.iter_mut() {
            finish_time.0 = format!(
                "You reached the goal in {:.2} seconds",
                time_tracker.finish_time
            );
        }
    }
    time_tracker.active.clean();
    time_tracker.show_finish_screen.clean();
}

fn stop_start_tracking(
    player_tracker_query: Query<(&TrackCollision), With<Player>>,
    mut time_tracker: ResMut<TimeTracker>,
    timer_starter_query: Query<&TimerStarter>,
    finish_query: Query<&Finish>,
    mut commands: Commands,
    global_color: Res<GlobalColor>,
) {
    for (collision_tracker) in player_tracker_query.iter() {
        for trigger in &collision_tracker.triggers {
            if let Ok(timer_starter) = timer_starter_query.get(*trigger) {
                time_tracker.active.set(true);
                time_tracker.time = 0.0;
            }

            if let Ok(finish) = finish_query.get(*trigger) {
                if (time_tracker.active.value) {
                    time_tracker.show_finish_screen.set(true);
                }
                time_tracker.active.set(false);
                time_tracker.finish_time = time_tracker.time;
                time_tracker.time = 0.0;
            }
        }
    }
}
fn track_time(time: Res<Time>, mut time_tracker: ResMut<TimeTracker>) {
    if (time_tracker.active.value) {
        time_tracker.time += time.delta_secs();
    }
}

#[derive(Component)]
struct FinishDisplay;

#[derive(Component)]
struct FinishTime;

#[derive(Component)]
struct TextColorChanger;

#[derive(Component)]
struct BackgroundColorChanger;
fn spawn_finish_screen(mut commands: Commands) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                ..default()
            },
            FinishDisplay,
            Visibility::Hidden,
        ))
        .with_children(|parent| {
            parent
                .spawn(Node {
                    width: Val::Auto,
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::Center,
                    ..default()
                })
                .with_children(|parent| {
                    parent
                        .spawn((
                            Node {
                                width: Val::Auto,
                                height: Val::Auto,
                                flex_direction: FlexDirection::Column,
                                justify_content: JustifyContent::Center,
                                padding: UiRect::all(Val::Px(5.0)),
                                ..default()
                            },
                            BackgroundColorChanger,
                            BackgroundColor(Color::BLACK),
                        ))
                        .with_children(|parent| {
                            parent
                                .spawn(Node {
                                    width: Val::Auto,
                                    height: Val::Auto,
                                    justify_content: JustifyContent::End,
                                    ..default()
                                })
                                .with_children(|parent| {
                                    parent.spawn((
                                        Text::new("X"),
                                        UIAction::Close,
                                        Button,
                                        TextFont { ..default() },
                                        Label,
                                        TextColorChanger,
                                        TextColor(Color::WHITE),
                                    ));
                                });
                            parent.spawn((
                                FinishTime,
                                Text::new(format!("You reached the goal in {:.2} seconds!", 0.0)),
                                TextFont { ..default() },
                                Label,
                                TextColorChanger,
                                TextColor(Color::WHITE),
                            ));
                        });
                });
        });
}

pub struct StateValue<T> {
    pub dirty: bool,
    pub value: T,
}
impl<T> StateValue<T> {
    pub fn new(initial: T) -> Self {
        Self {
            dirty: true,
            value: initial,
        }
    }

    pub fn set(&mut self, value: T) {
        self.value = value;
        self.dirty = true;
    }

    pub fn clean(&mut self) {
        self.dirty = false;
    }
}

#[derive(Component)]
enum UIAction {
    Close,
}
fn button_system(
    interaction_query: Query<(&Interaction, &UIAction), (Changed<Interaction>, With<Button>)>,
    mut timer_res: ResMut<TimeTracker>,
) {
    for (interaction, action) in interaction_query.iter() {
        match *interaction {
            Interaction::Pressed => match action {
                UIAction::Close => {
                    timer_res.show_finish_screen.set(false);
                }
            },
            Interaction::Hovered => {}
            Interaction::None => {}
        }
    }
}
