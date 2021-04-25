use std::marker::PhantomData;

use crate::{Effect, GameData, RevEffect};

pub fn new_effect<T, A>(apply: A) -> Box<dyn Effect<T>>
where
    T: GameData<EffectType = dyn Effect<T>> + 'static,
    A: Fn(&mut T) -> Option<Box<dyn Effect<T>>> + 'static,
{
    Box::new(EffectImpl {
        apply,
        _t: PhantomData,
    })
}

struct EffectImpl<T, A>
where
    T: GameData<EffectType = dyn Effect<T>>,
    A: Fn(&mut T) -> Option<Box<dyn Effect<T>>> + 'static,
{
    apply: A,
    _t: PhantomData<T>,
}

impl<T, A> Effect<T> for EffectImpl<T, A>
where
    T: GameData<EffectType = dyn Effect<T>>,
    A: Fn(&mut T) -> Option<Box<dyn Effect<T>>> + 'static,
{
    fn apply(&self, data: &mut T) -> Option<Box<dyn Effect<T>>> {
        (self.apply)(data)
    }
}

pub fn new_rev_effect<T, A, U>(apply: A, undo: U) -> Box<dyn RevEffect<T>>
where
    T: GameData<EffectType = dyn RevEffect<T>> + 'static,
    A: Fn(&mut T) -> Option<Box<dyn RevEffect<T>>> + 'static,
    U: Fn(&mut T) + 'static,
{
    Box::new(RevEffectImpl {
        apply,
        undo,
        _t: PhantomData,
    })
}

struct RevEffectImpl<T, A, U>
where
    T: GameData<EffectType = dyn RevEffect<T>>,
    A: Fn(&mut T) -> Option<Box<dyn RevEffect<T>>> + 'static,
    U: Fn(&mut T) + 'static,
{
    apply: A,
    undo: U,
    _t: PhantomData<T>,
}

impl<T, A, U> Effect<T> for RevEffectImpl<T, A, U>
where
    T: GameData<EffectType = dyn RevEffect<T>>,
    A: Fn(&mut T) -> Option<Box<dyn RevEffect<T>>> + 'static,
    U: Fn(&mut T) + 'static,
{
    fn apply(&self, data: &mut T) -> Option<Box<dyn RevEffect<T>>> {
        (self.apply)(data)
    }
}

impl<T, A, U> RevEffect<T> for RevEffectImpl<T, A, U>
where
    T: GameData<EffectType = dyn RevEffect<T>>,
    A: Fn(&mut T) -> Option<Box<dyn RevEffect<T>>> + 'static,
    U: Fn(&mut T) + 'static,
{
    fn undo(&self, data: &mut T) {
        (self.undo)(data)
    }
}
