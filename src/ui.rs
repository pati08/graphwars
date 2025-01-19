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
    mut state: ResMut<GamePhase>,
    start_playing_events: EventWriter<StartPlaying>,
    gizmos: Gizmos,
    start_graphing_events: EventWriter<StartGraphingEvent>,
) {
    match *state {
        GamePhase::Setup(_) => {
            setup_ui(contexts.ctx_mut(), &mut state, start_playing_events)
        }
        GamePhase::Playing(_) => play_ui(
            contexts.ctx_mut(),
            &mut state,
            gizmos,
            start_graphing_events,
        ),
        GamePhase::GameFinished(_) => {
            finished_ui(contexts.ctx_mut(), &mut state)
        }
    };
}

fn setup_ui(
    context: &bevy_egui::egui::Context,
    state: &mut GamePhase,
    mut start_playing_events: EventWriter<StartPlaying>,
) {
    let GamePhase::Setup(_) = state else {
        return;
    };
    egui::SidePanel::new(egui::panel::Side::Left, "setup_panel").show(
        context,
        |ui| {
            let &mut GamePhase::Setup(ref mut setup_state) = state else {
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
                    // TODO: This is for development, make it 20..=300 later
                    // .range(20..=300),
                    .range(2..=300),
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
    state: &mut GamePhase,
    mut gizmos: Gizmos,
    mut start_graphing_events: EventWriter<StartGraphingEvent>,
) {
    let &mut GamePhase::Playing(ref mut playing_state) = state else {
        return;
    };
    let current_player = if let PlayerSelect::Player1 = playing_state.turn {
        &mut playing_state.player_1
    } else {
        &mut playing_state.player_2
    };
    let current_soldier =
        &mut current_player.living_soldiers[current_player.active_soldier];
    let current_input = &mut current_soldier.equation;
    let active_soldier_pos = current_soldier.graph_location;
    gizmos.circle_2d(
        Isometry2d {
            rotation: Rot2::IDENTITY,
            translation: active_soldier_pos * 20.,
        },
        super::SOLDIER_RADIUS,
        super::ACTIVE_SOLDIER_OUTLINE_COLOR,
    );
    if let &mut TurnPhase::InputPhase { ref timer } =
        &mut playing_state.turn_phase
    {
        egui::TopBottomPanel::new(
            egui::panel::TopBottomSide::Bottom,
            "playing_input_panel",
        )
        .show(context, |ui| {
            ui.horizontal(|ui| {
                ui.text_edit_singleline(current_input);
                if ui.button("Done").clicked() {
                    if let Ok(func) = current_input.parse() {
                        start_graphing_events.send(StartGraphingEvent(func));
                    }
                }
                ui.label(timer.remaining().as_secs().to_string());
            })
        });
    }
}

fn finished_ui(context: &bevy_egui::egui::Context, state: &mut GamePhase) {
    let &mut GamePhase::GameFinished(ref mut finished_state) = state else {
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
                *state = GamePhase::default();
            }
        });
}
