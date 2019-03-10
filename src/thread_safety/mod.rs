#[cfg(feature = "std")]
mod std;
#[cfg(feature = "std")]
pub(crate) use self::std::*;

#[cfg(not(feature = "std"))]
mod no_std;
#[cfg(not(feature = "std"))]
pub(crate) use self::no_std::*;
