use std::fmt;
use std::num::NonZeroU32;

/// An error that is returned when initializing a rate limiter that is
/// too small to let a single cell through.
#[derive(Debug)]
pub struct InconsistentCapacity {
    capacity: NonZeroU32,
    cell_weight: NonZeroU32,
}

impl InconsistentCapacity {
    pub(crate) fn new(capacity: NonZeroU32, cell_weight: NonZeroU32) -> InconsistentCapacity {
        InconsistentCapacity {
            capacity,
            cell_weight,
        }
    }
}

impl fmt::Display for InconsistentCapacity {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(
            f,
            "bucket capacity {} too small for a single cell with weight {}",
            self.capacity, self.cell_weight
        )
    }
}

/// Gives additional information about the negative outcome of a batch
/// cell decision.
///
/// Since batch queries can be made for batch sizes bigger than the
/// rate limiter parameter could accomodate, there are now two
/// possible negative outcomes:
///
///   * `BatchNonConforming` - the query is valid but the Decider can
///     not accomodate them.
///
///   * `InsufficientCapacity` - the query was invalid as the rate
///     limite parameters can never accomodate the number of cells
///     queried for.
#[derive(Debug, PartialEq)]
pub enum NegativeMultiDecision<E: fmt::Display> {
    /// A batch of cells (the first argument) is non-conforming and
    /// can not be let through at this time. The second argument gives
    /// information about when that batch of cells might be let
    /// through again (not accounting for thundering herds and other,
    /// simultaneous decisions).
    BatchNonConforming(u32, E),

    /// The number of cells tested (the first argument) is larger than
    /// the bucket's capacity, which means the decision can never have
    /// a conforming result.
    InsufficientCapacity(u32),
}

impl<E> fmt::Display for NegativeMultiDecision<E>
where
    E: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            NegativeMultiDecision::BatchNonConforming(n, err) => write!(f, "{} cells: {}", n, err),
            NegativeMultiDecision::InsufficientCapacity(n) => write!(
                f,
                "bucket does not have enough capacity to accomodate {} cells",
                n
            ),
        }
    }
}
