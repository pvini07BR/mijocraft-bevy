use bevy::prelude::*;
use sickle_ui::{prelude::*, widgets::inputs::slider::SliderAxis};

use crate::player::PlayerSettings;

#[derive(Component)]
struct ColorHueSlider;

#[derive(Component)]
struct ColorSaturationSlider;

#[derive(Component)]
struct PlayerColorPreview;

#[derive(Component)]
struct ColorLightnessSlider;

pub struct PlayerSettingsWidgetPlugin;

impl Plugin for PlayerSettingsWidgetPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (color_sliders_system, update_color_preview));
    }
}

pub trait PlayerSettingsWidgetExt {
    fn player_settings(&mut self, player_settings_res: &PlayerSettings) -> UiBuilder<Entity>;
}

impl PlayerSettingsWidgetExt for UiBuilder<'_, Entity> {
    fn player_settings(&mut self, player_settings_res: &PlayerSettings) -> UiBuilder<Entity> {
        return self.column(|column| {
            column.style().height(Val::Auto);
            column.row(|color| {
                color.style().justify_content(JustifyContent::SpaceAround);

                color.container(
                    (
                        NodeBundle {
                            style: Style {
                                min_width: Val::Px(72.0),
                                height: Val::Percent(100.0),
                                ..default()
                            },
                            background_color: player_settings_res.color.into(),
                            ..default()
                        },
                        PlayerColorPreview,
                    ),
                    |_| {},
                );

                color.column(|sliders| {
                    sliders.style().width(Val::Percent(100.0));

                    let player_color = Hsla::from(player_settings_res.color);
                    sliders
                        .slider(SliderConfig::new(
                            "H".to_string(),
                            0.0,
                            360.0,
                            player_color.hue,
                            true,
                            SliderAxis::Horizontal,
                        ))
                        .insert(ColorHueSlider);
                    sliders
                        .slider(SliderConfig::new(
                            "S".to_string(),
                            0.0,
                            1.0,
                            player_color.saturation,
                            true,
                            SliderAxis::Horizontal,
                        ))
                        .insert(ColorSaturationSlider);
                    sliders
                        .slider(SliderConfig::new(
                            "L".to_string(),
                            0.0,
                            1.0,
                            player_color.lightness,
                            true,
                            SliderAxis::Horizontal,
                        ))
                        .insert(ColorLightnessSlider);
                });
            });
        });
    }
}

fn color_sliders_system(
    hue_slider: Query<&Slider, (Changed<Slider>, With<ColorHueSlider>)>,
    saturation_slider: Query<&Slider, (Changed<Slider>, With<ColorSaturationSlider>)>,
    lightness_slider: Query<&Slider, (Changed<Slider>, With<ColorLightnessSlider>)>,

    mut player_settings: ResMut<PlayerSettings>,
) {
    let mut color = Hsla::from(player_settings.color);

    if let Ok(slider) = hue_slider.get_single() {
        color.hue = slider.value();
    }

    if let Ok(slider) = saturation_slider.get_single() {
        color.saturation = slider.value();
    }

    if let Ok(slider) = lightness_slider.get_single() {
        color.lightness = slider.value();
    }

    player_settings.color = color.into();
}

fn update_color_preview(
    player_settings: Res<PlayerSettings>,
    mut color_preview_q: Query<&mut BackgroundColor, With<PlayerColorPreview>>,
) {
    if player_settings.is_changed() {
        if let Ok(mut bgc) = color_preview_q.get_single_mut() {
            bgc.0 = player_settings.color;
        }
    }
}
