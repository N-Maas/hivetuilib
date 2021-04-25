use std::fmt::Debug;

use crate::{GameData, RevEffect};

use super::EventListener;

#[derive(Clone)]
pub enum Event<T: GameData> {
    Effect(Box<T::EffectType>),
    Decision(usize),
}

impl<T: GameData> Event<T> {
    pub fn is_decision(&self) -> bool {
        match self {
            Event::Effect(_) => false,
            Event::Decision(_) => true,
        }
    }
}

impl<T: GameData> Debug for Event<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Event::Effect(_) => {
                write!(f, "Event::Effect(Box<_>)")
            }
            Event::Decision(val) => write!(f, "Event::Decision({:?})", val),
        }
    }
}

// TODO: some verification? E.g. player, hash?
// TODO: safe point you can reset to (via generational indizes)?
// TODO: snapshots (requires clone) - lift RevEffect requirement?
#[derive(Debug)]
pub struct EventLog<T: GameData>
where
    T::EffectType: RevEffect<T>,
{
    log: Vec<Event<T>>,
    redo_stack: Vec<usize>,
}

impl<T: GameData> Default for EventLog<T>
where
    T::EffectType: RevEffect<T>,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T: GameData> EventLog<T>
where
    T::EffectType: RevEffect<T>,
{
    pub fn new() -> Self {
        Self {
            log: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    // TODO: correct behavior when in subdecision state?
    pub fn undo_last_decision(&mut self, data: &mut T) -> bool {
        self.pop_subdecision();
        if self.log.is_empty() {
            return false;
        }
        while let Some(Event::Effect(effect)) = self.log.pop() {
            effect.undo(data);
        }
        while let Some(&Event::Decision(val)) = self.log.last() {
            self.log.pop();
            self.redo_stack.push(val);
        }
        true
    }

    pub fn redo_available(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// The event is also pushed to the log (so don't do this a second time).
    pub fn redo_step(&mut self) -> Option<usize> {
        self.redo_stack.pop().map(|index| {
            self.log.push(Event::Decision(index));
            index
        })
    }

    fn pop_subdecision(&mut self) {
        while let Some(Event::Decision(_)) = self.log.last() {
            self.log.pop();
        }
    }
}

impl<T: GameData> EventListener<T> for EventLog<T>
where
    T::EffectType: RevEffect<T>,
{
    fn effect_applied(&mut self, effect: Box<T::EffectType>) {
        self.log.push(Event::Effect(effect));
        self.redo_stack.clear();
    }

    fn option_selected(&mut self, index: usize) {
        self.log.push(Event::Decision(index));
        self.redo_stack.clear();
    }

    fn retracted_by_n(&mut self, mut n: usize) {
        while n > 0 {
            let top = self.log.pop().unwrap();
            n -= 1;
            assert!(top.is_decision());
        }
    }
}
