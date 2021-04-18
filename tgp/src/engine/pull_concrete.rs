use crate::GameData;

use super::{Engine, InternalState, PDecisionState, PEffectState};

/**
 * Concrete engine trait which provides context for each decision.
 */
pub trait GameEngine {
    type Data: GameData;

    fn pull(&mut self) -> GameState<'_, Self::Data>;
}

#[derive(Debug)]
pub enum GameState<'a, T: GameData> {
    PendingEffect(PendingEffect<'a, T>),
    PendingDecision(PendingDecision<'a, T>),
    Finished(Finished<'a, T>),
}

impl<T: GameData> GameEngine for Engine<T> {
    type Data = T;

    fn pull(&mut self) -> GameState<'_, Self::Data> {
        match &self.state {
            InternalState::PEffect(_) => GameState::PendingEffect(PendingEffect { engine: self }),
            InternalState::PDecision(_) => {
                GameState::PendingDecision(PendingDecision { engine: self })
            }
            InternalState::Finished => GameState::Finished(Finished { engine: self }),
            InternalState::Invalid => panic!("Internal error - invalid state"),
        }
    }
}

#[derive(Debug)]
pub struct PendingEffect<'a, T: GameData> {
    engine: &'a mut Engine<T>,
}

impl<'a, T: GameData> PendingEffect<'a, T> {
    pub fn next_effect(self) {
        self.engine.next_effect();
    }

    pub fn all_effects(self) {
        while self.engine.next_effect().is_some() {}
    }

    pub fn data(&self) -> &T {
        self.engine.data()
    }
}

#[derive(Debug)]
pub struct PendingDecision<'a, T: GameData> {
    engine: &'a mut Engine<T>,
}

impl<'a, T: GameData> PendingDecision<'a, T> {
    pub fn select_option(self, index: usize) {
        self.engine.select_option(index)
    }

    pub fn option_count(&self) -> usize {
        self.engine.option_count()
    }

    pub fn player(&self) -> usize {
        self.engine.player()
    }

    pub fn context(&self) -> &T::Context {
        self.engine.context()
    }

    pub fn data(&self) -> &T {
        self.engine.data()
    }
}

#[derive(Debug)]
pub struct Finished<'a, T: GameData> {
    engine: &'a mut Engine<T>,
    // TODO additional information?
}

impl<'a, T: GameData> Finished<'a, T> {
    pub fn data(&self) -> &T {
        self.engine.data()
    }
}
