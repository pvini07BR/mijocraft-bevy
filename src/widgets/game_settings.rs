use bevy::prelude::*;
use sickle_ui::{prelude::*, widgets::inputs::slider::SliderAxis};

use crate::GameSettings;

#[derive(Component)]
struct WallAOSetting;

#[derive(Component)]
struct SmoothLightingSetting;

#[derive(Component)]
struct WallDarknessSetting;

#[derive(Event)]
pub struct ApplyGameSettings;

#[derive(Resource, Deref, DerefMut)]
pub struct AutoApplySettings(pub bool);

pub struct GameSettingsWidgetPlugin;

impl Plugin for GameSettingsWidgetPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<ApplyGameSettings>();

        app.insert_resource(AutoApplySettings(true));

        app.add_systems(
            Update,
            (
                apply_game_settings,
                wall_ao_setting_set.run_if(does_auto_apply),
                smooth_lighting_setting_set.run_if(does_auto_apply),
                wall_darkness_setting_set.run_if(does_auto_apply),
            ),
        );
    }
}

#[derive(Component)]
pub struct GameSettingsWidget;

pub trait GameSettingsWidgetExt {
    fn game_settings(
        &mut self,
        asset_server: &AssetServer,
        settings_res: &GameSettings,
    ) -> UiBuilder<Entity>;
}

impl GameSettingsWidgetExt for UiBuilder<'_, Entity> {
    fn game_settings(
        &mut self,
        asset_server: &AssetServer,
        settings_res: &GameSettings,
    ) -> UiBuilder<Entity> {
        return self.container(NodeBundle { ..default() }, |container| {
            container.named("Game Settings Widget");
            container.insert(GameSettingsWidget);
            container
                .style()
                .flex_direction(FlexDirection::Column)
                .min_width(Val::Px(450.0));

            container.row(|wall_ao_option| {
                wall_ao_option
                    .style()
                    .justify_content(JustifyContent::SpaceBetween);

                wall_ao_option.spawn(TextBundle::from_section(
                    "Wall Ambient Occlusion",
                    TextStyle {
                        font: asset_server.load("fonts/nokiafc22.ttf"),
                        font_size: 24.0,
                        color: Color::WHITE,
                        ..default()
                    },
                ));

                wall_ao_option
                    .checkbox(None, settings_res.wall_ambient_occlusion)
                    .insert(WallAOSetting);
            });

            container.row(|smooth_light_option| {
                smooth_light_option
                    .style()
                    .justify_content(JustifyContent::SpaceBetween);

                smooth_light_option.spawn(TextBundle::from_section(
                    "Smooth Lighting",
                    TextStyle {
                        font: asset_server.load("fonts/nokiafc22.ttf"),
                        font_size: 24.0,
                        color: Color::WHITE,
                        ..default()
                    },
                ));

                smooth_light_option
                    .checkbox(None, settings_res.smooth_lighting)
                    .insert(SmoothLightingSetting);
            });

            container.row(|wall_darkness_option| {
                wall_darkness_option
                    .style()
                    .justify_content(JustifyContent::SpaceBetween);

                wall_darkness_option
                    .spawn(TextBundle::from_section(
                        "Wall Darkness",
                        TextStyle {
                            font: asset_server.load("fonts/nokiafc22.ttf"),
                            font_size: 24.0,
                            color: Color::WHITE,
                            ..default()
                        },
                    ))
                    .style()
                    .width(Val::Percent(100.0));

                wall_darkness_option
                    .slider(SliderConfig::new(
                        None,
                        0.0,
                        1.0,
                        settings_res.wall_darkness,
                        true,
                        SliderAxis::Horizontal,
                    ))
                    .insert(WallDarknessSetting);
            });
        });
    }
}

fn apply_game_settings(
    mut apply_game_settings_ev: EventReader<ApplyGameSettings>,
    mut settings_res: ResMut<GameSettings>,

    wall_ao_option: Query<&Checkbox, With<WallAOSetting>>,
    smooth_lighting_option: Query<&Checkbox, With<SmoothLightingSetting>>,
    wall_darkness_slider: Query<&Slider, With<WallDarknessSetting>>,
) {
    for _ in apply_game_settings_ev.read() {
        if let Ok(checkbox) = wall_ao_option.get_single() {
            settings_res.wall_ambient_occlusion = checkbox.checked;
        }

        if let Ok(checkbox) = smooth_lighting_option.get_single() {
            settings_res.smooth_lighting = checkbox.checked;
        }

        if let Ok(slider) = wall_darkness_slider.get_single() {
            settings_res.wall_darkness = slider.value();
        }
    }
}

fn does_auto_apply(res: Res<AutoApplySettings>) -> bool {
    return res.0;
}

fn wall_ao_setting_set(
    query: Query<&Checkbox, (Changed<Checkbox>, With<WallAOSetting>)>,
    mut game_settings: ResMut<GameSettings>,
) {
    if let Ok(checkbox) = query.get_single() {
        game_settings.wall_ambient_occlusion = checkbox.checked;
    }
}

fn smooth_lighting_setting_set(
    query: Query<&Checkbox, (Changed<Checkbox>, With<SmoothLightingSetting>)>,
    mut game_settings: ResMut<GameSettings>,
) {
    if let Ok(checkbox) = query.get_single() {
        game_settings.smooth_lighting = checkbox.checked;
    }
}

fn wall_darkness_setting_set(
    query: Query<&Slider, (Changed<Slider>, With<WallDarknessSetting>)>,
    mut game_settings: ResMut<GameSettings>,
) {
    if let Ok(slider) = query.get_single() {
        game_settings.wall_darkness = slider.value();
    }
}
