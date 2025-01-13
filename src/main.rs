use bevy::{prelude::*, window::WindowResized};
use bevy_egui::{
    EguiContexts, EguiPlugin,
    egui::{self, Id},
};

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
    window: Single<&Window>,
    mut app_exit_events: ResMut<Events<AppExit>>,
) {
    egui::Window::new("Controls").show(contexts.ctx_mut(), |ui| {});
    egui::containers::panel::TopBottomPanel::top(Id::new("top_panel")).show(
        contexts.ctx_mut(),
        |ui| {
            if ui.button("Close").clicked() {
                app_exit_events.send(AppExit::Success);
            }
        },
    );
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

    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(440., 440.))),
        MeshMaterial2d(materials.add(Color::WHITE)),
        Transform::default(),
    ));
}
