pub mod gcra;
mod leaky_bucket;

pub use self::gcra::GCRA;
pub use self::leaky_bucket::LeakyBucket;

/// An error that is returned when initializing a bucket that is too
/// small to let a single cell through.
#[derive(Fail, Debug)]
#[fail(display = "bucket capacity {} too small for a single cell with weight {}", capacity, weight)]
pub struct InconsistentCapacity {
    capacity: u32,
    weight: u32,
}
