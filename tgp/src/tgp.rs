use std::marker::PhantomData;

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
    fn next_decision(&self) -> Option<Box<dyn Decision<Self>>>;
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
