use std::time::Duration;

use bevy::prelude::*;
use rand::{Rng, thread_rng};

mod models;
use models::*;

mod ui;
use ui::ui_system;

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
        .insert_resource(GamePhase::default())
        .add_event::<StartPlaying>()
        .add_event::<StartGraphing>()
        .add_event::<DoneGraphing>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                capture_info,
                next_turn.run_if(currently_graphing),
                finish_graphing.run_if(currently_graphing),
                update_turn.after(next_turn).after(finish_graphing),
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

fn next_turn(
    mut commands: Commands,
    mut state: ResMut<GamePhase>,
    graph: Single<Entity, With<InProgressGraph>>,
    player_name: Single<Entity, With<CurrentPlayerText>>,
    mut soldiers: Query<(Entity, &mut Soldier, &mut Transform), With<Soldier>>,
    background: Single<Entity, With<GridBackground>>,
    time: Res<Time>,
) {
    let &mut GamePhase::Playing(ref mut playing_state) = &mut *state else {
        return;
    };

    let &mut TurnPhase::ShowPhase(TurnShowPhase::Waiting { ref mut timer }) =
        &mut playing_state.turn_phase
    else {
        return;
    };
    if !timer.tick(time.delta()).finished() {
        return;
    }

    commands.entity(*graph).despawn();
    commands.entity(*player_name).despawn();

    // See if somebody won and display that they did if so
    let winner = if playing_state.player_2.living_soldiers.is_empty() {
        Some(PlayerSelect::Player1)
    } else if playing_state.player_1.living_soldiers.is_empty() {
        Some(PlayerSelect::Player2)
    } else {
        None
    };
    if let Some(winner) = winner {
        *state = GamePhase::GameFinished(FinishedPhase { winner });
        // Clean up
        for soldier in soldiers.iter() {
            commands.entity(soldier.0).despawn();
        }
        commands.entity(*background).despawn();

        return;
    }

    let graphed_player = if let PlayerSelect::Player1 = playing_state.turn {
        &mut playing_state.player_1
    } else {
        &mut playing_state.player_2
    };

    // Select the next soldier
    graphed_player.active_soldier = (graphed_player.active_soldier + 1)
        % graphed_player.living_soldiers.len();

    // Switch to the other player's turn
    if let PlayerSelect::Player1 = playing_state.turn {
        playing_state.turn = PlayerSelect::Player2;
    } else {
        playing_state.turn = PlayerSelect::Player1;
    };

    // Move all soldiers
    for mut soldier in soldiers.iter_mut() {
        soldier.2.translation.x *= -1.;
        soldier.1.graph_location.x *= -1.;
    }
    for soldier in playing_state
        .player_1
        .living_soldiers
        .iter_mut()
        .chain(playing_state.player_2.living_soldiers.iter_mut())
    {
        soldier.graph_location.x *= -1.;
    }

    // Update the turn phase
    playing_state.turn_phase = TurnPhase::InputPhase {
        timer: Timer::new(playing_state.turn_length, TimerMode::Repeating),
    };

    let next_player = if let PlayerSelect::Player1 = playing_state.turn {
        &mut playing_state.player_1
    } else {
        &mut playing_state.player_2
    };

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

#[derive(Event)]
struct StartPlaying;

fn start_playing(
    mut events: EventReader<StartPlaying>,
    mut state: ResMut<GamePhase>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    if events.read().next().is_none() {
        return;
    }
    let &mut GamePhase::Setup(ref mut setup_state) = &mut *state else {
        return;
    };
    let player_1 = PlayerState {
        name: setup_state.player_1.name.clone(),
        living_soldiers: gen_soldiers(
            PlayerSelect::Player1,
            setup_state.player_1.soldier_num.into(),
        ),
        active_soldier: 0,
    };
    let player_2 = PlayerState {
        name: setup_state.player_2.name.clone(),
        living_soldiers: gen_soldiers(
            PlayerSelect::Player2,
            setup_state.player_2.soldier_num.into(),
        ),
        active_soldier: 0,
    };
    *state = GamePhase::Playing(PlayPhase {
        player_1,
        player_2,
        turn: PlayerSelect::Player1,
        turn_phase: TurnPhase::InputPhase {
            timer: Timer::new(
                Duration::from_secs(setup_state.turn_seconds.into()),
                TimerMode::Repeating,
            ),
        },
        turn_length: Duration::from_secs(setup_state.turn_seconds.into()),
    });
    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(440., 440.))),
        MeshMaterial2d(materials.add(Color::WHITE)),
        Transform {
            translation: Vec3::new(0., 0., GRID_BACKGROUND_Z),
            ..Default::default()
        },
        GridBackground,
    ));
    let &mut GamePhase::Playing(ref mut playing_state) = &mut *state else {
        unreachable!();
    };
    let p1_color = materials.add(Color::srgb(0., 0., 1.));
    let p2_color = materials.add(Color::srgb(1., 0., 0.));
    let mesh = meshes.add(Circle::new(SOLDIER_RADIUS));

    for soldier in playing_state
        .player_1
        .living_soldiers
        .iter()
        .chain(playing_state.player_2.living_soldiers.iter())
    {
        let pos = soldier.graph_location * 20.;
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
                if let PlayerSelect::Player1 = soldier.player {
                    p1_color.clone()
                } else {
                    p2_color.clone()
                },
            ),
        };
        commands.spawn(bundle);
    }

    commands.spawn((
        Text2d::new(&playing_state.player_1.name),
        CurrentPlayerText,
        Transform {
            translation: Vec3::new(0., 300., PLAYER_NAME_Z),
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        },
    ));
}

fn gen_soldiers(player: PlayerSelect, num: u8) -> Vec<Soldier> {
    let mut rng = thread_rng();
    let mut soldiers = Vec::with_capacity(num.into());
    while soldiers.len() < num.into() {
        let new_soldier = {
            let x = rng.gen_range(0.0..10.0);
            let y = rng.gen_range(-10.0..10.0);
            let pos = Vec2 { x, y };
            Soldier {
                player,
                id: soldiers.len() as u8,
                graph_location: pos,
                equation: DEFAULT_FUNCTION.to_string(),
            }
        };
        if !soldiers.iter().any(|i: &Soldier| {
            new_soldier.graph_location.distance(i.graph_location) < 2.
        }) {
            soldiers.push(new_soldier);
        }
    }
    if let PlayerSelect::Player1 = player {
        for soldier in &mut soldiers {
            soldier.graph_location.x *= -1.;
        }
    }
    soldiers
}
