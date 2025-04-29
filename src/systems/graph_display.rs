use crate::consts::*;
use crate::models::*;
use crate::parse::ParsedFunction;
use crate::util::smoothstep;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Duration;

#[derive(Component)]
pub struct CurrentPlayerText;

#[derive(Component)]
pub struct GridBackground;

#[derive(Component)]
pub struct ExplosionFadeTimer(Timer);

#[derive(Component)]
pub struct SoldierNameText;

#[derive(Event, Clone)]
pub struct StartGraphingEvent(pub ParsedFunction);

#[derive(Event)]
pub struct SkipGraphingEvent;

#[derive(Event)]
pub enum DoneGraphingEvent {
    Failed(f32),
    Done,
}

pub fn start_graphing(
    mut state: ResMut<GameState>,
    mut events: EventReader<StartGraphingEvent>,
    mut finish_graphing_events: EventWriter<DoneGraphingEvent>,
) {
    let Some(StartGraphingEvent(mut parsed_function)) =
        events.read().next().cloned()
    else {
        return;
    };
    let Some(playing_state) = state.playing_state_mut() else {
        return;
    };

    if !playing_state.turn_phase().is_input() {
        return;
    };

    let current_player = playing_state.current_player();

    parsed_function.add_var("e", std::f32::consts::E);
    parsed_function.add_var("π", std::f32::consts::PI);
    let func = parsed_function.bind("x");

    let active_soldier_pos = current_player.current_soldier().graph_location();
    let Ok(y_start) = func(active_soldier_pos.x) else {
        finish_graphing_events
            .send(DoneGraphingEvent::Failed(active_soldier_pos.x));
        return;
    };
    let offset = active_soldier_pos.y - y_start;
    // - expression.clone().bind("x").unwrap()(active_soldier_pos.x as f64)
    // as f32;
    *playing_state.turn_phase_mut() =
        TurnPhase::ShowPhase(TurnShowPhase::Graphing {
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

pub fn finish_drawing_graph(
    mut events: EventReader<DoneGraphingEvent>,
    mut state: ResMut<GameState>,
) {
    match events.read().next() {
        Some(DoneGraphingEvent::Failed(fail_x)) => {
            log::info!("Func failed at {fail_x}")
        }
        None => return,
        _ => (),
    };

    let Some(playing_state) = state.playing_state_mut() else {
        return;
    };

    *playing_state.turn_phase_mut() =
        TurnPhase::ShowPhase(TurnShowPhase::Waiting {
            timer: Timer::new(AFTER_GRAPH_PAUSE, TimerMode::Once),
        });
}

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
            Text2d::new((soldier.id() + 1).to_string()),
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
    mut graph: Option<Single<&mut InProgressGraph>>,
    mut start_graphing_events: EventWriter<StartGraphingEvent>,
    mut finish_graphing_events: EventWriter<DoneGraphingEvent>,
    mut skip_graphing_events: EventWriter<SkipGraphingEvent>,
    soldiers: Query<(Entity, &Soldier)>,
    mut resources: UpdateTurnResources,
) {
    let Some(playing_state) = resources.state.playing_state_mut() else {
        return;
    };
    match playing_state.turn_phase_mut() {
        TurnPhase::ShowPhase(TurnShowPhase::Graphing {
            function,
            prev_y,
            next_x,
            timer,
        }) => {
            let func = Arc::clone(&function.original);
            let func_shift = function.shift_up;
            let mut points = Vec::new();
            let prev_y = *prev_y;
            let mut current_x = *next_x;
            for _ in 0..timer
                .tick(resources.time.delta())
                .times_finished_this_tick()
            {
                // if timer.tick(time.delta()).finished() {
                let Ok(next_y) = func(current_x) else {
                    finish_graphing_events
                        .send(DoneGraphingEvent::Failed(current_x));
                    break;
                };
                let point = Vec2::new(current_x, next_y + func_shift);
                if point.y.is_nan()
                    || point.y.is_infinite()
                    || prev_y.is_some_and(|y| {
                        (y - point.y).abs()
                            > GRAPH_RES * DISCONTINUITY_THRESHOLD
                    })
                {
                    finish_graphing_events
                        .send(DoneGraphingEvent::Failed(point.x));
                    break;
                } else if point.x.abs() > 10. || point.y.abs() > 10. {
                    finish_graphing_events.send(DoneGraphingEvent::Done);
                    break;
                }
                current_x += GRAPH_RES;
                points.push(point * 20.);

                #[allow(clippy::unnecessary_to_owned)]
                for i in playing_state
                    .other_player()
                    .soldiers()
                    .to_vec()
                    .into_iter()
                    .filter(|i| {
                        i.graph_location().distance(point)
                            < SOLDIER_RADIUS / 20.
                    })
                {
                    commands.spawn((
                        Sprite::from_image(
                            resources.asset_server.load("explosion.png"),
                        ),
                        ExplosionFadeTimer(Timer::new(
                            Duration::from_secs(1),
                            TimerMode::Once,
                        )),
                        Transform {
                            translation: Vec3::new(
                                i.graph_location().x * 20.,
                                i.graph_location().y * 20.,
                                EXPLOSION_Z,
                            ),
                            rotation: Quat::IDENTITY,
                            scale: Vec3::ONE
                                * (EXPLOSION_SPRITE_SIZE
                                    / EXPLOSION_IMAGE_SIZE),
                        },
                    ));
                    commands.spawn(AudioPlayer::new(
                        resources.asset_server.load("explosion.mp3"),
                    ));
                    for soldier in soldiers.iter() {
                        if soldier.1.player() == i.player()
                            && soldier.1.id() == i.id()
                        {
                            commands.entity(soldier.0).despawn();
                        }
                    }
                    playing_state.current_player_mut().destroy_soldier(i.id());
                }
                playing_state.players_mut().0.verify_active_soldier();
                playing_state.players_mut().1.verify_active_soldier();
            }
            if let Some(graph) = &mut graph {
                graph.points.extend(points)
            } else {
                commands.spawn(InProgressGraph { points });
            }
            if let TurnPhase::ShowPhase(TurnShowPhase::Graphing {
                next_x,
                ..
            }) = playing_state.turn_phase_mut()
            {
                *next_x = current_x;
            }
        }
        TurnPhase::InputPhase { timer } => {
            if timer.tick(resources.time.delta()).finished() {
                let current_player = playing_state.current_player();
                let func_input = &current_player.current_soldier().equation;
                let func = match func_input
                    .parse::<crate::parse::ParsedFunction>()
                {
                    Ok(f) => f,
                    Err(e) => {
                        skip_graphing_events.send(SkipGraphingEvent);
                        log::info!(
                            "User typed bad function. Input:\n`{func_input}`\nError:\n{e}"
                        );
                        return;
                    }
                };
                start_graphing_events.send(StartGraphingEvent(func));
            }
        }
        _ => (),
    }
}

#[derive(SystemParam)]
pub struct UpdateTurnResources<'w, 's> {
    state: ResMut<'w, GameState>,
    time: Res<'w, Time>,
    asset_server: Res<'w, AssetServer>,
    _phantom_data: PhantomData<&'s ()>,
}

pub fn draw_graph(
    mut gizmos: Gizmos,
    state: Res<GameState>,
    graph: Option<Single<&InProgressGraph>>,
) {
    if state.playing_state().is_none() {
        return;
    }
    // let GamePhase::Playing(_) = *state else {
    //     return;
    // };

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
