use bevy::{prelude::*, window::WindowResized};
use bevy_egui::{
    EguiContexts, EguiPlugin,
    egui::{self, Id},
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin)
        .insert_resource(Time::new(std::time::Instant::now()))
        .insert_resource(TextMoveDistance(0.0))
        .insert_resource(IsMoving(false))
        .insert_resource(EditorText("".to_string()))
        .insert_resource(InputCaptureState {
            keyboard_captured: false,
            pointer_captured: false,
        })
        // Systems that create Egui widgets should be run during the `Update` Bevy schedule,
        // or after the `EguiPreUpdateSet::BeginPass` system (which belongs to the `PreUpdate` Bevy schedule).
        .add_systems(Update, ui_system)
        .add_systems(Update, capture_info)
        .add_systems(Startup, setup)
        .add_systems(Update, move_text)
        .add_systems(Update, on_resize)
        .add_systems(Update, space_toggle.after(capture_info))
        .run();
}

#[derive(Resource)]
struct TextMoveDistance(f32);

#[derive(Resource)]
struct IsMoving(bool);

#[derive(Resource)]
struct EditorText(String);

fn ui_system(
    mut contexts: EguiContexts,
    mut is_moving: ResMut<IsMoving>,
    mut text: Query<&mut Transform, With<HelloWorldText>>,
    mut moved: ResMut<TextMoveDistance>,
    mut editortext: ResMut<EditorText>,
    window: Single<&Window>,
    mut app_exit_events: ResMut<Events<AppExit>>,
) {
    egui::Window::new("Controls").show(contexts.ctx_mut(), |ui| {
        ui.checkbox(&mut is_moving.0, "Move text");
        if ui.button("Reset").clicked() {
            moved.0 = 0.;
            for mut i in text.iter_mut() {
                i.translation.x = -window.size().x / 2.;
            }
        }
        ui.text_edit_multiline(&mut editortext.0);
    });
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

fn setup(mut commands: Commands, window: Single<&Window>) {
    commands.spawn(Camera2d);
    println!("Window width: {}", window.size().x);
    commands.spawn((Text2d::new("Hello, world"), HelloWorldText, Transform {
        translation: Vec3 {
            x: -window.size().x / 2.,
            y: 0.,
            z: 0.,
        },
        rotation: Quat::default(),
        scale: Vec3::ONE,
    }));
}

fn space_toggle(
    keys: Res<ButtonInput<KeyCode>>,
    mut is_moving: ResMut<IsMoving>,
    input_capture_state: Res<InputCaptureState>,
) {
    if !input_capture_state.keyboard_captured && keys.just_pressed(KeyCode::Space) {
        is_moving.0 = !is_moving.0;
    }
}

fn on_resize(
    mut text: Query<&mut Transform, With<HelloWorldText>>,
    mut resize_reader: EventReader<WindowResized>,
    moved: Res<TextMoveDistance>,
) {
    for event in resize_reader.read() {
        for mut i in text.iter_mut() {
            i.translation.x = -event.width / 2. + moved.0;
        }
    }
}

#[derive(Component)]
struct HelloWorldText;

fn move_text(
    mut text: Query<&mut Transform, With<HelloWorldText>>,
    time: Res<Time>,
    mut moved: ResMut<TextMoveDistance>,
    is_moving: Res<IsMoving>,
) {
    if !is_moving.0 {
        return;
    }
    for mut i in text.iter_mut() {
        let move_dist = 100. * time.delta_secs_f64() as f32;
        i.translation.x += move_dist;
        moved.0 += move_dist;
    }
}
