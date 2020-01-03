use std::{collections::VecDeque, mem};

use crate::tgp::*;

/**
 * An entity that makes decisions.
 */
pub trait Player {
    fn handle_decision(&mut self, num_possibilities: usize) -> Option<usize>;
}

impl<F: FnMut(usize) -> Option<usize>> Player for F {
    fn handle_decision(&mut self, num_possibilities: usize) -> Option<usize> {
        self(num_possibilities)
    }
}

// TODO better name
/**
 * Provides a dynamic abstraction for generically handling the game data.
 */
pub trait GameEngine {
    fn next_step(&mut self, players: &mut [&mut dyn Player]);

    fn state(&self) -> GameState;
}

pub enum GameState {
    PendingEffect,
    PendingDecision,
    Finished,
}

// Implementations

enum InternalState<T: GameData> {
    PendingEffect,
    PendingDecision(Box<dyn Decision<T>>),
    Finished,
}

pub struct Engine<T: GameData> {
    num_players: usize,
    state: InternalState<T>,
    effects: VecDeque<Box<dyn Effect<T>>>,
    data: T,
}

impl<T: GameData> Engine<T> {
    pub fn new(num_players: usize, data: T) -> Self {
        Self {
            num_players,
            state: Self::fetch_decision(&data),
            effects: VecDeque::new(),
            data,
        }
    }

    pub fn data(&self) -> &T {
        &self.data
    }

    fn fetch_decision(data: &T) -> InternalState<T> {
        match data.next_decision() {
            Some(dec) => InternalState::PendingDecision(dec),
            None => InternalState::Finished,
        }
    }
}

impl<T: GameData> GameEngine for Engine<T> {
    fn next_step(&mut self, players: &mut [&mut dyn Player]) {
        assert_eq!(
            players.len(),
            self.num_players,
            "Number of players not matching - expected {:?}, got {:?}.",
            self.num_players,
            players.len()
        );

        match &self.state {
            InternalState::PendingEffect => {
                self.effects
                    .pop_front()
                    .expect("Internal error - effects must not be empty.")
                    .apply(&mut self.data);

                if self.effects.is_empty() {
                    self.state = Self::fetch_decision(self.data());
                }
            }
            InternalState::PendingDecision(decision) => {
                let next = players[decision.player()].handle_decision(decision.option_count());

                if let Some(index) = next {
                    debug_assert!(
                        index < decision.option_count(),
                        "Illegal index returned by player: {:?}",
                        index
                    );

                    // move out the decision
                    let state = mem::replace(&mut self.state, InternalState::PendingEffect);

                    if let InternalState::PendingDecision(decision) = state {
                        let effect = decision.select_option(index);
                        self.effects.push_back(effect);
                    } else {
                        panic!("Impossible case.");
                    }
                }
            }
            InternalState::Finished => {
                panic!("Game is finished, thus no next step possible.");
            }
        }
    }

    fn state(&self) -> GameState {
        match self.state {
            InternalState::PendingEffect => GameState::PendingEffect,
            InternalState::PendingDecision(_) => GameState::PendingDecision,
            InternalState::Finished => GameState::Finished,
        }
    }
}
