use bevy::{color::palettes::css::*, prelude::*};
use bevy_ecs::system::EntityCommands;
use sickle_ui::prelude::*;

use crate::menu::WorldListEntry;

#[derive(Event)]
pub struct ButtonPressed;

pub struct ButtonWidgetPlugin;

impl Plugin for ButtonWidgetPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<ButtonPressed>();

        app.add_systems(Update, button_system);
    }
}

pub trait ButtonWidgetExt {
    fn button(&mut self, title: String, font_size: f32) -> EntityCommands;
}

impl ButtonWidgetExt for UiBuilder<'_, Entity> {
    fn button(&mut self, title: String, font_size: f32) -> EntityCommands {
        let id = self
            .container(
                ButtonBundle {
                    style: Style {
                        border: UiRect::all(Val::Px(5.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        padding: UiRect::all(Val::Px(5.0)),
                        ..default()
                    },
                    border_color: Color::WHITE.into(),
                    background_color: Color::BLACK.into(),
                    ..default()
                },
                |button| {
                    button.named(format!("'{}' Button", title));
                    button
                        .spawn(
                            TextBundle::from_section(
                                title,
                                TextStyle {
                                    font_size,
                                    color: Color::WHITE,
                                    ..default()
                                },
                            )
                            .with_text_justify(JustifyText::Center),
                        )
                        .style()
                        .font("fonts/nokiafc22.ttf".to_string());
                },
            )
            .id();

        return self.commands().entity(id);
    }
}

fn button_system(
    mut commands: Commands,
    mut interaction_query: Query<
        (
            Entity,
            &Interaction,
            &mut BackgroundColor,
            &mut BorderColor,
            &Children,
        ),
        (With<Button>, Without<WorldListEntry>),
    >,
    mut text_query: Query<&mut Text>,
    input: Res<ButtonInput<MouseButton>>,
) {
    for (entity, interaction, mut color, mut border_color, children) in &mut interaction_query {
        if let Ok(mut text) = text_query.get_mut(children[0]) {
            match *interaction {
                Interaction::None => {
                    *color = BackgroundColor(Color::BLACK);
                    border_color.0 = Color::WHITE;
                    text.sections[0].style.color = Color::WHITE;
                }
                Interaction::Hovered => {
                    *color = BackgroundColor(GRAY.into());
                    border_color.0 = Color::WHITE;
                    text.sections[0].style.color = Color::WHITE;

                    if input.just_released(MouseButton::Left) {
                        commands.trigger_targets(ButtonPressed, entity);
                    }
                }
                Interaction::Pressed => {
                    *color = BackgroundColor(Color::WHITE);
                    border_color.0 = Color::BLACK;
                    text.sections[0].style.color = Color::BLACK;

                    if input.just_released(MouseButton::Left) {
                        commands.trigger_targets(ButtonPressed, entity);
                    }
                }
            }
        }
    }
}
