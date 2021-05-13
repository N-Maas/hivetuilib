use std::{cmp::Ord, convert::TryFrom, slice};

use tgp::{
    engine::{Engine, EventListener, GameEngine, PendingDecision},
    GameData,
};

use crate::{IndexType, RatingType, INTERNAL_ERROR};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecisionType {
    HigherLevel,
    BottomLevel,
    Mixed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Rating {
    Value(RatingType),
    Equivalency(IndexType),
    Moved(Option<IndexType>),
    None,
}

#[derive(Debug, Clone)]
pub struct Rater<T: GameData> {
    decisions: Vec<(T::Context, IndexType)>,
    move_ratings: Vec<Rating>,
    max_rating: RatingType,
}

impl<T: GameData> Rater<T> {
    pub fn context(&self, index: usize) -> &T::Context {
        &self.decisions[index].0
    }

    pub fn num_decisions(&self) -> usize {
        self.decisions.len()
    }

    pub fn current_max(&self) -> RatingType {
        self.max_rating
    }

    pub fn rate(&mut self, dec_index: usize, option_index: usize, value: RatingType) {
        self.set_rating(dec_index, option_index, Rating::Value(value));
        self.max_rating = RatingType::max(self.max_rating, value);
    }

    pub fn is_rated(&self, dec_index: usize, option_index: usize) -> bool {
        let index = self.to_move_index(dec_index, option_index);
        match self.move_ratings[index] {
            Rating::Equivalency(_) | Rating::Value(_) => true,
            Rating::None => false,
            _ => panic!("{}", INTERNAL_ERROR),
        }
    }

    pub fn set_equivalent_to(
        &mut self,
        dec_index: usize,
        option_index: usize,
        dec_index_target: usize,
        option_index_target: usize,
    ) {
        let own_index = self.to_move_index(dec_index, option_index);
        let target = self.contracted_target(own_index, dec_index_target, option_index_target);
        self.set_rating(dec_index, option_index, Rating::Equivalency(target));
    }

    pub fn set_equivalent_as_representative(
        &mut self,
        dec_index: usize,
        option_index: usize,
        dec_index_target: usize,
        option_index_target: usize,
        value: RatingType,
    ) {
        let own_index = self.to_move_index(dec_index, option_index);
        let target = self.contracted_target(own_index, dec_index_target, option_index_target);
        let target = usize::try_from(target).unwrap();
        let own_index = IndexType::try_from(own_index).unwrap();

        self.set_rating(dec_index, option_index, Rating::Value(value));
        self.move_ratings[target] = Rating::Equivalency(own_index);
    }

    pub fn iter(&self) -> Iter<T> {
        Iter {
            iter: self.decisions.iter(),
        }
    }

    pub(crate) fn new<F, L: EventListener<T>>(engine: &mut Engine<T, L>, type_mapping: F) -> Self
    where
        F: Fn(&T::Context) -> DecisionType,
    {
        let mut decisions = Vec::new();
        let mut move_ratings = Vec::new();
        let mut start = 0;
        for_each_decision_flat(engine, type_mapping, |dec, context| {
            let option_count = dec.option_count();
            start += option_count;
            decisions.push((
                context,
                IndexType::try_from(start).expect("Too large index caused overflow."),
            ));
            for _ in 0..option_count {
                move_ratings.push(Rating::None);
            }
            // always continue the iteration
            false
        });
        Self {
            decisions,
            move_ratings,
            max_rating: RatingType::MIN,
        }
    }

    pub(crate) fn cut_and_sort(self, min: RatingType) -> Vec<(RatingType, IndexType)> {
        let mut result = self
            .move_ratings
            .into_iter()
            .enumerate()
            .filter_map(|(i, rating)| {
                let index = IndexType::try_from(i).unwrap();
                match rating {
                    Rating::Value(val) => Some((val, index)),
                    Rating::Equivalency(_) => None,
                    Rating::None => panic!("Move with index {} is not rated.", index),
                    Rating::Moved(_) => panic!("{}", INTERNAL_ERROR),
                }
            })
            .filter(|&(val, _)| val >= min)
            .collect::<Vec<_>>();
        result.sort_by(|(val1, _), (val2, _)| val1.cmp(val2));
        result
    }

    pub(crate) fn cut_and_sort_with_equivalency(
        mut self,
        min: RatingType,
    ) -> Vec<(RatingType, IndexType, Vec<IndexType>)> {
        let mut result = Vec::new();
        for i in 0..self.move_ratings.len() {
            self.move_rating_at(i, min, &mut result);
        }
        result.sort_by(|(val1, _, _), (val2, _, _)| val1.cmp(val2));
        result
    }

    fn move_rating_at(
        &mut self,
        i: usize,
        min: RatingType,
        result: &mut Vec<(RatingType, IndexType, Vec<IndexType>)>,
    ) {
        let index = IndexType::try_from(i).unwrap();
        let rating = &mut self.move_ratings[i];
        match *rating {
            Rating::Value(val) => {
                if val >= min {
                    let mapped = IndexType::try_from(result.len()).unwrap();
                    *rating = Rating::Moved(Some(mapped));
                    result.push((val, index, Vec::new()))
                } else {
                    *rating = Rating::Moved(None);
                }
            }
            Rating::Equivalency(target) => {
                let target = self.contracted_target_from_index(i, usize::try_from(target).unwrap());
                // move target if it is not moved yet
                if let Rating::Value(_) = self.move_ratings[target] {
                    self.move_rating_at(target, min, result);
                }
                // add value to list of equivalent moves
                match self.move_ratings[target] {
                    Rating::Moved(to) => {
                        to.map(|mapped| {
                            let (_, _, list) = &mut result[usize::try_from(mapped).unwrap()];
                            list.push(index);
                        });
                        self.move_ratings[i] = Rating::Moved(to);
                    }
                    _ => panic!("{}", INTERNAL_ERROR),
                }
            }
            // Value of an equivalency class that is already moved
            Rating::Moved(_) => {}
            Rating::None => panic!("Move with index {} is not rated.", index),
        }
    }

    fn moves_start_index(&self, dec_index: usize) -> usize {
        if dec_index == 0 {
            0
        } else {
            usize::try_from(self.decisions[dec_index - 1].1).unwrap()
        }
    }

    fn set_rating(&mut self, dec_index: usize, option_index: usize, value: Rating) {
        let index = self.to_move_index(dec_index, option_index);
        let rating = &mut self.move_ratings[index];
        match rating {
            Rating::None => {
                *rating = value;
            }
            _ => panic!(
                "Option {} for decision with index {} is already rated!",
                option_index, dec_index
            ),
        }
    }

    fn to_move_index(&self, dec_index: usize, option_index: usize) -> usize {
        let start = self.moves_start_index(dec_index);
        assert!(start + option_index < usize::try_from(self.decisions[dec_index].1).unwrap());
        start + option_index
    }

    fn contracted_target(
        &self,
        own_index: usize,
        dec_index_target: usize,
        option_index_target: usize,
    ) -> IndexType {
        let target_index = self.to_move_index(dec_index_target, option_index_target);
        IndexType::try_from(self.contracted_target_from_index(own_index, target_index)).unwrap()
    }

    fn contracted_target_from_index(&self, own_index: usize, mut target_index: usize) -> usize {
        loop {
            match self.move_ratings[target_index] {
                Rating::Value(_) | Rating::Moved(_) => break target_index,
                Rating::Equivalency(parent) => {
                    let parent = usize::try_from(parent).unwrap();
                    assert_ne!(own_index, parent, "Cycle in equivalencies!");
                    target_index = parent;
                }
                Rating::None => panic!("Equivalent move already must be initialized."),
            }
        }
    }
}

pub struct Iter<'a, T: GameData> {
    iter: slice::Iter<'a, (T::Context, IndexType)>,
}

