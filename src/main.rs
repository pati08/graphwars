use std::time::Duration;

use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPlugin};
use rand::{Rng, thread_rng};

mod models;
use models::*;

mod ui;
use ui::ui_system;

const SOLDIER_RADIUS: f32 = 12.;
const GRAPH_RES: f32 = 0.05;
const GRAPHING_SPEED: f32 = 5.;
const DEFAULT_FUNCTION: &str = "x";

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
        .add_event::<StartPlaying>()
        .add_event::<StartGraphing>()
        .add_event::<DoneGraphing>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                capture_info,
                finish_graphing.run_if(currently_graphing),
                update_turn.after(finish_graphing),
                start_graphing.after(update_turn),
                ui_system.after(update_turn),
                start_playing.after(ui_system),
                draw_graph,
            ),
        )
        .run();
}

fn currently_graphing(graph: Option<Single<&InProgressGraph>>) -> bool {
    graph.is_some()
}

fn finish_graphing(
    mut events: EventReader<DoneGraphing>,
    mut commands: Commands,
    mut state: ResMut<GamePhase>,
    graph: Single<Entity, With<InProgressGraph>>,
    mut soldiers: Query<(Entity, &mut Soldier, &mut Transform), With<Soldier>>,
    background: Single<Entity, With<GridBackground>>,
) {
    if events.read().next().is_none() {
        return;
    }
    commands.entity(*graph).despawn();

    let &mut GamePhase::Playing(ref mut playing_state) = &mut *state else {
        return;
    };

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
        soldier.1.graph_location.x = -soldier.1.graph_location.x;
        soldier.2.translation.x = -soldier.2.translation.x;
        dbg!(soldier.1.graph_location.x, soldier.2.translation.x);
    }

    // Update the turn phase
    playing_state.turn_phase = TurnPhase::InputPhase {
        input: DEFAULT_FUNCTION.to_string(),
        timer: Timer::new(playing_state.turn_length, TimerMode::Repeating),
    };
}

#[derive(Event)]
struct StartGraphing;

#[derive(Event)]
enum DoneGraphing {
    Failed(f32),
    Done,
}

fn update_turn(
    mut commands: Commands,
    mut state: ResMut<GamePhase>,
    time: Res<Time>,
    graph: Option<Single<&mut InProgressGraph>>,
    mut start_graphing_events: EventWriter<StartGraphing>,
    mut finish_graphing_events: EventWriter<DoneGraphing>,
) {
    let &mut GamePhase::Playing(ref mut playing_state) = &mut *state else {
        return;
    };
    match &mut playing_state.turn_phase {
        TurnPhase::ShowPhase {
            function,
            next_x,
            timer,
        } => {
            if timer.tick(time.delta()).just_finished() {
                let Ok(func) = function.original.clone().bind("x") else {
                    todo!()
                };
                let point = Vec2::new(
                    *next_x,
                    func(*next_x as f64) as f32 + function.shift_up,
                );
                if let Some(mut graph) = graph {
                    graph.points.push(point * 20.)
                } else {
                    commands.spawn(InProgressGraph {
                        points: vec![point * 20.],
                    });
                }
                if point.x.abs() > 10. || point.y.abs() > 10. {
                    finish_graphing_events.send(DoneGraphing::Done);
                }
                *next_x += GRAPH_RES;
            }
        }
        TurnPhase::InputPhase { input: _, timer } => {
            if timer.tick(time.delta()).finished() {
                start_graphing_events.send(StartGraphing);
            }
        }
    }
}

