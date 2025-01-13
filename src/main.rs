use std::{num::NonZeroU8, time::Duration};

use bevy::{prelude::*, window::WindowResized};
use bevy_egui::{
    EguiContexts, EguiPlugin,
    egui::{self, Id},
};

enum PlayerSelect {
    Player1,
    Player2,
}

#[derive(Resource)]
enum GamePhase {
    Setup(SetupPhase),
    Playing(PlayPhase),
    GameFinished(FinishedPhase),
}

impl Default for GamePhase {
    fn default() -> Self {
        Self::Setup(SetupPhase {
            player_1: PlayerConfig {
                soldier_num: NonZeroU8::new(1).unwrap(),
                name: "Player 1".to_string(),
            },
            player_2: PlayerConfig {
                soldier_num: NonZeroU8::new(1).unwrap(),
                name: "Player 2".to_string(),
            },
            turn_seconds: 60,
        })
    }
}

struct SetupPhase {
    player_1: PlayerConfig,
    player_2: PlayerConfig,
    turn_seconds: u32,
}

struct PlayerConfig {
    soldier_num: NonZeroU8,
    name: String,
}

struct PlayPhase {
    turn_time: Duration,
    player_1: PlayerState,
    player_2: PlayerState,
    turn: PlayerSelect,
    turn_eq: String,
}
struct PlayerState {
    name: String,
    living_soldiers: Vec<Soldier>,
}

#[derive(Component)]
struct Soldier {
    player: PlayerSelect,
    id: u8,
    graph_location: Vec2,
}
#[derive(Bundle)]
struct SoldierBundle {
    soldier: Soldier,
    transform: Transform,
    mesh: Mesh2d,
    material: MeshMaterial2d<ColorMaterial>,
}

struct FinishedPhase {
    winner: PlayerSelect,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin)
        // .insert_resource(ClearColor(Color::srgb(0.9, 0.9, 0.9)))
        .insert_resource(Time::new(std::time::Instant::now()))
        .insert_resource(InputCaptureState {
            keyboard_captured: false,
            pointer_captured: false,
        })
        .insert_resource(GamePhase::default())
        // Systems that create Egui widgets should be run during the `Update` Bevy schedule,
        // or after the `EguiPreUpdateSet::BeginPass` system (which belongs to the `PreUpdate` Bevy schedule).
        .add_systems(Update, ui_system)
        .add_systems(Update, capture_info)
        .add_systems(Startup, setup)
        .add_systems(Update, draw_graph)
        .run();
}

fn draw_graph(mut gizmos: Gizmos) {
    gizmos.grid_2d(
        Isometry2d::default(),
        UVec2::new(20, 20),
        Vec2::new(20., 20.),
        Color::BLACK,
    );
    const RES: f32 = 0.01;
    gizmos.linestrip_2d(
        (0..(5. / RES).round() as usize)
            .map(|i| i as f32 * RES)
            .map(|i| Vec2 {
                x: i * 20.,
                y: i * i * 20.,
            })
            .take_while(|i| i.x.abs() <= 200. && i.y.abs() <= 200.),
        Color::srgb(1., 0., 0.),
    );
}

fn ui_system(
    mut contexts: EguiContexts,
    mut app_exit_events: ResMut<Events<AppExit>>,
    mut state: ResMut<GamePhase>,
) {
    egui::containers::panel::TopBottomPanel::top(Id::new("top_panel")).show(
        contexts.ctx_mut(),
        |ui| {
            if ui.button("Close").clicked() {
                app_exit_events.send(AppExit::Success);
            }
        },
    );
    match *state {
        GamePhase::Setup(_) => setup_ui(contexts.ctx_mut(), &mut state),
        GamePhase::Playing(_) => play_ui(contexts.ctx_mut(), &mut state),
        GamePhase::GameFinished(_) => finished_ui(contexts.ctx_mut(), &mut state),
    };
}

fn setup_ui(mut context: &mut bevy_egui::egui::Context, mut state: &mut GamePhase) {
    let &mut GamePhase::Setup(ref mut setup_state) = state else {
        return;
    };
}
fn play_ui(mut context: &mut bevy_egui::egui::Context, mut state: &mut GamePhase) {
    let &mut GamePhase::Playing(ref mut playing_state) = state else {
        return;
    };
}
fn finished_ui(mut context: &mut bevy_egui::egui::Context, mut state: &mut GamePhase) {
    let &mut GamePhase::GameFinished(ref mut finished_state) = state else {
        return;
    };
}

fn capture_info(mut input_capture_state: ResMut<InputCaptureState>, mut egui: EguiContexts) {
    input_capture_state.keyboard_captured = egui.ctx_mut().wants_keyboard_input();
    input_capture_state.pointer_captured = egui.ctx_mut().wants_pointer_input();
}

#[derive(Resource)]
struct InputCaptureState {
    keyboard_captured: bool,
    pointer_captured: bool,
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn(Camera2d);

    // commands.spawn((
    //     Mesh2d(meshes.add(Rectangle::new(440., 440.))),
    //     MeshMaterial2d(materials.add(Color::WHITE)),
    //     Transform::default(),
    // ));
}
