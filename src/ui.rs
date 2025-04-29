use super::StartPlaying;
use crate::{StartGraphingEvent, models::*};
use bevy::prelude::*;
use bevy_egui::{
    EguiContexts,
    egui::{self, RichText},
};

/// Render the UI (run each frame on the Update schedule) and handle user
/// interactions with the UI. This sends events for major state transitions
/// that should be handled in other systems
pub fn ui_system(
    mut contexts: EguiContexts,
    mut state: ResMut<GameState>,
    start_playing_events: EventWriter<StartPlaying>,
    gizmos: Gizmos,
    start_graphing_events: EventWriter<StartGraphingEvent>,
) {
    match state.game_phase() {
        GamePhaseNoData::Setup => {
            setup_ui(contexts.ctx_mut(), &mut state, start_playing_events)
        }
        GamePhaseNoData::Playing => play_ui(
            contexts.ctx_mut(),
            &mut state,
            gizmos,
            start_graphing_events,
        ),
        GamePhaseNoData::GameFinished => {
            finished_ui(contexts.ctx_mut(), &mut state)
        }
    };
}

fn setup_ui(
    context: &bevy_egui::egui::Context,
    state: &mut GameState,
    mut start_playing_events: EventWriter<StartPlaying>,
) {
    #[cfg(debug_assertions)]
    const MIN_SECONDS: usize = 2;
    #[cfg(not(debug_assertions))]
    const MIN_SECONDS: usize = 20;
    if state.setup_state().is_none() {
        return;
    };
    egui::SidePanel::new(egui::panel::Side::Left, "setup_panel").show(
        context,
        |ui| {
            let Some(setup_state) = state.setup_state_mut() else {
                return;
            };
            ui.label(RichText::new("Player 1").heading());
            ui.label("Starting soldiers:");
            ui.add(
                egui::widgets::DragValue::new(
                    &mut setup_state.player_1.soldier_num,
                )
                .range(1..=4),
            );
            ui.label("Name:");
            ui.text_edit_singleline(&mut setup_state.player_1.name);
            ui.separator();
            ui.label(RichText::new("Player 2").heading());
            ui.label("Starting soldiers:");
            ui.add(
                egui::widgets::DragValue::new(
                    &mut setup_state.player_2.soldier_num,
                )
                .range(1..=4),
            );
            ui.label("Name:");
            ui.text_edit_singleline(&mut setup_state.player_2.name);

            ui.separator();
            ui.horizontal(|ui| {
                ui.label("Seconds per turn:");
                ui.add(
                    egui::widgets::DragValue::new(
                        &mut setup_state.turn_seconds,
                    )
                    .range(MIN_SECONDS..=300),
                );
            });
            if ui.button(RichText::new("Start").size(20.)).clicked() {
                start_playing_events.send(StartPlaying);
            }
        },
    );
}

fn play_ui(
    context: &bevy_egui::egui::Context,
    state: &mut GameState,
    mut gizmos: Gizmos,
    mut start_graphing_events: EventWriter<StartGraphingEvent>,
) {
    let Some(playing_state) = state.playing_state_mut() else {
        return;
    };
    let data = PlayUiData::new(playing_state);
    gizmos.circle_2d(
        Isometry2d {
            rotation: Rot2::IDENTITY,
            translation: data.soldier_loc * 20.,
        },
        super::SOLDIER_RADIUS,
        super::ACTIVE_SOLDIER_OUTLINE_COLOR,
    );
    if let Some(input_data) = data.input_ui {
        egui::TopBottomPanel::new(
            egui::panel::TopBottomSide::Bottom,
            "playing_input_panel",
        )
        .show(context, |ui| {
            ui.horizontal(|ui| {
                ui.text_edit_singleline(input_data.current_input);
                if ui.button("Done").clicked() {
                    if let Ok(func) = input_data.current_input.parse() {
                        start_graphing_events.send(StartGraphingEvent(func));
                    }
                }
                ui.label(input_data.timer.remaining().as_secs().to_string());
            })
        });
    }
}

fn finished_ui(context: &bevy_egui::egui::Context, state: &mut GameState) {
    let Some(finished_state) = state.finished_state_mut() else {
        return;
    };

    let winner = match finished_state.winner {
        PlayerSelect::Player1 => 1,
        PlayerSelect::Player2 => 2,
    };

    egui::Window::new("Game Over!")
        .movable(false)
        .resizable(false)
        .collapsible(false)
        .show(context, |ui| {
            ui.label(format!("Player {} wins!", winner));
            if ui.button("Restart").clicked() {
                *state = GameState::default();
            }
        });
}
