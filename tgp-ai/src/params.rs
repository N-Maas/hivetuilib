use crate::RatingType;

/// Note that all depths are measured in 2-steps, i.e. a +1 in depth
/// corresponds to an additional move of both players.
pub struct Params {
    // general parameters
    pub depth: usize,
    pub limit_multiplier_first_move: f32,

    // parameters for branch cutting
    pub branch_limit_first_cut: usize,
    pub branch_limit_long_tail: usize,
    // TODO: branch_limit_long_long_tail
    pub first_cut_delay_depth: usize,
    pub first_move_added_delay_depth: usize,
    pub tail_to_first_cut_depth: usize,
    pub branch_differences: DifferenceParams,

    // parameters for moves
    pub move_limit: usize,
    pub move_differences: DifferenceParams,
    // TODO: scale limits for larger tree?
}

// TODO: proper builder pattern?
impl Params {
    pub fn integrity_check(&self) {
        assert!(self.limit_multiplier_first_move >= 1.0);
        assert!(self.move_limit >= self.branch_limit_first_cut);
        assert!(self.branch_limit_first_cut >= self.branch_limit_long_tail);
        assert!(self.first_cut_delay_depth > 0);
        self.branch_differences.integrity_check();
        self.move_differences.integrity_check();
    }

    pub fn new(
        depth: usize,
        branch_limit: usize,
        branch_difference_probable: RatingType,
        move_difference_probable: RatingType,
    ) -> Self {
        Self {
            depth,
            limit_multiplier_first_move: 2.0,
            branch_limit_first_cut: branch_limit,
            branch_limit_long_tail: (branch_limit + 1) / 2,
            first_cut_delay_depth: 2,
            first_move_added_delay_depth: 0,
            tail_to_first_cut_depth: 2,
            branch_differences: DifferenceParams::new(branch_difference_probable),
            move_limit: 2 * branch_limit,
            move_differences: DifferenceParams::new(move_difference_probable),
        }
    }
}

pub struct DifferenceParams {
    pub noticable: RatingType,
    pub probably_worse: RatingType,
    pub surely_worse: RatingType,
}

impl DifferenceParams {
    pub fn new(probably_worse: RatingType) -> Self {
        Self {
            noticable: (probably_worse + 3) / 4,
            probably_worse,
            surely_worse: 2 * probably_worse,
        }
    }

    pub fn detailed(
        noticable: RatingType,
        probably_worse: RatingType,
        surely_worse: RatingType,
    ) -> Self {
        Self {
            noticable,
            probably_worse,
            surely_worse,
        }
    }

    pub fn integrity_check(&self) {
        assert!(self.surely_worse >= self.probably_worse);
        assert!(self.probably_worse >= self.noticable);
    }
}
