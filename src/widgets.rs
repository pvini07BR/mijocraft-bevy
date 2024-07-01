use bevy::prelude::*;
use bevy_inspector_egui::egui::util::undoer::Settings;
use sickle_ui::prelude::*;
use sickle_ui::prelude::StylableAttribute::AlignSelf;
use sickle_ui::widgets::inputs::slider::SliderAxis;
use crate::{GameSettings, GameState};

pub struct UIWidgetsPlugin;

#[derive(Event, Deref, DerefMut)]
pub struct SpawnSettingsWidget(pub Entity);

impl Plugin for UIWidgetsPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<SpawnSettingsWidget>();

        app.add_systems(Update,
            spawn_settings_widget.run_if(in_state(GameState::Menu))
        );
    }
}

fn spawn_settings_widget(
    mut commands: Commands,
    mut spawn_ev: EventReader<SpawnSettingsWidget>,
    asset_server: Res<AssetServer>,
    settings_res: Res<GameSettings>
) {
    for ev in spawn_ev.read() {
        let mut root = commands.ui_builder(ev.0);

        root.container(NodeBundle {..default()}, |container| {
            container.style()
                .flex_direction(FlexDirection::Column)
                .min_width(Val::Px(500.0))
            ;

            container.row(|wall_ao_option| {
                wall_ao_option.style()
                    .justify_content(JustifyContent::SpaceBetween)
                ;

                wall_ao_option.spawn(
                    TextBundle::from_section("Wall Ambient Occlusion",
                    TextStyle {
                        font: asset_server.load("fonts/nokiafc22.ttf"),
                        font_size: 24.0,
                        color: Color::WHITE,
                        ..default()
                    })
                );

                wall_ao_option.checkbox(None, settings_res.wall_ambient_occlusion);
            });

            container.row(|smooth_light_option| {
                smooth_light_option.style()
                    .justify_content(JustifyContent::SpaceBetween)
                ;

                smooth_light_option.spawn(
                    TextBundle::from_section("Smooth Lighting",
                    TextStyle {
                        font: asset_server.load("fonts/nokiafc22.ttf"),
                        font_size: 24.0,
                        color: Color::WHITE,
                        ..default()
                    })
                );

                smooth_light_option.checkbox(None, settings_res.smooth_lighting);
            });

            container.row(|wall_darkness_option| {
                wall_darkness_option.style()
                    .justify_content(JustifyContent::SpaceBetween)
                ;

                wall_darkness_option.spawn(
                    TextBundle::from_section("Wall Darkness",
                     TextStyle {
                         font: asset_server.load("fonts/nokiafc22.ttf"),
                         font_size: 24.0,
                         color: Color::WHITE,
                         ..default()
                     })
                ).style().width(Val::Percent(100.0));

                wall_darkness_option.slider(SliderConfig::new(None, 0.0, 1.0, settings_res.wall_darkness, true, SliderAxis::Horizontal));
            });
        });
    }
}