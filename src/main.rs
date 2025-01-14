use std::time::Duration;

use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPlugin};
use rand::{Rng, thread_rng};

mod models;
use models::*;

mod ui;
use ui::ui_system;

const SOLDIER_RADIUS: f32 = 12.;
const ACTIVE_SOLDIER_OUTLINE_COLOR: Color = Color::srgb(0., 1., 0.);
const GRAPH_RES: f32 = 0.05;
const GRAPHING_SPEED: f32 = 15.;
const DEFAULT_FUNCTION: &str = "x";
const DISCONTINUITY_THRESHOLD: f32 = 15.;
const AFTER_GRAPH_PAUSE: Duration = Duration::from_secs(1);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin)
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
            ),
        )
        .run();
}

fn currently_graphing(graph: Option<Single<&InProgressGraph>>) -> bool {
    graph.is_some()
}

fn finish_graphing(
    mut events: EventReader<DoneGraphing>,
    mut state: ResMut<GamePhase>,
) {
    match events.read().next() {
        Some(DoneGraphing::Failed(fail_x)) => {
            log::info!("Func failed at {fail_x}")
        }
        None => return,
        _ => (),
    };

    let &mut GamePhase::Playing(ref mut playing_state) = &mut *state else {
        return;
    };

    playing_state.turn_phase = TurnPhase::ShowPhase(TurnShowPhase::Waiting {
        timer: Timer::new(AFTER_GRAPH_PAUSE, TimerMode::Once),
    });
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
        input: DEFAULT_FUNCTION.to_string(),
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
            translation: Vec3::new(0., 300., 15.),
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        },
    ));
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
        TurnPhase::ShowPhase(TurnShowPhase::Graphing {
            function,
            prev_y,
            next_x,
            timer,
        }) => {
            if timer.tick(time.delta()).just_finished() {
                let Ok(next_y) = function.original.solve_float(Some(
                    &[("x".to_string(), next_x.to_string())]
                        .into_iter()
                        .collect(),
                )) else {
                    finish_graphing_events.send(DoneGraphing::Failed(*next_x));
                    return;
                };
                let point =
                    Vec2::new(*next_x, next_y as f32 + function.shift_up);
                if let Some(mut graph) = graph {
                    graph.points.push(point * 20.)
                } else {
                    commands.spawn(InProgressGraph {
                        points: vec![point * 20.],
                    });
                }
                if point.y.is_nan()
                    || point.y.is_infinite()
                    || prev_y.is_some_and(|y| {
                        (y - point.y).abs()
                            > GRAPH_RES * DISCONTINUITY_THRESHOLD
                    })
                {
                    finish_graphing_events.send(DoneGraphing::Failed(point.x));
                } else if point.x.abs() > 10. || point.y.abs() > 10. {
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
        _ => (),
    }
}

fn start_graphing(
    mut state: ResMut<GamePhase>,
    mut events: EventReader<StartGraphing>,
    mut finish_graphing_events: EventWriter<DoneGraphing>,
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
    let re = regex::Regex::new(r"(-?\d+(?:\.\d+)?)x").unwrap();
    let input = re
        .replace_all(input, |caps: &regex::Captures| {
            format!("{} * x", &caps[1])
        })
        .to_string();

    let expression = math_parse::MathParse::parse(&input).unwrap(); // TODO: not this
    let current_player = if let PlayerSelect::Player1 = playing_state.turn {
        &playing_state.player_1
    } else {
        &playing_state.player_2
    };
    let active_soldier_pos = current_player.living_soldiers
        [current_player.active_soldier]
        .graph_location;
    let Ok(y_start) = expression.solve_float(Some(
        &[("x".to_string(), active_soldier_pos.x.to_string())]
            .into_iter()
            .collect(),
    )) else {
        finish_graphing_events.send(DoneGraphing::Failed(active_soldier_pos.x));
        return;
    };
    let offset = active_soldier_pos.y - y_start as f32;
    // - expression.clone().bind("x").unwrap()(active_soldier_pos.x as f64)
    // as f32;
    playing_state.turn_phase = TurnPhase::ShowPhase(TurnShowPhase::Graphing {
        function: Function {
            original: expression,
            shift_up: offset,
        },
        prev_y: None,
        next_x: active_soldier_pos.x,
        timer: Timer::new(
            Duration::from_secs_f32(GRAPH_RES / GRAPHING_SPEED),
            TimerMode::Repeating,
        ),
    });
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

    for soldier in playing_state
        .player_1
        .living_soldiers
        .iter()
        .chain(playing_state.player_2.living_soldiers.iter())
    {
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
            translation: Vec3::new(0., 300., 15.),
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        },
    ));
}

#[derive(Component)]
struct CurrentPlayerText;

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
    let mut rng = thread_rng();
    let mut soldiers = Vec::with_capacity(num.into());
    while soldiers.len() < num.into() {
        let new_soldier = {
            let x = rng.gen_range(0.0..10.0);
            let y = rng.gen_range(-10.0..10.0);
            let pos = Vec2 { x, y };
            Soldier {
                player,
                id: num,
                graph_location: pos,
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

#[derive(Component)]
struct GridBackground;

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