fn start_graphing(
    mut state: ResMut<GamePhase>,
    mut events: EventReader<StartGraphing>,
) {
    if events.read().next().is_none() {
        return;
    }
    let &mut GamePhase::Playing(ref mut playing_state) = &mut *state else {
        return;
    };

    let TurnPhase::InputPhase { input, timer: _ } = &playing_state.turn_phase
    else {
        return;
    };
    let expression = input.parse::<meval::Expr>().unwrap(); // TODO: not this
    let current_player = if let PlayerSelect::Player1 = playing_state.turn {
        &playing_state.player_1
    } else {
        &playing_state.player_2
    };
    let active_soldier_pos = current_player.living_soldiers
        [current_player.active_soldier]
        .graph_location;
    let offset = active_soldier_pos.y
        - expression.clone().bind("x").unwrap()(active_soldier_pos.x as f64)
            as f32;
    playing_state.turn_phase = TurnPhase::ShowPhase {
        function: Function {
            original: expression,
            shift_up: offset,
        },
        next_x: active_soldier_pos.x,
        timer: Timer::new(
            Duration::from_secs_f32(GRAPH_RES / GRAPHING_SPEED),
            TimerMode::Repeating,
        ),
    };
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
    *state = GamePhase::Playing(PlayPhase {
        player_1: PlayerState {
            name: setup_state.player_1.name.clone(),
            living_soldiers: gen_soldiers(
                PlayerSelect::Player1,
                setup_state.player_1.soldier_num.into(),
            ),
            active_soldier: 0,
        },
        player_2: PlayerState {
            name: setup_state.player_2.name.clone(),
            living_soldiers: gen_soldiers(
                PlayerSelect::Player2,
                setup_state.player_2.soldier_num.into(),
            ),
            active_soldier: 0,
        },
        turn: PlayerSelect::Player1,
        turn_phase: TurnPhase::InputPhase {
            input: DEFAULT_FUNCTION.to_string(),
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
            translation: Vec3::new(0., 0., -10.),
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

    for soldier in &playing_state.player_1.living_soldiers {
        let pos = soldier.graph_location * 20.;
        let translation = Vec3::new(pos.x, pos.y, 10.0);
        let bundle = SoldierBundle {
            soldier: soldier.clone(),
            transform: Transform {
                translation,
                rotation: Quat::IDENTITY,
                scale: Vec3::ONE,
            },
            mesh: Mesh2d(mesh.clone()),
            material: MeshMaterial2d(p1_color.clone()),
        };
        commands.spawn(bundle);
    }
    for soldier in &playing_state.player_2.living_soldiers {
        let pos = soldier.graph_location * 20.;
        let translation = Vec3::new(pos.x, pos.y, 10.0);
        let bundle = SoldierBundle {
            soldier: soldier.clone(),
            transform: Transform {
                translation,
                rotation: Quat::IDENTITY,
                scale: Vec3::ONE,
            },
            mesh: Mesh2d(mesh.clone()),
            material: MeshMaterial2d(p2_color.clone()),
        };
        commands.spawn(bundle);
    }
}

fn draw_graph(
    mut gizmos: Gizmos,
    state: Res<GamePhase>,
    graph: Option<Single<&InProgressGraph>>,
) {
    let GamePhase::Playing(_) = *state else {
        return;
    };

    gizmos
        .grid_2d(
            Isometry2d::default(),
            UVec2::new(20, 20),
            Vec2::new(20., 20.),
            Color::BLACK,
        )
        .outer_edges();

    if let Some(graph) = graph {
        gizmos.linestrip_2d(graph.points.clone(), Color::srgb(1., 0., 0.));
    }
}

fn gen_soldiers(player: PlayerSelect, num: u8) -> Vec<Soldier> {
    let mut soldiers = Vec::with_capacity(num.into());
    while soldiers.len() < num.into() {
        let new_soldier = gen_soldier(player, soldiers.len() as u8);
        if !soldiers.iter().any(|i: &Soldier| {
            new_soldier.graph_location.distance(i.graph_location) < 2.
        }) {
            soldiers.push(new_soldier);
        }
    }
    soldiers
}

#[derive(Component)]
struct GridBackground;

fn gen_soldier(player: PlayerSelect, num: u8) -> Soldier {
    let mut rng = thread_rng();
    let x = rng.gen_range(0.0..10.0)
        * match player {
            PlayerSelect::Player1 => -1.0,
            PlayerSelect::Player2 => 1.0,
        };
    let y = rng.gen_range(-10.0..10.0);
    let pos = Vec2 { x, y };
    Soldier {
        player,
        id: num,
        graph_location: pos,
    }
}

fn capture_info(
    mut input_capture_state: ResMut<InputCaptureState>,
    mut egui: EguiContexts,
) {
    input_capture_state.keyboard_captured =
        egui.ctx_mut().wants_keyboard_input();
    input_capture_state.pointer_captured = egui.ctx_mut().wants_pointer_input();
}

#[derive(Resource)]
struct InputCaptureState {
    keyboard_captured: bool,
    pointer_captured: bool,
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
}
