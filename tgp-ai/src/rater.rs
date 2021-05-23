use std::{cmp::Ord, convert::TryFrom, slice, usize};

use tgp::{
    engine::{Engine, EventListener, GameEngine, GameState, PendingDecision},
    GameData,
};

use crate::{IndexType, RateAndMap, RatingType, INTERNAL_ERROR};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecisionType {
    HigherLevel,
    BottomLevel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Rating {
    Value(RatingType),
    Equivalency(IndexType),
    Moved(Option<IndexType>),
    None,
}

#[derive(Debug, Clone)]
pub struct Rater {
    start_index: Vec<IndexType>,
    decision_path: Vec<Box<[IndexType]>>,
    move_ratings: Vec<Rating>,
    max_rating: RatingType,
}

impl Rater {
    pub fn num_decisions(&self) -> usize {
        self.start_index.len() - 1
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

    /// exists primarily for testing purposes
    pub fn create_rating<T: GameData, L: EventListener<T>, R: RateAndMap<T>>(
        engine: &mut Engine<T, L>,
        r_a_m: &R,
    ) -> Vec<(RatingType, Box<[usize]>)>
    where
        T::Context: Clone,
    {
        let (mut rater, context_list) = Self::new(engine, |c| r_a_m.apply_type_mapping(c));
        r_a_m.rate_moves(&mut rater, &context_list, engine.data(), &[]);
        // extract the results
        let min = rater
            .move_ratings
            .iter()
            .filter_map(|&r| match r {
                Rating::Value(val) => Some(val),
                Rating::Equivalency(_) => None,
                Rating::Moved(_) => unreachable!(),
                Rating::None => panic!("Move not rated!"),
            })
            .min()
            .unwrap();
        let result = rater.cut_and_sort(min);
        result
            .into_iter()
            .map(|(val, path)| {
                (
                    val,
                    path.into_iter()
                        .map(|&i| usize::try_from(i).unwrap())
                        .collect(),
                )
            })
            .collect()
    }

    pub(crate) fn new<T: GameData, F, L: EventListener<T>>(
        engine: &mut Engine<T, L>,
        type_mapping: F,
    ) -> (Self, Vec<T::Context>)
    where
        F: Fn(&T::Context) -> DecisionType,
    {
        let mut decisions = Vec::new();
        let mut decision_path = Vec::new();
        let mut start_index = vec![0];
        let mut move_ratings = Vec::new();
        let mut start = 0;
        for_each_decision_flat(engine, type_mapping, |dec, path, context| {
            let option_count = dec.option_count();
            start += option_count;
            decisions.push(context);
            decision_path.push(Box::from(path));
            start_index.push(IndexType::try_from(start).expect("Too large index caused overflow."));
            for _ in 0..option_count {
                move_ratings.push(Rating::None);
            }
        });
        (
            Self {
                start_index,
                decision_path,
                move_ratings,
                max_rating: RatingType::MIN,
            },
            decisions,
        )
    }

    /// The result is sorted in decreasing order.
    pub(crate) fn cut_and_sort(self, min: RatingType) -> Vec<(RatingType, Box<[IndexType]>)> {
        let mut result = Vec::new();
        for i in 0..self.num_decisions() {
            let start = self.start_index[i];
            let range = self.start_index[i + 1] - start;
            for j in 0..range {
                match self.move_ratings[usize::try_from(start + j).unwrap()] {
                    Rating::Value(val) => {
                        if val >= min {
                            result.push((val, self.extended_path(i, j)));
                        }
                    }
                    Rating::Equivalency(_) => {}
                    Rating::None => panic!("Move with index {} is not rated.", i),
                    Rating::Moved(_) => panic!("{}", INTERNAL_ERROR),
                }
            }
        }
        result.sort_unstable_by(|(val1, _), (val2, _)| val2.cmp(val1));
        result
    }

    /// The result is sorted in decreasing order.
    pub(crate) fn cut_and_sort_with_equivalency(
        mut self,
        min: RatingType,
    ) -> Vec<(RatingType, Box<[IndexType]>, Vec<Box<[IndexType]>>)> {
        let mut result = Vec::new();
        for i in 0..self.num_decisions() {
            let range = self.start_index[i + 1] - self.start_index[i];
            for j in 0..range {
                self.move_rating_at(i, j, min, &mut result);
            }
        }
        result.sort_unstable_by(|(val1, _, _), (val2, _, _)| val2.cmp(val1));
        result
    }

    fn extended_path(&self, i: usize, index: IndexType) -> Box<[IndexType]> {
        let mut indizes = self.decision_path[i].as_ref().to_owned();
        indizes.push(index);
        Box::from(indizes)
    }

    fn move_rating_at(
        &mut self,
        i: usize,
        j: IndexType,
        min: RatingType,
        result: &mut Vec<(RatingType, Box<[IndexType]>, Vec<Box<[IndexType]>>)>,
    ) {
        let start = usize::try_from(self.start_index[i]).unwrap();
        let index = start + usize::try_from(j).unwrap();
        let rating = &mut self.move_ratings[index];
        match *rating {
            Rating::Value(val) => {
                if val >= min {
                    let mapped = IndexType::try_from(result.len()).unwrap();
                    *rating = Rating::Moved(Some(mapped));
                    result.push((val, self.extended_path(i, j), Vec::new()))
                } else {
                    *rating = Rating::Moved(None);
                }
            }
            Rating::Equivalency(target) => {
                let target = self.contracted_target_from_index(i, usize::try_from(target).unwrap());
                // move target if it is not moved yet
                if let Rating::Value(_) = self.move_ratings[target] {
                    let i = match self
                        .start_index
                        .binary_search(&IndexType::try_from(target).unwrap())
                    {
                        Ok(i) => i,
                        Err(i) => i - 1,
                    };
                    let j = IndexType::try_from(target).unwrap() - self.start_index[i];
                    self.move_rating_at(i, j, min, result);
                }
                // add value to list of equivalent moves
                match self.move_ratings[target] {
                    Rating::Moved(to) => {
                        if let Some(mapped) = to {
                            let (_, _, list) = &mut result[usize::try_from(mapped).unwrap()];
                            list.push(self.extended_path(i, j));
                        }
                        self.move_ratings[i] = Rating::Moved(to);
                    }
                    _ => panic!("{}", INTERNAL_ERROR),
                }
            }
            // Value of an equivalency class that is already moved
            Rating::Moved(_) => {}
            Rating::None => panic!("Move at decision {} with index {} is not rated.", i, j),
        }
    }

    fn moves_start_index(&self, dec_index: usize) -> usize {
        usize::try_from(self.start_index[dec_index]).unwrap()
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
        assert!(start + option_index < usize::try_from(self.start_index[dec_index + 1]).unwrap());
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
pub fn for_each_decision_flat<T: GameData, L: EventListener<T>, F, A>(
    engine: &mut Engine<T, L>,
    type_mapping: F,
    mut apply: A,
) where
    F: Fn(&T::Context) -> DecisionType,
    A: FnMut(&PendingDecision<T, L>, &[IndexType], T::Context),
{
    for_each_decision_flat_impl(engine, &type_mapping, &mut apply, &mut Vec::new());
}

fn for_each_decision_flat_impl<T: GameData, L: EventListener<T>, F, A>(
    engine: &mut Engine<T, L>,
    type_mapping: &F,
    apply: &mut A,
    path: &mut Vec<IndexType>,
) where
    F: Fn(&T::Context) -> DecisionType,
    A: FnMut(&PendingDecision<T, L>, &[IndexType], T::Context),
{
    let dec = pull_decision(engine, "Internal error - type mapping invalid?");
    let context = dec.context();
    match type_mapping(&context) {
        DecisionType::HigherLevel => {
            let option_count = dec.option_count();
            for i in 0..option_count {
                let index = IndexType::try_from(i).unwrap();
                path.push(index);
                pull_decision(engine, INTERNAL_ERROR).select_option(i);
                for_each_decision_flat_impl(engine, type_mapping, apply, path);
                path.pop();
                pull_decision(engine, INTERNAL_ERROR)
                    .into_follow_up_decision()
                    .expect(INTERNAL_ERROR)
                    .retract();
            }
        }
        DecisionType::BottomLevel => apply(&dec, &path, context),
    }
}

pub fn translate<T: GameData, L: EventListener<T>, F>(
    engine: &mut Engine<T, L>,
    type_mapping: F,
    index: IndexType,
) -> Vec<usize>
where
    F: Fn(&T::Context) -> DecisionType,
{
    let index = usize::try_from(index).unwrap();
    let (result, _) = translate_impl(engine, &type_mapping, index);
    if let GameState::PendingDecision(dec) = engine.pull() {
        if let Some(fu) = dec.into_follow_up_decision() {
            fu.retract_all();
        }
    }
    result.expect(INTERNAL_ERROR).into_iter().rev().collect()
}

pub fn translate_impl<T: GameData, L: EventListener<T>, F>(
    engine: &mut Engine<T, L>,
    type_mapping: &F,
    index: usize,
) -> (Option<Vec<usize>>, usize)
where
    F: Fn(&T::Context) -> DecisionType,
{
    let dec = pull_decision(engine, "Internal error - type mapping invalid?");
    let context = dec.context();
    match type_mapping(&context) {
        DecisionType::HigherLevel => {
            let mut count = 0;
            for i in 0..dec.option_count() {
                pull_decision(engine, INTERNAL_ERROR).select_option(i);
                let (result, add) = translate_impl(engine, type_mapping, index - count);
                if let Some(mut list) = result {
                    list.push(i);
                    return (Some(list), 0);
                }
                count += add;
                pull_decision(engine, INTERNAL_ERROR)
                    .into_follow_up_decision()
                    .expect("Internal error - type mapping invalid?")
                    .retract();
            }
            (None, count)
        }
        DecisionType::BottomLevel => {
            if index < dec.option_count() {
                (Some(vec![index]), 0)
            } else {
                (None, dec.option_count())
            }
        }
    }
}

fn pull_decision<'a, T: GameData, L: EventListener<T>>(
    engine: &'a mut Engine<T, L>,
    error: &str,
) -> PendingDecision<'a, T, L> {
    match engine.pull() {
        GameState::PendingDecision(dec) => dec,
        _ => panic!("{}", error),
    }
}

#[cfg(test)]
mod test {
    use tgp::engine::Engine;

    use crate::{
        rater::translate,
        test::{type_mapping, ZeroOneContext, ZeroOneGame},
    };

    use super::Rater;

    #[test]
    fn basic_test() {
        let data = ZeroOneGame::new(false, 1);
        let mut engine = Engine::new_logging(2, data);
        let (rater, _) = Rater::new(&mut engine, type_mapping);

        assert_eq!(rater.num_decisions(), 1);
        assert_eq!(rater.move_ratings.len(), 2);
    }

    #[test]
    fn translate_test() {
        let data = ZeroOneGame::new(false, 1);
        let mut engine = Engine::new_logging(2, data);

        assert_eq!(translate(&mut engine, type_mapping, 0), vec![0]);

        let data = ZeroOneGame::new(true, 2);
        let mut engine = Engine::new_logging(2, data);

        assert_eq!(translate(&mut engine, type_mapping, 1), vec![0, 1]);

        let data = ZeroOneGame::new(true, 2);
        let mut engine = Engine::new_logging(2, data);

        assert_eq!(translate(&mut engine, type_mapping, 3), vec![1, 1]);
    }

    #[test]
    fn two_level_test() {
        let data = ZeroOneGame::new(true, 2);
        let mut engine = Engine::new_logging(2, data);
        let (rater, c) = Rater::new(&mut engine, type_mapping);

        assert_eq!(rater.num_decisions(), 2);
        assert_eq!(c[0], ZeroOneContext::ZeroAnd);
        assert_eq!(c[1], ZeroOneContext::OneAnd);
        assert_eq!(rater.move_ratings.len(), 4);
    }

    #[test]
    fn cut_and_sort_test() {
        let data = ZeroOneGame::new(true, 1);
        let mut engine = Engine::new_logging(2, data);

        let (mut rater, _) = Rater::new(&mut engine, type_mapping);
        rater.rate(0, 0, 0);
        rater.rate(0, 1, 3);
        rater.rate(1, 0, 2);
        rater.rate(1, 1, 1);
        let result = rater.cut_and_sort(1);
        assert_eq!(
            result,
            vec![
                (3, Box::from([0, 1])),
                (2, Box::from([1, 0])),
                (1, Box::from([1, 1]))
            ]
        );

        let (mut rater, _) = Rater::new(&mut engine, type_mapping);
        rater.rate(0, 0, 0);
        rater.rate(0, 1, 1);
        rater.set_equivalent_to(1, 0, 0, 1);
        rater.rate(1, 1, 4);
        let result = rater.cut_and_sort_with_equivalency(1);
        assert_eq!(
            result,
            vec![
                (4, Box::from([1, 1]), Vec::new()),
                (1, Box::from([0, 1]), vec![Box::from([1, 0])])
            ]
        );
    }
}
