mod pull_abstract;
mod pull_concrete;

pub use pull_abstract::*;
pub use pull_concrete::*;

use std::{
    fmt::{self, Debug},
    mem,
};

use crate::{Decision, Effect, GameData};

enum InternalState<T: GameData> {
    PEffect(Box<dyn Effect<T>>),
    PDecision(Box<dyn Decision<T>>),
    Finished,
    Invalid,
}

pub struct Engine<T: GameData> {
    state: InternalState<T>,
    data: T,
    num_players: usize,
}

impl<T: GameData> Engine<T> {
    pub fn new(num_players: usize, data: T) -> Self {
        Self {
            state: Self::fetch_decision(num_players, &data),
            data,
            num_players,
        }
    }

    pub fn data(&self) -> &T {
        &self.data
    }
}

impl<T: GameData + Debug> Debug for Engine<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let state_str = match self.state {
            InternalState::PEffect(_) => "PendingEffect",
            InternalState::PDecision(_) => "PendingDecision",
            InternalState::Finished => "Finished",
            InternalState::Invalid => "INVALID",
        };
        write!(
            f,
            "Engine {{ state: {}, data: {:?}, num_players: {:?} }}",
            state_str, &self.data, self.num_players
        )
    }
}

// ----- internal implementation -----
// TODO: transition-based API?
// internal traits for dynamic state handling
// must not be used for anything else!

trait PEffectState {
    fn next_effect(&mut self) -> Option<&mut dyn PEffectState>;
}

trait PDecisionState {
    fn select_option(&mut self, index: usize);

    fn option_count(&self) -> usize;

    fn player(&self) -> usize;

    // TODO: transition-based API?
    // fn select_option(&mut self, index: usize) -> &mut dyn PEffectState;
}

impl<T: GameData> Engine<T> {
    fn fetch_decision(num_players: usize, data: &T) -> InternalState<T> {
        match data.next_decision() {
            Some(dec) => {
                assert!(
                    dec.player() < num_players,
                    "Illegal player for decision: {:?}",
                    dec.player()
                );
                InternalState::PDecision(dec)
            }
            None => InternalState::Finished,
        }
    }

    fn take_effect(state: &mut InternalState<T>) -> Box<dyn Effect<T>> {
        let state = mem::replace(state, InternalState::Invalid);
        match state {
            InternalState::PEffect(effect) => effect,
            _ => panic!("Internal error - invalid state"),
        }
    }

    fn take_decision(state: &mut InternalState<T>) -> Box<dyn Decision<T>> {
        let state = mem::replace(state, InternalState::Invalid);
        match state {
            InternalState::PDecision(decision) => decision,
            _ => panic!("Internal error - invalid state"),
        }
    }

    fn decision(&self) -> &dyn Decision<T> {
        match &self.state {
            InternalState::PDecision(dec) => dec.as_ref(),
            _ => panic!("Internal error - invalid state"),
        }
    }

    fn context(&self) -> &T::Context {
        match &self.state {
            InternalState::PDecision(dec) => dec.context(&self.data),
            _ => panic!("Internal error - invalid state"),
        }
    }
}

impl<T: GameData> PEffectState for Engine<T> {
    fn next_effect(&mut self) -> Option<&mut dyn PEffectState> {
        Self::take_effect(&mut self.state).apply(&mut self.data);

        // TODO: multiple effects
        self.state = Self::fetch_decision(self.num_players, self.data());
        None
    }
}

impl<T: GameData> PDecisionState for Engine<T> {
    fn select_option(&mut self, index: usize) {
        assert!(
            index < self.option_count(),
            "Invalid option: {}. Only {} options available.",
            index,
            self.option_count()
        );

        let decision = Self::take_decision(&mut self.state);
        let effect = decision.select_option(&self.data, index);
        self.state = InternalState::PEffect(effect);
    }

    fn option_count(&self) -> usize {
        self.decision().option_count()
    }

    fn player(&self) -> usize {
        self.decision().player()
    }
}
