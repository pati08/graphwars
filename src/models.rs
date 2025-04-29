use bevy::prelude::*;
use std::{num::NonZeroU8, sync::Arc, time::Duration};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlayerSelect {
    Player1,
    Player2,
}

#[derive(Resource, Default)]
pub struct GameState(GamePhase);

enum GamePhase {
    Setup(SetupPhase),
    Playing(PlayPhase),
    GameFinished(FinishedPhase),
}

pub enum GamePhaseNoData {
    Setup,
    Playing,
    GameFinished,
}

impl GameState {
    pub fn playing_state(&self) -> Option<&PlayPhase> {
        match self.0 {
            GamePhase::Playing(ref state) => Some(state),
            _ => None,
        }
    }
    pub fn playing_state_mut(&mut self) -> Option<&mut PlayPhase> {
        match self.0 {
            GamePhase::Playing(ref mut state) => Some(state),
            _ => None,
        }
    }
    pub fn set_finished(&mut self, winner: PlayerSelect) {
        self.0 = GamePhase::GameFinished(FinishedPhase { winner });
    }
    pub fn setup_state(&self) -> Option<&SetupPhase> {
        match self.0 {
            GamePhase::Setup(ref state) => Some(state),
            _ => None,
        }
    }
    pub fn setup_state_mut(&mut self) -> Option<&mut SetupPhase> {
        match self.0 {
            GamePhase::Setup(ref mut state) => Some(state),
            _ => None,
        }
    }
    pub fn start_playing(&mut self) -> Result<(), ()> {
        let Some(setup_state) = self.setup_state() else {
            return Err(());
        };
        let soldiers = (
            gen_soldiers(
                PlayerSelect::Player1,
                setup_state.player_1.soldier_num.into(),
            ),
            gen_soldiers(
                PlayerSelect::Player2,
                setup_state.player_2.soldier_num.into(),
            ),
        );
        let player_1 = PlayerState::new(
            setup_state.player_1.name.clone(),
            soldiers.0.clone(),
        );
        let player_2 = PlayerState::new(
            setup_state.player_2.name.clone(),
            soldiers.1.clone(),
        );
        let playing_state = PlayPhase {
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
        };
        self.0 = GamePhase::Playing(playing_state);
        Ok(())
    }
    pub fn finished_state_mut(&mut self) -> Option<&mut FinishedPhase> {
        match self.0 {
            GamePhase::GameFinished(ref mut state) => Some(state),
            _ => None,
        }
    }
    pub fn game_phase(&self) -> GamePhaseNoData {
        match self.0 {
            GamePhase::GameFinished(_) => GamePhaseNoData::GameFinished,
            GamePhase::Setup(_) => GamePhaseNoData::Setup,
            GamePhase::Playing(_) => GamePhaseNoData::Playing,
        }
    }
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
    player_1: PlayerState,
    player_2: PlayerState,
    turn: PlayerSelect,
    turn_phase: TurnPhase,
    turn_length: Duration,
}

impl PlayPhase {
    pub fn turn_phase(&self) -> &TurnPhase {
        &self.turn_phase
    }
    pub fn turn_phase_mut(&mut self) -> &mut TurnPhase {
        &mut self.turn_phase
    }
    pub fn get_winner(&self) -> Option<PlayerSelect> {
        if self.player_2.living_soldiers.is_empty() {
            Some(PlayerSelect::Player1)
        } else if self.player_1.living_soldiers.is_empty() {
            Some(PlayerSelect::Player2)
        } else {
            None
        }
    }
    pub fn current_player(&self) -> &PlayerState {
        if self.turn == PlayerSelect::Player1 {
            &self.player_1
        } else {
            &self.player_2
        }
    }
    pub fn current_player_mut(&mut self) -> &mut PlayerState {
        if self.turn == PlayerSelect::Player1 {
            &mut self.player_1
        } else {
            &mut self.player_2
        }
    }
    pub fn other_player(&self) -> &PlayerState {
        if self.turn == PlayerSelect::Player1 {
            &self.player_2
        } else {
            &self.player_1
        }
    }
    pub fn next_turn(&mut self) {
        self.turn = if self.turn == PlayerSelect::Player1 {
            PlayerSelect::Player2
        } else {
            PlayerSelect::Player1
        }
    }
    pub fn swap_soldiers(&mut self) {
        for soldier in &mut self.player_1.living_soldiers {
            soldier.graph_location.x *= -1.;
        }
        for soldier in &mut self.player_2.living_soldiers {
            soldier.graph_location.x *= -1.;
        }
    }
    pub fn begin_input_phase(&mut self) {
        self.turn_phase = TurnPhase::InputPhase {
            timer: Timer::new(self.turn_length, TimerMode::Repeating),
        };
    }
    pub fn player_soldiers(&self) -> (&[Soldier], &[Soldier]) {
        (
            &self.player_1.living_soldiers,
            &self.player_2.living_soldiers,
        )
    }
    pub fn players_mut(&mut self) -> (&mut PlayerState, &mut PlayerState) {
        (&mut self.player_1, &mut self.player_2)
    }
}

