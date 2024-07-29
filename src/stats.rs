use bevy::diagnostic::{
    Diagnostic, DiagnosticPath, Diagnostics, DiagnosticsStore, FrameTimeDiagnosticsPlugin,
    RegisterDiagnostic,
};
use bevy::prelude::*;

#[cfg(debug_assertions)]
use bevy::diagnostic::LogDiagnosticsPlugin;

pub struct StatsPlugin;
impl Plugin for StatsPlugin {
    fn build(&self, app: &mut App) {
        // For printing FPS diagnostics to the screen
        app.add_plugins(FrameTimeDiagnosticsPlugin)
            .add_systems(Startup, setup_fps_counter)
            .add_systems(Update, update_fps_counter);

        #[cfg(debug_assertions)]
        app.add_plugins(LogDiagnosticsPlugin::default()); // Adds print out to console

        // Add a custom diagnostic
        app.register_diagnostic(Diagnostic::new(REAL_TIME_RATE).with_suffix(" iterations"))
            .add_systems(Update, update_custom_diagnostic);
        // .add_systems(Update, print_custom_diagnostic);
    }
}

#[derive(Component)]
struct FpsText;

fn setup_fps_counter(mut commands: Commands) {
    commands
        .spawn(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                top: Val::Px(5.0),
                right: Val::Px(5.0),
                ..Default::default()
            },
            background_color: Color::NONE.into(),
            ..Default::default()
        })
        .with_children(|parent| {
            parent
                .spawn(TextBundle {
                    text: Text::from_section(
                        "FPS: ".to_string(),
                        TextStyle {
                            font_size: 30.0,
                            color: Color::WHITE,
                            ..default()
                        },
                    ),
                    ..Default::default()
                })
                .insert(FpsText);
        });
}

fn update_fps_counter(
    diagnostics: Res<DiagnosticsStore>,
    mut query: Query<&mut Text, With<FpsText>>,
) {
    let fps = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS);
    let real_time = diagnostics.get(&REAL_TIME_RATE);

    if let (Some(fpsd), Some(real_timed)) = (fps, real_time) {
        for mut text in query.iter_mut() {
            text.sections[0].value = format!(
                "FPS: {:.2}\nREAL TIME: {:.2}",
                fpsd.average().unwrap_or(0.0),
                real_timed.average().unwrap_or(0.0)
            );
        }
    };
}

// Add a measurement for REAL_TIME_RATE, note this should probably be the physics time ...
const REAL_TIME_RATE: DiagnosticPath = DiagnosticPath::const_new("real_time_rate");
fn update_custom_diagnostic(
    mut diagnostics: Diagnostics,
    diagnostics_store: Res<DiagnosticsStore>,
) {
    if let Some(fps) = diagnostics_store.get(&FrameTimeDiagnosticsPlugin::FPS) {
        diagnostics.add_measurement(&REAL_TIME_RATE, || fps.average().unwrap_or(0.0) / 60.0);
    };
}
