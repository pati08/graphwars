use crate::consts::*;
use crate::models::*;
use crate::parse;
use bevy::prelude::*;
use std::sync::Arc;
use std::time::Duration;

#[derive(Component)]
pub struct SoldierNameText;

pub fn draw_soldier_names(
    mut commands: Commands,
    soldiers: Query<(&Soldier, &Transform)>,
    soldier_names: Query<Entity, With<SoldierNameText>>,
) {
    // Despawn previous ones
    for i in soldier_names.iter() {
        commands.entity(i).despawn();
    }

    for (soldier, loc) in soldiers.iter() {
        commands.spawn((
            Text2d::new((soldier.id + 1).to_string()),
            TextColor(Color::BLACK),
            SoldierNameText,
            Transform {
                translation: loc.translation
                    + Vec3::new(0., SOLDIER_RADIUS * 2., SOLDIER_NAME_Z),
                rotation: Quat::IDENTITY,
                scale: Vec3::ONE,
            },
        ));
    }
}

pub fn currently_graphing(graph: Option<Single<&InProgressGraph>>) -> bool {
    graph.is_some()
}

pub fn finish_graphing(
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

#[derive(Component)]
pub struct ExplosionFadeTimer(Timer);

#[derive(Event)]
pub struct StartGraphing;

#[derive(Event)]
pub enum DoneGraphing {
    Failed(f32),
    Done,
}

pub fn smoothstep(x: f32) -> f32 {
    if x < 0. {
        0.
    } else if x > 1. {
        1.
    } else {
        x * x * (3. - 2. * x)
    }
}

pub fn fade_explosions(
    mut commands: Commands,
    mut explosions: Query<(Entity, &mut ExplosionFadeTimer, &mut Sprite)>,
    time: Res<Time>,
) {
    for (entity, mut timer, mut sprite) in explosions.iter_mut() {
        if timer.0.tick(time.delta()).finished() {
            commands.entity(entity).despawn();
            continue;
        }
        sprite.color =
            Color::hsva(0., 0., 1., smoothstep(1. - timer.0.fraction()));
    }
}

pub fn update_turn(
    mut commands: Commands,
    mut state: ResMut<GamePhase>,
    time: Res<Time>,
    mut graph: Option<Single<&mut InProgressGraph>>,
    mut start_graphing_events: EventWriter<StartGraphing>,
    mut finish_graphing_events: EventWriter<DoneGraphing>,
    soldiers: Query<(Entity, &Soldier)>,
    asset_server: Res<AssetServer>,
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
            let mut points = Vec::new();
            for _ in 0..timer.tick(time.delta()).times_finished_this_tick() {
                // if timer.tick(time.delta()).finished() {
                let Ok(next_y) = (function.original)(*next_x) else {
                    finish_graphing_events.send(DoneGraphing::Failed(*next_x));
                    break;
                };
                let point = Vec2::new(*next_x, next_y + function.shift_up);
                if point.y.is_nan()
                    || point.y.is_infinite()
                    || prev_y.is_some_and(|y| {
                        (y - point.y).abs()
                            > GRAPH_RES * DISCONTINUITY_THRESHOLD
                    })
                {
                    finish_graphing_events.send(DoneGraphing::Failed(point.x));
                    break;
                } else if point.x.abs() > 10. || point.y.abs() > 10. {
                    finish_graphing_events.send(DoneGraphing::Done);
                    break;
                }
                *next_x += GRAPH_RES;
                points.push(point * 20.);

                // Destroy any soldier that is hit
                *(if let PlayerSelect::Player1 = playing_state.turn {
                    &mut playing_state.player_2.living_soldiers
                } else {
                    &mut playing_state.player_1.living_soldiers
                }) = if let PlayerSelect::Player1 = playing_state.turn {
                    playing_state.player_2.living_soldiers.clone()
                } else {
                    playing_state.player_1.living_soldiers.clone()
                }
                .into_iter()
                .filter(|i| {
                    if i.graph_location.distance(point) < SOLDIER_RADIUS / 20. {
                        commands.spawn((
                            Sprite::from_image(
                                asset_server.load("explosion.png"),
                            ),
                            ExplosionFadeTimer(Timer::new(
                                Duration::from_secs(1),
                                TimerMode::Once,
                            )),
                            Transform {
                                translation: Vec3::new(
                                    i.graph_location.x * 20.,
                                    i.graph_location.y * 20.,
                                    EXPLOSION_Z,
                                ),
                                rotation: Quat::IDENTITY,
                                scale: Vec3::ONE
                                    * (EXPLOSION_SPRITE_SIZE
                                        / EXPLOSION_IMAGE_SIZE),
                            },
                        ));
                        for soldier in soldiers.iter() {
                            if soldier.1.player == i.player
                                && soldier.1.id == i.id
                            {
                                commands.entity(soldier.0).despawn();
                            }
                        }
                        false
                    } else {
                        true
                    }
                })
                .collect();
            }
            if let Some(graph) = &mut graph {
                graph.points.extend(points)
            } else {
                commands.spawn(InProgressGraph { points });
            }
        }
        TurnPhase::InputPhase { timer } => {
            if timer.tick(time.delta()).finished() {
                start_graphing_events.send(StartGraphing);
            }
        }
        _ => (),
    }
}

pub fn start_graphing(
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

    let TurnPhase::InputPhase { timer: _ } = &playing_state.turn_phase else {
        return;
    };

    let current_player = if let PlayerSelect::Player1 = playing_state.turn {
        &playing_state.player_1
    } else {
        &playing_state.player_2
    };

    let current_input =
        &current_player.living_soldiers[current_player.active_soldier].equation;
    let mut parsed_function =
        current_input.parse::<parse::ParsedFunction>().unwrap(); // TODO: Don't unwrap
    parsed_function.add_var("e", std::f32::consts::E);
    parsed_function.add_var("π", std::f32::consts::PI);
    let func = parsed_function.bind("x");

    let active_soldier_pos = current_player.living_soldiers
        [current_player.active_soldier]
        .graph_location;
    let Ok(y_start) = func(active_soldier_pos.x) else {
        finish_graphing_events.send(DoneGraphing::Failed(active_soldier_pos.x));
        return;
    };
    let offset = active_soldier_pos.y - y_start;
    // - expression.clone().bind("x").unwrap()(active_soldier_pos.x as f64)
    // as f32;
    playing_state.turn_phase = TurnPhase::ShowPhase(TurnShowPhase::Graphing {
        function: Function {
            original: Arc::new(func),
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

#[derive(Component)]
pub struct CurrentPlayerText;

pub fn draw_graph(
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

#[derive(Component)]
pub struct GridBackground;
