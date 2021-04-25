use std::fmt::Debug;

use crate::{GameData, RevEffect};

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

pub trait EventListener<T: GameData> {
    fn effect_applied(&mut self, effect: Box<T::EffectType>);

    fn option_selected(&mut self, index: usize);

    fn retracted_by_n(&mut self, n: usize);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotListening {}

impl<T: GameData> EventListener<T> for NotListening {
    fn effect_applied(&mut self, _effect: Box<T::EffectType>) {}

    fn option_selected(&mut self, _index: usize) {}

    fn retracted_by_n(&mut self, _n: usize) {}
}

// TODO: some verification? E.g. player, hash?
// TODO: safe point you can reset to (via generational indizes)?
// TODO: snapshots (requires clone)?
#[derive(Debug)]
pub struct EventLog<T: GameData>
where
    T::EffectType: RevEffect<T>,
{
    log: Vec<Event<T>>,
    redo_stack: Vec<usize>,
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
}

impl<T: GameData> EventListener<T> for EventLog<T>
where
    T::EffectType: RevEffect<T>,
{
    fn effect_applied(&mut self, effect: Box<T::EffectType>) {
        self.log.push(Event::Effect(effect));
    }

    fn option_selected(&mut self, index: usize) {
        self.log.push(Event::Decision(index));
    }

    fn retracted_by_n(&mut self, mut n: usize) {
        while n > 0 {
            let top = self.log.pop().unwrap();
            n -= 1;
            assert!(top.is_decision());
        }
    }
}
