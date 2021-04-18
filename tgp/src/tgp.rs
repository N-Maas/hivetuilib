use crate::{Decision, Effect};

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

    pub fn add_option<E: Effect<T> + 'static>(&mut self, effect: E) -> &mut Self {
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
