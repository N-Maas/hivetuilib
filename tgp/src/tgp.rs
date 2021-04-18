use crate::{Decision, Effect, GameData};

pub struct VecDecision<T: GameData> {
    options: Vec<Box<dyn Effect<T>>>,
    player: usize,
    context: T::Context,
}

// TODO: graceful context handling
impl<T: GameData> VecDecision<T> {
    pub fn new(player: usize, context: T::Context) -> Self {
        Self {
            options: Vec::new(),
            player,
            context,
        }
    }

    pub fn add_option<E: Effect<T> + 'static>(&mut self, effect: E) -> &mut Self {
        self.add_option_from_box(Box::new(effect))
    }

    pub fn add_option_from_box(&mut self, effect: Box<dyn Effect<T>>) -> &mut Self {
        self.options.push(effect);
        self
    }

    pub fn context_mut(&mut self) -> &mut T::Context {
        &mut self.context
    }
}

impl<T: GameData> Decision<T> for VecDecision<T> {
    fn select_option(mut self: Box<Self>, _data: &T, index: usize) -> Box<dyn Effect<T>> {
        self.options.swap_remove(index)
    }

    fn option_count(&self) -> usize {
        self.options.len()
    }

    fn player(&self) -> usize {
        self.player
    }

    fn context(&self, _data: &T) -> &T::Context {
        &self.context
    }
}
