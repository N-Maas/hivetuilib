use std::ops::RangeFrom;

use crate::RatingType;

/// Note that all depths are measured in 2-steps, i.e. a +1 in depth
/// corresponds to an additional move of both players.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Params {
    // general parameters
    pub depth: usize,
    pub first_cut_delay_depth: usize,
    pub first_move_added_delay_depth: usize,

    pub sliding: SlidingParams,
    // TODO: scale limits for larger tree
    // pub tail_to_first_cut_depth: usize,
    // pub branch_limit_long_tail: usize,
}

// TODO: proper builder pattern?
impl Params {
    pub fn integrity_check(&self) {
        assert!(self.depth > 0);
        assert!(self.first_cut_delay_depth > 0);
        assert!(self.first_cut_delay_depth + self.first_move_added_delay_depth <= self.depth);
        self.sliding
            .integrity_check(self.depth, self.first_cut_delay_depth);
    }

    pub fn new(depth: usize, sliding: SlidingParams) -> Self {
        Self {
            depth,
            first_cut_delay_depth: usize::min(2, depth),
            first_move_added_delay_depth: 0,
            sliding,
        }
    }
}

// #[derive(Debug, Clone, PartialEq, Eq)]
// pub struct DifferenceParams {
//     pub noticable: RatingType,
//     pub probably_worse: RatingType,
//     pub surely_worse: RatingType,
// }

// impl DifferenceParams {
//     pub fn new(probably_worse: RatingType) -> Self {
//         Self {
//             noticable: (probably_worse + 3) / 4,
//             probably_worse,
//             surely_worse: 2 * probably_worse,
//         }
//     }

//     pub fn detailed(
//         noticable: RatingType,
//         probably_worse: RatingType,
//         surely_worse: RatingType,
//     ) -> Self {
//         Self {
//             noticable,
//             probably_worse,
//             surely_worse,
//         }
//     }

//     pub fn integrity_check(&self) {
//         assert!(self.surely_worse >= self.probably_worse);
//         assert!(self.probably_worse >= self.noticable);
//     }
// }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlidingParams {
    pub branch_cut_limit: Vec<usize>,
    pub branch_cut_difference: Vec<RatingType>,
    pub move_limit: Vec<usize>,
    pub move_cut_difference: Vec<RatingType>,
    pub equivalency_class_limit: Vec<usize>,
}

impl SlidingParams {
    pub fn new(
        branch_cut_limit: Vec<usize>,
        branch_cut_difference: Vec<RatingType>,
        move_limit: Vec<usize>,
        move_cut_difference: Vec<RatingType>,
        equivalency_class_limit: Vec<usize>,
    ) -> Self {
        Self {
            branch_cut_limit,
            branch_cut_difference,
            move_limit,
            move_cut_difference,
            equivalency_class_limit,
        }
    }

    pub fn with_defaults(
        depth: usize,
        first_cut_delay_depth: usize,
        branch_cut_limit: usize,
        move_limit: usize,
        branch_difference_probable: RatingType,
        move_difference_probable: RatingType,
        equivalency_class_limit: usize,
    ) -> Self {
        assert!(depth > 0);
        let reduced_depth = usize::max(2, 2 * (depth.saturating_sub(first_cut_delay_depth) + 1));
        let mut branch_cut_limit_vec = Vec::new();
        branch_cut_limit_vec.push(2 * branch_cut_limit);
        branch_cut_limit_vec.resize(reduced_depth, branch_cut_limit);
        let mut branch_cut_difference = Vec::new();
        branch_cut_difference.push(2 * branch_difference_probable);
        branch_cut_difference.resize(reduced_depth, branch_difference_probable);
        let mut move_limit_vec = Vec::new();
        move_limit_vec.push(2 * move_limit);
        move_limit_vec.resize(2 * depth, move_limit);
        let mut move_cut_difference = Vec::new();
        move_cut_difference.push(2 * move_difference_probable);
        move_cut_difference.resize(2 * depth, move_difference_probable);
        let mut equivalency_class_limit_vec = Vec::new();
        equivalency_class_limit_vec.push(4 * equivalency_class_limit);
        equivalency_class_limit_vec.resize(reduced_depth, equivalency_class_limit);

        Self::new(
            branch_cut_limit_vec,
            branch_cut_difference,
            move_limit_vec,
            move_cut_difference,
            equivalency_class_limit_vec,
        )
    }

    pub fn integrity_check(&self, depth: usize, first_cut_delay_depth: usize) {
        let reduced_depth = usize::max(2, 2 * (depth.saturating_sub(first_cut_delay_depth) + 1));
        assert_eq!(self.branch_cut_limit.len(), reduced_depth);
        assert_eq!(self.branch_cut_difference.len(), reduced_depth);
        assert_eq!(self.move_limit.len(), 2 * depth);
        assert_eq!(self.move_cut_difference.len(), 2 * depth);
        assert_eq!(self.equivalency_class_limit.len(), reduced_depth);
    }

    pub(crate) fn get(&self, range: RangeFrom<usize>) -> Sliding<'_> {
        Sliding {
            branch_cut_limit: &self.branch_cut_limit[range.clone()],
            branch_cut_difference: &self.branch_cut_difference[range.clone()],
            move_limit: &self.move_limit[range.clone()],
            move_cut_difference: &self.move_cut_difference[range.clone()],
            equivalency_class_limit: &self.equivalency_class_limit[range.clone()],
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) struct Sliding<'a> {
    branch_cut_limit: &'a [usize],
    branch_cut_difference: &'a [RatingType],
    move_limit: &'a [usize],
    move_cut_difference: &'a [RatingType],
    equivalency_class_limit: &'a [usize],
}

impl<'a> Sliding<'a> {
    pub fn branch_cut_limit(&self) -> usize {
        self.branch_cut_limit[0]
    }

    pub fn branch_cut_difference(&self) -> RatingType {
        self.branch_cut_difference[0]
    }

    pub fn move_limit(&self) -> usize {
        self.move_limit[0]
    }

    pub fn move_cut_difference(&self) -> RatingType {
        self.move_cut_difference[0]
    }

    pub fn equivalency_class_limit(&self) -> usize {
        self.equivalency_class_limit[0]
    }

    pub fn next(&self) -> Self {
        let branch_cut_limit = if self.branch_cut_limit.is_empty() {
            &[]
        } else {
            &self.branch_cut_limit[1..]
        };
        let branch_cut_difference = if self.branch_cut_difference.is_empty() {
            &[]
        } else {
            &self.branch_cut_difference[1..]
        };
        let equivalency_class_limit = if self.equivalency_class_limit.is_empty() {
            &[]
        } else {
            &self.equivalency_class_limit[1..]
        };
        Self {
            branch_cut_limit,
            branch_cut_difference,
            move_limit: &self.move_limit[1..],
            move_cut_difference: &self.move_cut_difference[1..],
            equivalency_class_limit,
        }
    }
}
