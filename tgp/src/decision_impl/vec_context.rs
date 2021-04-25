use std::{iter::FromIterator, ops::Deref};

// TODO: we should be able to lift the Clone bounds with GATs
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VecContext<C: Clone, I: Clone = ()> {
    data: Vec<C>,
    inner: I,
}

impl<C: Clone, I: Clone> Default for VecContext<C, I>
where
    I: Default,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<C: Clone, I: Clone> FromIterator<C> for VecContext<C, I>
where
    I: Default,
{
    fn from_iter<T: IntoIterator<Item = C>>(iter: T) -> Self {
        Self {
            data: iter.into_iter().collect(),
            inner: Default::default(),
        }
    }
}

impl<C: Clone, I: Clone> VecContext<C, I>
where
    I: Default,
{
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            inner: Default::default(),
        }
    }
}

impl<C: Clone, I: Clone> VecContext<C, I> {
    pub fn with_inner(inner: I) -> Self {
        Self {
            data: Vec::new(),
            inner,
        }
    }

    pub fn with_data(data: Vec<C>, inner: I) -> Self {
        Self { data, inner }
    }

    pub fn inner(&self) -> &I {
        &self.inner
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn data(&mut self) -> &mut Vec<C> {
        &mut self.data
    }

    pub fn push(&mut self, el: C) {
        self.data.push(el)
    }
}

impl<C: Clone, I: Clone> Into<Vec<C>> for VecContext<C, I> {
    fn into(self) -> Vec<C> {
        self.data
    }
}

// TODO: don't know whether this is a desireable API in the long term
impl<C: Clone, I: Clone> Deref for VecContext<C, I> {
    type Target = [C];

    fn deref(&self) -> &[C] {
        &self.data
    }
}

impl<C: Clone, I: Clone> AsRef<[C]> for VecContext<C, I> {
    fn as_ref(&self) -> &[C] {
        &self.data
    }
}