pub enum TurnPhase {
    InputPhase { timer: Timer },
    ShowPhase(TurnShowPhase),
}

impl TurnPhase {
    pub fn is_input(&self) -> bool {
        matches!(self, TurnPhase::InputPhase { .. })
    }
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
    pub original:
        Arc<dyn Fn(f32) -> Result<f32, crate::parse::EvalError> + Send + Sync>,
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
    living_soldiers: Vec<Soldier>,
    active_soldier: u8,
}

impl PlayerState {
    // TODO: Prevent initialization with zero soldiers
    pub fn new(name: String, soldiers: Vec<Soldier>) -> PlayerState {
        Self {
            name,
            living_soldiers: soldiers,
            active_soldier: 0,
        }
    }
    pub fn next_soldier(&mut self) {
        self.active_soldier = self.living_soldiers[(self
            .living_soldiers
            .iter()
            .position(|i| i.id == self.active_soldier)
            .unwrap_or(0)
            + 1)
            % self.living_soldiers.len()]
        .id;
    }
    pub fn current_soldier(&self) -> &Soldier {
        self.living_soldiers
            .iter()
            .find(|i| i.id == self.active_soldier)
            .unwrap_or_else(|| &self.living_soldiers[0])
    }
    pub fn current_soldier_mut(&mut self) -> &mut Soldier {
        let current_is_valid = self
            .living_soldiers
            .iter()
            .any(|i| i.id == self.active_soldier);
        if current_is_valid {
            self.living_soldiers
                .iter_mut()
                .find(|i| i.id == self.active_soldier)
                .unwrap()
        } else {
            &mut self.living_soldiers[0]
        }
    }
    pub fn soldiers(&self) -> &[Soldier] {
        &self.living_soldiers
    }
    pub fn verify_active_soldier(&mut self) -> bool {
        if !self
            .living_soldiers
            .iter()
            .any(|i| i.id == self.active_soldier)
        {
            self.active_soldier = self.living_soldiers[0].id;
            true
        } else {
            false
        }
    }
    pub fn destroy_soldier(&mut self, id: u8) -> bool {
        self.living_soldiers.pop_if(|i| i.id == id).is_some()
    }
}

pub struct PlayUiData<'a> {
    pub input_ui: Option<InputUiData<'a>>,
    pub soldier_loc: Vec2,
}
pub struct InputUiData<'a> {
    pub current_input: &'a mut String,
    pub timer: &'a mut Timer,
}
impl<'a> PlayUiData<'a> {
    pub fn new(state: &'a mut PlayPhase) -> PlayUiData<'a> {
        let loc = state.current_player().current_soldier().graph_location;
        let TurnPhase::InputPhase { timer, .. } = &mut state.turn_phase else {
            return Self {
                input_ui: None,
                soldier_loc: loc,
            };
        };
        let current_player = if state.turn == PlayerSelect::Player1 {
            &mut state.player_1
        } else {
            &mut state.player_2
        };
        let soldier = current_player.current_soldier_mut();
        Self {
            input_ui: Some(InputUiData {
                current_input: &mut soldier.equation,
                timer,
            }),
            soldier_loc: loc,
        }
    }
}

#[derive(Component, Clone, Debug)]
pub struct Soldier {
    player: PlayerSelect,
    id: u8,
    graph_location: Vec2,
    pub equation: String,
}

impl PartialEq for Soldier {
    fn eq(&self, other: &Self) -> bool {
        self.player == other.player && self.id == other.id
    }
}

impl Soldier {
    pub fn graph_location(&self) -> Vec2 {
        self.graph_location
    }
    pub fn player(&self) -> PlayerSelect {
        self.player
    }
    pub fn id(&self) -> u8 {
        self.id
    }
}

fn gen_soldiers(player: PlayerSelect, num: u8) -> Vec<Soldier> {
    use rand::{Rng, thread_rng};
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
                equation: crate::consts::DEFAULT_FUNCTION.to_string(),
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
