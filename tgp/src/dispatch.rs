use std::marker::PhantomData;

use crate::Effect;

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
