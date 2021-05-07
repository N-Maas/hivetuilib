use std::convert::TryFrom;

use tgp::{
    engine::{logging::EventLog, Engine, GameEngine},
    GameData, RevEffect,
};

use crate::{DecIndex, IndexType, RatingType, INTERNAL_ERROR};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TreeIndex(usize, usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TreeEntry {
    pub dec_index: DecIndex,
    pub rating: RatingType,
    pub num_children: IndexType,
}

pub(crate) struct SearchTreeState {
    tree: Vec<Vec<TreeEntry>>,
    next_level: Option<Vec<TreeEntry>>,
}

impl SearchTreeState {
    pub fn new() -> Self {
        Self {
            tree: Vec::new(),
            next_level: None,
        }
    }

    pub fn depth(&self) -> usize {
        self.tree.len()
    }

    pub fn add_level(&mut self) {
        assert!(self.next_level.is_none());
        self.next_level = Some(Vec::new());
    }

    pub fn push_child(&mut self, leaf: Option<TreeIndex>, dec_index: DecIndex, rating: RatingType) {
        self.next_level
            .as_mut()
            .expect(INTERNAL_ERROR)
            .push(TreeEntry {
                dec_index,
                rating,
                num_children: 0,
            });
        if let Some(index) = leaf {
            assert!(index.0 + 1 == self.depth());
            self.entry_mut(index).num_children += 1;
        }
    }

    pub fn set_base_level(&mut self) {
        assert!(
            self.depth() == 0 && self.next_level.is_some(),
            "{}",
            INTERNAL_ERROR
        );
        self.tree.push(self.next_level.take().unwrap());
    }

    /// f must return the engine in the same state as before
    pub fn for_each_leaf<T: GameData, F>(
        &mut self,
        engine: &mut Engine<T, EventLog<T>>,
        function: F,
    ) where
        T::EffectType: RevEffect<T>,
        F: Fn(&mut Engine<T, EventLog<T>>, TreeIndex),
    {
        assert!(self.depth() > 0);
        let mut children_start = vec![0; self.depth()];
        for index in 0..self.get_level(0).len() {
            let entry = self.get_level(0)[index];
            self.for_each_leaf_recursive(
                engine,
                &function,
                TreeIndex(0, index),
                &mut children_start,
            );
            children_start[0] += usize::try_from(entry.num_children).unwrap();
        }
    }

    fn for_each_leaf_recursive<T: GameData, F>(
        &mut self,
        engine: &mut Engine<T, EventLog<T>>,
        function: F,
        index: TreeIndex,
        children_start: &mut Vec<usize>,
    ) where
        T::EffectType: RevEffect<T>,
        F: Fn(&mut Engine<T, EventLog<T>>, TreeIndex),
    {
        let entry = self.entry(index);
        // TODO
        match engine.pull() {
            tgp::engine::GameState::PendingDecision(dec) => {
                // TODO
            }
            _ => panic!("{}", INTERNAL_ERROR),
        }
        if index.0 + 1 == self.depth() {
            function(engine, index);
        } else {
        }
        // TODO: reset
    }

    fn get_level(&self, index: usize) -> &[TreeEntry] {
        &self.tree.get(index).expect(INTERNAL_ERROR)
    }

    fn entry(&self, index: TreeIndex) -> TreeEntry {
        self.tree[index.0][index.1]
    }

    fn entry_mut(&mut self, index: TreeIndex) -> &mut TreeEntry {
        &mut self.tree[index.0][index.1]
    }
}