impl<'a, T: GameData> Iterator for Iter<'a, T> {
    type Item = &'a T::Context;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(context, _)| context)
    }
}

/// If apply returns true, the iteration is stopped
pub(crate) fn for_each_decision_flat<T: GameData, L: EventListener<T>, F, A>(
    engine: &mut Engine<T, L>,
    type_mapping: F,
    mut apply: A,
) where
    F: Fn(&T::Context) -> DecisionType,
    A: FnMut(&PendingDecision<T, L>, T::Context) -> bool,
{
    for_each_decision_flat_impl(engine, &type_mapping, &mut apply);
}

fn for_each_decision_flat_impl<T: GameData, L: EventListener<T>, F, A>(
    engine: &mut Engine<T, L>,
    type_mapping: &F,
    apply: &mut A,
) -> bool
where
    F: Fn(&T::Context) -> DecisionType,
    A: FnMut(&PendingDecision<T, L>, T::Context) -> bool,
{
    let dec = pull_decision(engine);
    let context = dec.context();
    match type_mapping(&context) {
        DecisionType::HigherLevel => {
            let option_count = dec.option_count();
            for i in 0..option_count {
                pull_decision(engine).select_option(i);
                let stop = for_each_decision_flat_impl(engine, type_mapping, apply);
                if stop {
                    return true;
                }
                pull_decision(engine)
                    .into_follow_up_decision()
                    .expect(INTERNAL_ERROR)
                    .retract();
            }
            false
        }
        DecisionType::BottomLevel => apply(&dec, context),
        DecisionType::Mixed => panic!("Mixed decisions are not supported yet by the AI."),
    }
}

