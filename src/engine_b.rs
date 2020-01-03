use std::{collections::VecDeque, mem};

use crate::tgp::*;

pub struct PendingEffect<'a> {
    state: &'a mut dyn PEffectState,
}

impl<'a> PendingEffect<'a> {
    pub fn next_effect(self) -> Option<PendingEffect<'a>> {
        self.state
            .next_effect()
            .map(|state| PendingEffect { state })
    }

    pub fn all_effects(mut self) {
        while let Some(state) = self.state.next_effect() {
            self.state = state;
        }
    }

    pub fn remaining_count(&self) -> usize {
        self.state.remaining_count()
    }
}

pub struct PendingDecision<'a> {
    // panics at wrong index
    state: &'a mut dyn PDecisionState,
}

impl<'a> PendingDecision<'a> {
    pub fn select_option(self, index: usize) -> PendingEffect<'a> {
        PendingEffect {
            state: self.state.select_option(index),
        }
    }

    pub fn option_count(&self) -> usize {
        self.state.option_count()
    }

    pub fn player(&self) -> usize {
        self.state.player()
    }
}

pub struct Finished {
    // TODO additional information?
}

pub enum GameState<'a> {
    PendingEffect(PendingEffect<'a>),
    PendingDecision(PendingDecision<'a>),
    Finished(Finished),
}

pub trait GameEngine {
    fn state(&mut self) -> GameState<'_>;

    fn select_option(&mut self, index: usize) -> PendingEffect<'_>;
}

// Implementation

enum InternalState<T: GameData> {
    PEffect(VecDeque<Box<dyn Effect<T>>>),
    PDecision(Box<dyn Decision<T>>),
    Finished,
}

pub struct Engine<T: GameData> {
    num_players: usize,
    state: InternalState<T>,
    data: T,
}

impl<T: GameData> Engine<T> {
    pub fn new(num_players: usize, data: T) -> Self {
        Self {
            num_players,
            state: Self::fetch_decision(num_players, &data),
            data,
        }
    }

    pub fn data(&self) -> &T {
        &self.data
    }
}

impl<T: GameData> GameEngine for Engine<T> {
    fn state(&mut self) -> GameState<'_> {
        match &self.state {
            InternalState::PEffect(_) => GameState::PendingEffect(PendingEffect { state: self }),
            InternalState::PDecision(_) => {
                GameState::PendingDecision(PendingDecision { state: self })
            }
            InternalState::Finished => GameState::Finished(Finished {}),
        }
    }

    fn select_option(&mut self, index: usize) -> PendingEffect<'_> {
        PendingEffect {
            state: PDecisionState::select_option(self, index),
        }
    }
}

// internal traits for dynamic state handling
// must not be used for anything else!

trait PEffectState {
    fn next_effect(&mut self) -> Option<&mut dyn PEffectState>;

    fn remaining_count(&self) -> usize;
}

trait PDecisionState {
    fn select_option(&mut self, index: usize) -> &mut dyn PEffectState;

    fn option_count(&self) -> usize;

    fn player(&self) -> usize;
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

    fn get_effects(state: &mut InternalState<T>) -> &mut VecDeque<Box<dyn Effect<T>>> {
        match state {
            InternalState::PEffect(effects) => effects,
            _ => panic!("Internal error - invalid state"),
        }
    }

    fn get_decision(&self) -> &dyn Decision<T> {
        match &self.state {
            InternalState::PDecision(dec) => dec.as_ref(),
            _ => panic!("Internal error - invalid state"),
        }
    }
}

impl<T: GameData> PEffectState for Engine<T> {
    fn next_effect(&mut self) -> Option<&mut dyn PEffectState> {
        let effects = Self::get_effects(&mut self.state);
        effects
            .pop_front()
            .expect("Internal error - no effect available")
            .apply(&mut self.data);

        if effects.is_empty() {
            self.state = Self::fetch_decision(self.num_players, self.data());
            None
        } else {
            Some(self)
        }
    }

    fn remaining_count(&self) -> usize {
        match &self.state {
            InternalState::PEffect(effects) => effects.len(),
            _ => panic!("Internal error - invalid state"),
        }
    }
}

impl<T: GameData> PDecisionState for Engine<T> {
    fn select_option(&mut self, index: usize) -> &mut dyn PEffectState {
        assert!(
            index < self.option_count(),
            "Invalid option: {}. Only {} options available.",
            index,
            self.option_count()
        );

        let state = mem::replace(
            &mut self.state,
            InternalState::PEffect(VecDeque::with_capacity(1)),
        );

        if let InternalState::PDecision(decision) = state {
            let effect = decision.select_option(index);
            Self::get_effects(&mut self.state).push_back(effect);
            self
        } else {
            panic!("Attemted to select an option at invalid state.");
        }
    }

    fn option_count(&self) -> usize {
        self.get_decision().option_count()
    }

    fn player(&self) -> usize {
        self.get_decision().player()
    }
}
