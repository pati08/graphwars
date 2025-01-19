use bevy::prelude::*;

pub fn capture_info(
    mut input_capture_state: ResMut<InputCaptureState>,
    mut egui: bevy_egui::EguiContexts,
) {
    input_capture_state.keyboard_captured =
        egui.ctx_mut().wants_keyboard_input();
    input_capture_state.pointer_captured = egui.ctx_mut().wants_pointer_input();
}

#[derive(Resource)]
pub struct InputCaptureState {
    pub keyboard_captured: bool,
    pub pointer_captured: bool,
}

pub fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
}

pub fn currently_graphing(
    graph: Option<Single<&crate::InProgressGraph>>,
) -> bool {
    graph.is_some()
}
