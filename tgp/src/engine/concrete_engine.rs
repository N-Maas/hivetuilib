use crate::{trait_definitions::Effect, GameData, RevEffect};

use super::{
    logging::EventLog, Engine, EventListener, InternalState, NotListening, PDecisionState,
    PEffectState, INTERNAL_ERROR,
};

#[derive(Debug)]
pub enum GameState<'a, T: GameData, L: EventListener<T> = NotListening> {
    PendingEffect(PendingEffect<'a, T, L>),
    PendingDecision(PendingDecision<'a, T, L>),
    Finished(Finished<'a, T, L>),
}

impl<T: GameData, L: EventListener<T>> Engine<T, L> {
    pub fn pull(&mut self) -> GameState<'_, T, L> {
        match &self.state {
            InternalState::PEffect(_) => GameState::PendingEffect(PendingEffect { engine: self }),
            InternalState::PDecision(_, _) => {
                assert!(
                    self.option_count() > 0,
                    "At least one possible option per decision required!"
                );
                GameState::PendingDecision(PendingDecision { engine: self })
            }
            InternalState::Finished => GameState::Finished(Finished { engine: self }),
            InternalState::Invalid => panic!("Internal error - invalid state"),
        }
    }

    pub fn data(&self) -> &T {
        &self.data
    }
}

#[derive(Debug)]
pub struct PendingEffect<'a, T: GameData, L: EventListener<T> = NotListening> {
    engine: &'a mut Engine<T, L>,
}

impl<'a, T: GameData, L: EventListener<T>> PendingEffect<'a, T, L> {
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

impl<T: GameData> PendingEffect<'_, T, EventLog<T>>
where
    T::EffectType: RevEffect<T>,
{
    pub fn undo_last_decision(&mut self) -> bool {
        self.engine.undo_last_decision()
    }
}

#[derive(Debug)]
pub struct PendingDecision<'a, T: GameData, L: EventListener<T> = NotListening> {
    engine: &'a mut Engine<T, L>,
}

impl<'a, T: GameData, L: EventListener<T>> PendingDecision<'a, T, L> {
    pub fn select_option(self, index: usize) {
        self.engine.select_option(index)
    }

    pub fn option_count(&self) -> usize {
        self.engine.option_count()
    }

    pub fn player(&self) -> usize {
        self.engine.player()
    }

    pub fn context(&self) -> T::Context {
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

    pub fn into_follow_up_decision(self) -> Option<FollowUpDecision<'a, T, L>> {
        if self.is_follow_up_decision() {
            Some(FollowUpDecision {
                engine: self.engine,
            })
        } else {
            None
        }
    }

    pub fn try_into_follow_up_decision(self) -> Result<FollowUpDecision<'a, T, L>, Self> {
        if self.is_follow_up_decision() {
            Ok(FollowUpDecision {
                engine: self.engine,
            })
        } else {
            Err(self)
        }
    }
}

impl<T: GameData> PendingDecision<'_, T, EventLog<T>>
where
    T::EffectType: RevEffect<T>,
{
    pub fn undo_last_decision(&mut self) -> bool {
        self.engine.undo_last_decision()
    }

    pub fn redo_decision(self) -> bool {
        if !self.engine.listener.redo_available() {
            return false;
        }

        loop {
            let index = self.engine.listener.redo_step().expect(INTERNAL_ERROR);
            self.engine.select_and_apply_option(index);

            match &mut self.engine.state {
                InternalState::PEffect(_) => {
                    break;
                }
                InternalState::PDecision(_, _) => {}
                InternalState::Finished | InternalState::Invalid => panic!("{}", INTERNAL_ERROR),
            }
        }
        let mut effect = Some(self.engine.take_effect());
        while let Some(next) = effect {
            effect = next.apply(&mut self.engine.data);
            self.engine.listener.redo_effect(next);
        }
        self.engine.state = self.engine.fetch_next_state();
        true
    }
}

#[derive(Debug)]
pub struct FollowUpDecision<'a, T: GameData, L: EventListener<T>> {
    engine: &'a mut Engine<T, L>,
}

impl<'a, T: GameData, L: EventListener<T>> FollowUpDecision<'a, T, L> {
    pub fn select_option(self, index: usize) {
        self.engine.select_option(index)
    }

    pub fn option_count(&self) -> usize {
        self.engine.option_count()
    }

    pub fn player(&self) -> usize {
        self.engine.player()
    }

    pub fn context(&self) -> T::Context {
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
        assert!(self.engine.retract_n(1), "{}", INTERNAL_ERROR)
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

impl<T: GameData> FollowUpDecision<'_, T, EventLog<T>>
where
    T::EffectType: RevEffect<T>,
{
    pub fn undo_last_decision(&mut self) -> bool {
        self.engine.undo_last_decision()
    }
}

#[derive(Debug)]
pub struct Finished<'a, T: GameData, L: EventListener<T>> {
    engine: &'a mut Engine<T, L>,
    // TODO additional information?
}

impl<'a, T: GameData, L: EventListener<T>> Finished<'a, T, L> {
    pub fn data(&self) -> &T {
        self.engine.data()
    }
}

impl<T: GameData> Finished<'_, T, EventLog<T>>
where
    T::EffectType: RevEffect<T>,
{
    pub fn undo_last_decision(&mut self) -> bool {
        self.engine.undo_last_decision()
    }
}
