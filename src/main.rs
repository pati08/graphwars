#![feature(vec_pop_if)]
#![feature(let_chains)]

use bevy::prelude::*;

mod models;
use models::*;

mod ui;
use ui::ui_system;

mod util;

mod parse;

mod systems;
use systems::graph_display::*;
use systems::util::*;

mod consts;
use consts::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(bevy_egui::EguiPlugin)
        .insert_resource(Time::new(std::time::Instant::now()))
        .insert_resource(InputCaptureState {
            keyboard_captured: false,
            pointer_captured: false,
        })
        .insert_resource(GameState::default())
        .add_event::<StartPlaying>()
        .add_event::<StartGraphingEvent>()
        .add_event::<DoneGraphingEvent>()
        .add_event::<SkipGraphingEvent>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                capture_info,
                (reset_graph, next_turn)
                    .run_if(is_turn_over)
                    .after(update_turn_timer),
                update_turn_timer,
                finish_drawing_graph.run_if(currently_graphing),
                update_turn.after(reset_graph).after(finish_drawing_graph),
                start_graphing.after(update_turn),
                ui_system.after(update_turn),
                start_playing.after(ui_system),
                draw_graph,
                draw_soldier_names,
                fade_explosions,
            ),
        )
        .run();
}

/// Tick the timer for the current turn (if one is active)
fn update_turn_timer(mut state: ResMut<GameState>, time: Res<Time>) {
    let Some(playing_state) = state.playing_state_mut() else {
        return;
    };
    if let TurnPhase::ShowPhase(TurnShowPhase::Waiting { timer }) =
        &mut playing_state.turn_phase_mut()
    {
        timer.tick(time.delta());
    }
}

/// Send a `SkipGraphingEvent` if a player's turn has expired
fn is_turn_over(
    mut events: EventReader<SkipGraphingEvent>,
    state: Res<GameState>,
) -> bool {
    let Some(playing_state) = state.playing_state() else {
        return false;
    };
    events.read().next().is_some()
        || match playing_state.turn_phase() {
            TurnPhase::ShowPhase(TurnShowPhase::Waiting { timer }) => {
                timer.finished()
            }
            _ => false,
        }
}

/// Do the processes needed to switch the turns of the players, including:
/// - Checking for a winner
/// - Going to the next soldier for the current player
/// - Switch the turn data
/// - Swap the x coordinates of all soldiers
/// - Spawn name of new player
fn next_turn(
    mut commands: Commands,
    mut state: ResMut<GameState>,
    mut soldiers: Query<(Entity, &mut Soldier, &mut Transform), With<Soldier>>,
    background: Single<Entity, With<GridBackground>>,
) {
    let Some(playing_state) = state.playing_state_mut() else {
        return;
    };

    // See if somebody won and display that they did if so
    let winner = playing_state.get_winner();
    if let Some(winner) = winner {
        state.set_finished(winner);
        // Clean up
        for soldier in soldiers.iter() {
            commands.entity(soldier.0).despawn();
        }
        commands.entity(*background).despawn();

        return;
    }

    let graphed_player = playing_state.current_player_mut();

    // Select the next soldier
    graphed_player.next_soldier();

    // Switch to the other player's turn
    playing_state.next_turn();

    // Move all soldiers
    for mut soldier in soldiers.iter_mut() {
        soldier.2.translation.x *= -1.;
        soldier.1.graph_location().x *= -1.;
    }
    playing_state.swap_soldiers();

    // Update the turn phase
    playing_state.begin_input_phase();

    let next_player = playing_state.current_player_mut();

    // Spawn the next player's name
    commands.spawn((
        Text2d::new(&next_player.name),
        CurrentPlayerText,
        Transform {
            translation: Vec3::new(0., 300., PLAYER_NAME_Z),
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        },
    ));
}

/// Despawn displays from currently graphed player
fn reset_graph(
    mut commands: Commands,
    graph: Single<Entity, With<InProgressGraph>>,
    player_name: Single<Entity, With<CurrentPlayerText>>,
) {
    commands.entity(*graph).despawn();
    commands.entity(*player_name).despawn();
}

/// Event that triggers the game to start from the setup phase
#[derive(Event)]
struct StartPlaying;

/// Transition from a setup phase to a playing phase by changing the game state
/// and spawning relevant entities
fn start_playing(
    mut events: EventReader<StartPlaying>,
    mut state: ResMut<GameState>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    if events.read().next().is_none() {
        return;
    }
    if state.start_playing().is_err() {
        return;
    }
    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(440., 440.))),
        MeshMaterial2d(materials.add(Color::WHITE)),
        Transform {
            translation: Vec3::new(0., 0., GRID_BACKGROUND_Z),
            ..Default::default()
        },
        GridBackground,
    ));
    let Some(playing_state) = state.playing_state_mut() else {
        unreachable!();
    };
    let p1_color = materials.add(Color::srgb(0., 0., 1.));
    let p2_color = materials.add(Color::srgb(1., 0., 0.));
    let mesh = meshes.add(Circle::new(SOLDIER_RADIUS));

    let (p1_soldiers, p2_soldiers) = playing_state.player_soldiers();

    for soldier in p1_soldiers.iter().chain(p2_soldiers.iter()) {
        let pos = soldier.graph_location() * 20.;
        let translation = Vec3::new(pos.x, pos.y, SOLDIER_Z);
        let bundle = SoldierBundle {
            soldier: soldier.clone(),
            transform: Transform {
                translation,
                rotation: Quat::IDENTITY,
                scale: Vec3::ONE,
            },
            mesh: Mesh2d(mesh.clone()),
            material: MeshMaterial2d(
                if let PlayerSelect::Player1 = soldier.player() {
                    p1_color.clone()
                } else {
                    p2_color.clone()
                },
            ),
        };
        commands.spawn(bundle);
    }

    commands.spawn((
        Text2d::new(&playing_state.current_player().name),
        CurrentPlayerText,
        Transform {
            translation: Vec3::new(0., 300., PLAYER_NAME_Z),
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        },
    ));
}
