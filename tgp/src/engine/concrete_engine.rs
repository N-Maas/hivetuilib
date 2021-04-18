use crate::GameData;

use super::{Engine, InternalState, PDecisionState, PEffectState, INTERNAL_ERROR};

/// Concrete engine trait which provides context for each decision.
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
            InternalState::PDecision(_, _) => {
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

    pub fn level_in_chain(&self) -> usize {
        self.engine.level_in_chain()
    }

    pub fn is_follow_up_decision(&self) -> bool {
        self.engine.level_in_chain() > 0
    }

    pub fn into_follow_up_decision(self) -> Option<FollowUpDecision<'a, T>> {
        if self.is_follow_up_decision() {
            Some(FollowUpDecision {
                engine: self.engine,
            })
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct FollowUpDecision<'a, T: GameData> {
    engine: &'a mut Engine<T>,
}

impl<'a, T: GameData> FollowUpDecision<'a, T> {
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

    pub fn level_in_chain(&self) -> usize {
        self.engine.level_in_chain()
    }

    /// Retracts from the current subdecision.
    pub fn retract(self) {
        assert!(self.engine.retract_n(1), INTERNAL_ERROR)
    }

    /// Retracts from n subdecisions and returns whether the retraction was successful.
    ///
    /// This is the case if and only if n <= #{pending decisions}.
    /// Otherwise, it has no effect.
    pub fn retract_n(self, n: usize) -> bool {
        self.engine.retract_n(n)
    }

    /// Retracts from all subdecisions until the root decision is reached.
    pub fn retract_all(self) {
        self.engine.retract_all()
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