fn pull_decision<T: GameData, L: EventListener<T>>(
    engine: &mut Engine<T, L>,
) -> PendingDecision<T, L> {
    match engine.pull() {
        tgp::engine::GameState::PendingDecision(dec) => dec,
        _ => panic!("{}", INTERNAL_ERROR),
    }
}

#[cfg(test)]
mod test {
    use tgp::engine::Engine;

    use crate::test::{type_mapping, ZeroOneContext, ZeroOneGame};

    use super::Rater;

    #[test]
    fn basic_test() {
        let data = ZeroOneGame::new(false, 1);
        let mut engine = Engine::new_logging(2, data);
        let rater = Rater::new(&mut engine, type_mapping);

        assert_eq!(rater.num_decisions(), 1);
        assert_eq!(rater.move_ratings.len(), 2);
    }

    #[test]
    fn two_level_test() {
        let data = ZeroOneGame::new(true, 2);
        let mut engine = Engine::new_logging(2, data);
        let rater = Rater::new(&mut engine, type_mapping);

        assert_eq!(rater.num_decisions(), 2);
        assert_eq!(rater.decisions[0].0, ZeroOneContext::ZeroAnd);
        assert_eq!(rater.decisions[1].0, ZeroOneContext::OneAnd);
        assert_eq!(rater.move_ratings.len(), 4);
    }

    #[test]
    fn cut_and_sort_test() {
        let data = ZeroOneGame::new(true, 1);
        let mut engine = Engine::new_logging(2, data);

        let mut rater = Rater::new(&mut engine, type_mapping);
        rater.rate(0, 0, 0);
        rater.rate(0, 1, 3);
        rater.rate(1, 0, 2);
        rater.rate(1, 1, 1);
        let result = rater.cut_and_sort(1);
        assert_eq!(result, vec![(1, 3), (2, 2), (3, 1)]);

        let mut rater = Rater::new(&mut engine, type_mapping);
        rater.rate(0, 0, 0);
        rater.rate(0, 1, 1);
        rater.set_equivalent_to(1, 0, 0, 1);
        rater.rate(1, 1, 4);
        let result = rater.cut_and_sort_with_equivalency(1);
        assert_eq!(result, vec![(1, 1, vec![2]), (4, 3, Vec::new())]);
    }
}
