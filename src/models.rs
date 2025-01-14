use bevy::prelude::*;
use std::{num::NonZeroU8, time::Duration};

#[derive(Clone, Copy, Debug)]
pub enum PlayerSelect {
    Player1,
    Player2,
}

#[derive(Resource)]
pub enum GamePhase {
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

pub struct SetupPhase {
    pub player_1: PlayerConfig,
    pub player_2: PlayerConfig,
    pub turn_seconds: u32,
}

pub struct PlayerConfig {
    pub soldier_num: NonZeroU8,
    pub name: String,
}

pub struct PlayPhase {
    pub player_1: PlayerState,
    pub player_2: PlayerState,
    pub turn: PlayerSelect,
    pub turn_phase: TurnPhase,
    pub turn_length: Duration,
}
pub enum TurnPhase {
    InputPhase { input: String, timer: Timer },
    ShowPhase(TurnShowPhase), // ShowPhase {
                              //     function: Function,
                              //     prev_y: Option<f32>,
                              //     next_x: f32,
                              //     timer: Timer,
                              // },
}
pub enum TurnShowPhase {
    Graphing {
        function: Function,
        prev_y: Option<f32>,
        next_x: f32,
        timer: Timer,
    },
    Waiting {
        timer: Timer,
    },
}
pub struct Function {
    pub original: math_parse::MathParse,
    pub shift_up: f32,
}

#[derive(Debug)]
pub struct PlayerState {
    pub name: String,
    // TODO: consider implementing this with
    // an explicitly non-empty array type to
    // convey that information in the type
    // system. For now, just know that it
    // CANNOT be empty.
    pub living_soldiers: Vec<Soldier>,
    pub active_soldier: usize,
}

#[derive(Component, Clone, Debug)]
pub struct Soldier {
    pub player: PlayerSelect,
    pub id: u8,
    pub graph_location: Vec2,
}
#[derive(Bundle)]
pub struct SoldierBundle {
    pub soldier: Soldier,
    pub transform: Transform,
    pub mesh: Mesh2d,
    pub material: MeshMaterial2d<ColorMaterial>,
}

pub struct FinishedPhase {
    pub winner: PlayerSelect,
}

#[derive(Component)]
pub struct InProgressGraph {
    pub points: Vec<Vec2>,
}
