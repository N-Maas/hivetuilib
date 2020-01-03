use std::{collections::VecDeque, marker::PhantomData, mem::replace};

// ----- Basic Definitions -----

// TODO: better lifetime?
// TODO: context instead of data -> adding new effects possible
// TODO: better API just using Fn?

/**
 * An effect changes the data of the game.
 */
pub trait Effect<M>: 'static {
    fn apply(&self, data: &mut M);
}

impl<M, F: Fn(&mut M) + 'static> Effect<M> for F {
    fn apply(&self, data: &mut M) {
        self(data)
    }
}

/**
 * A dispatcher maps a state to a subcomponent (module) of itself.
 */
pub trait Dispatcher<T, M>: Copy + 'static {
    fn dispatch<'a>(&self, data: &'a mut T) -> &'a mut M;
}

impl<T, M, F: Copy + 'static> Dispatcher<T, M> for F
where
    for<'a> F: Fn(&'a mut T) -> &'a mut M,
{
    fn dispatch<'a>(&self, data: &'a mut T) -> &'a mut M {
        self(data)
    }
}

pub fn build_dispatcher<T: 'static, M: 'static, F: Fn(&mut T) -> &mut M + Copy + 'static>(
    f: F,
) -> F {
    f
}

#[macro_export]
macro_rules! dispatch {
    ($type:ty) => {
        build_dispatcher(|data: &mut $type| data)
    };
    ($type:ty, $first:ident $(. $path:ident)*) => {
        build_dispatcher(|data: &mut $type| { &mut data.$first$(.$path)* })
    };
    ($type:ty, $path:tt) => {
        build_dispatcher(|data: &mut $type| { &mut data.$path })
    }
}

pub trait Remap<T, M, D: Dispatcher<T, M>, R> {
    fn remap(self, dispatcher: D) -> R;
}

pub struct EffectWrapper<T: 'static, M: 'static, E: Effect<M>, D: Dispatcher<T, M>> {
    effect: E,
    dispatcher: D,
    _t: PhantomData<T>,
    _m: PhantomData<M>,
}

impl<T: 'static, M: 'static, E: Effect<M>, D: Dispatcher<T, M>> Effect<T>
    for EffectWrapper<T, M, E, D>
{
    fn apply(&self, data: &mut T) {
        self.effect.apply(self.dispatcher.dispatch(data))
    }
}

impl<T: 'static, M: 'static, E: Effect<M>, D: Dispatcher<T, M>>
    Remap<T, M, D, EffectWrapper<T, M, E, D>> for E
{
    fn remap(self, dispatcher: D) -> EffectWrapper<T, M, E, D> {
        EffectWrapper {
            effect: self,
            dispatcher,
            _t: PhantomData,
            _m: PhantomData,
        }
    }
}

impl<T: 'static, M: 'static, D: Dispatcher<T, M>> Remap<T, M, D, Box<dyn Effect<T>>>
    for Box<dyn Effect<M>>
{
    fn remap(self, dispatcher: D) -> Box<dyn Effect<T>> {
        Box::new(move |data: &mut T| self.apply(dispatcher.dispatch(data)))
    }
}

// ----- Data Abstraction Layer -----

/**
 * An entity that makes decisions.
 */
pub trait Player {
    fn next_decision(&mut self, num_possibilities: usize) -> Option<usize>;
}

impl<F: FnMut(usize) -> Option<usize>> Player for F {
    fn next_decision(&mut self, num_possibilities: usize) -> Option<usize> {
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

/**
 * A game decision.
 */
pub trait Decision<T> {
    // panics at wrong index
    fn select_option(self: Box<Self>, index: usize) -> Box<dyn Effect<T>>;

    fn option_count(&self) -> usize;

    fn player(&self) -> usize;
}

/**
 * Interface between the data and the GameEngine.
 */
pub trait GameData: 'static {
    fn next_decision(&mut self) -> Option<Box<dyn Decision<Self>>>;
}

// Implementations

enum InternalState<T: GameData> {
    FetchEffect,
    FetchDecision,
    CachedDecision(Box<dyn Decision<T>>),
    Finished,
    Invalid,
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
            state: InternalState::FetchDecision,
            effects: VecDeque::new(),
            data,
        }
    }

    pub fn data(&self) -> &T {
        &self.data
    }

    fn forward_decision_to_player(
        &mut self,
        players: &mut [&mut dyn Player],
        decision: Box<dyn Decision<T>>,
    ) -> InternalState<T> {
        match players[decision.player()].next_decision(decision.option_count()) {
            Some(index) => {
                debug_assert!(
                    index < decision.option_count(),
                    "Illegal index returned by player - {:?}",
                    index
                );

                let effect = decision.select_option(index);
                self.effects.push_back(effect);
                InternalState::FetchEffect
            }
            None => InternalState::CachedDecision(decision),
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

        let current_state = replace(&mut self.state, InternalState::Invalid);
        self.state = match current_state {
            InternalState::FetchEffect => {
                self.effects
                    .pop_front()
                    .expect("Internal error - effects must not be empty.")
                    .apply(&mut self.data);

                if self.effects.is_empty() {
                    InternalState::FetchDecision
                } else {
                    InternalState::FetchEffect
                }
            }
            InternalState::FetchDecision => match self.data.next_decision() {
                Some(decision) => self.forward_decision_to_player(players, decision),
                None => InternalState::Finished,
            },
            InternalState::CachedDecision(decision) => {
                self.forward_decision_to_player(players, decision)
            }
            InternalState::Finished => {
                panic!("Game is finished, thus no next step possible.");
            }
            InternalState::Invalid => {
                panic!("Internal error - invalid state.");
            }
        }
    }

    fn state(&self) -> GameState {
        match self.state {
            InternalState::FetchEffect => GameState::PendingEffect,
            InternalState::FetchDecision => GameState::PendingDecision,
            InternalState::CachedDecision(_) => GameState::PendingDecision,
            InternalState::Finished => GameState::Finished,
            InternalState::Invalid => panic!("Internal error - invalid state."),
        }
    }
}

pub struct VecDecision<T> {
    options: Vec<Box<dyn Effect<T>>>,
    player: usize,
}

impl<T> VecDecision<T> {
    pub fn new(player: usize) -> Self {
        Self {
            options: Vec::new(),
            player,
        }
    }

    pub fn add_option<E: Effect<T>>(&mut self, effect: E) -> &mut Self {
        self.add_option_from_box(Box::new(effect))
    }

    pub fn add_option_from_box(&mut self, effect: Box<dyn Effect<T>>) -> &mut Self {
        self.options.push(effect);
        self
    }

    // TODO: by value?
    // pub fn select_option(&self, index: usize) -> Option<&Result<T>> {
    //     self.options.get(index).map(|x| x.as_ref())
    // }
}

impl<T> Decision<T> for VecDecision<T> {
    fn select_option(mut self: Box<Self>, index: usize) -> Box<dyn Effect<T>> {
        self.options.swap_remove(index)
    }

    fn option_count(&self) -> usize {
        self.options.len()
    }

    fn player(&self) -> usize {
        self.player
    }
}
