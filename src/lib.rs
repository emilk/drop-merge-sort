//! Drop-Merge sort created and implemented by Emil Ernerfeldt.
//!
//! Drop-Merge sort is an adaptive, unstable sorting algorithm designed for nearly-sorted data.
//! An example use-case would be re-sorting an already sorted list after minor modifications.
//!
//! Drop-Merge sort is especially useful for:
//!
//! * Long lists (>10k elements)
//! * Where >80% of the data is already in-order
//! * The unsorted elements are evenly distributed.
//!
//! Expected number of comparisons is `O(N + K * log(K))` where `K` is the number of elements not in order.
//! Expected memory usage is `O(K)`.
//! Works best when `K < 0.2 * N`.
//! The out-of-order elements are expected to be randomly distributed (NOT clumped).
//!
//! # Examples
//! ```
//! extern crate dmsort;
//!
//! fn main() {
//! 	let mut numbers : Vec<i32> = vec!(0, 1, 6, 7, 2, 3, 4, 5);
//!
//! 	// Sort with custom key:
//! 	dmsort::sort_by_key(&mut numbers, |x| -x);
//! 	assert_eq!(numbers, vec!(7, 6, 5, 4, 3, 2, 1, 0));
//!
//! 	// Sort with Ord trait:
//! 	dmsort::sort(&mut numbers);
//! 	assert_eq!(numbers, vec!(0, 1, 2, 3, 4, 5, 6, 7));
//!
//! 	// Sort with custom compare:
//! 	dmsort::sort_by(&mut numbers, |a, b| b.cmp(a));
//! 	assert_eq!(numbers, vec!(7, 6, 5, 4, 3, 2, 1, 0));
//! }
//! ```

pub use dmsort::{sort, sort_by, sort_by_key};

/// For in module-level testing only. TODO: this shouldn't be public.
pub use dmsort::sort_copy;

mod dmsort;
