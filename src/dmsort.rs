// Copyright (c) 2017 Emil Ernerfeldt

use std::cmp::Ordering;
use std::mem;
use std::ptr;

// ----------------------------------------------------------------------------

/// This speeds up well-ordered input by quite a lot.
const DOUBLE_COMPARISONS: bool = true;

/// Low RECENCY = faster when there is low disorder (a lot of order).
/// High RECENCY = more resilient against long stretches of noise.
/// If RECENCY is too small we are more dependent on nice data/luck.
const RECENCY: usize = 8;

/// Back-track several elements at once. This is helpful when there are big clumps out-of-order.
const FAST_BACKTRACKING: bool = true;

/// Break early if we notice that the input is not ordered enough.
const EARLY_OUT: bool = true;

/// Test for early-out when we have processed len / `EARLY_OUT_TEST_AT` elements.
const EARLY_OUT_TEST_AT: usize = 4;

/// If more than this percentage of elements have been dropped, we abort.
const EARLY_OUT_DISORDER_FRACTION: f32 = 0.60;

// ----------------------------------------------------------------------------

/// This is the readable reference implementation that only works for Copy types.
/// Returns the number of dropped elements for diagnostic purposes.
fn sort_copy_by<T, F>(slice: &mut [T], mut compare: F) -> usize
	where T: Copy,
	      F: FnMut(&T, &T) -> Ordering
{
	if slice.len() < 2 {
		return slice.len();
	}

	// ------------------------------------------------------------------------
	// First step: heuristically find the Longest Nondecreasing Subsequence (LNS).
	// The LNS is shifted into slice[..write] while slice[write..] will be left unchanged.
	// Elements not part of the LNS will be put in the "dropped" vector.

	let mut dropped = Vec::new();
	let mut num_dropped_in_row = 0;
	let mut write = 0; // Index of where to write the next element to keep.
	let mut read  = 0; // Index of the input stream.
	let mut iteration = 0;
	let ealy_out_stop = slice.len() / EARLY_OUT_TEST_AT;

	while read < slice.len() {
		iteration += 1;
		if EARLY_OUT
			&& iteration == ealy_out_stop
			&& dropped.len() as f32 > read as f32 * EARLY_OUT_DISORDER_FRACTION {
			// We have seen a lot of the elements and dropped a lot of them.
			// This doesn't look good. Abort.
			for (i, &element) in dropped.iter().enumerate() {
				slice[write + i] = element;
			}
			slice.sort_unstable_by(|a, b| compare(a, b));
			return dropped.len() * EARLY_OUT_TEST_AT; // Just an estimate.
		}

		if write == 0 || compare(&slice[read], &slice[write - 1]) != Ordering::Less {
			// The element is order - keep it:
			slice[write] = slice[read];
			read += 1;
			write += 1;
			num_dropped_in_row = 0;
		} else {
			// The next element is smaller than the last stored one.
			// The question is - should we drop the new element, or was accepting the previous element a mistake?

			/*
			   Check this situation:
			   0 1 2 3 9 5 6 7  (the 9 is a one-off)
			           | |
			           | read
			           write - 1
				Checking this improves performance because we catch common problems earlier (without back-tracking).
			*/
			if DOUBLE_COMPARISONS
				&& num_dropped_in_row == 0
				&& 2 <= write
				&& compare(&slice[read], &slice[write - 2]) != Ordering::Less
			{
				// Quick undo: drop previously accepted element, and overwrite with new one:
				dropped.push(slice[write - 1]);
				slice[write - 1] = slice[read];
				read += 1;
				continue;
			}

			if num_dropped_in_row < RECENCY {
				// Drop it:
				dropped.push(slice[read]);
				read += 1;
				num_dropped_in_row += 1;
			} else {
				/*
				We accepted something num_dropped_in_row elements back that made us drop all RECENCY subsequent items.
				Accepting that element was obviously a mistake - so let's undo it!

				Example problem (RECENCY = 3):    0 1 12 3 4 5 6
					0 1 12 is accepted. 3, 4, 5 will be rejected because they are larger than the last kept item (12).
					When we get to 5 we reach num_dropped_in_row == RECENCY.
					This will trigger an undo where we drop the 12.
					When we again go to 3, we will keep it because it is larger than the last kept item (1).

				Example worst-case (RECENCY = 3):   ...100 101 102 103 104 1 2 3 4 5 ....
					100-104 is accepted. When we get to 3 we reach num_dropped_in_row == RECENCY.
					We drop 104 and reset the read by RECENCY. We restart, and then we drop again.
					This can lead us to backtracking RECENCY number of elements
					as many times as the leading non-decreasing subsequence is long.
				*/

				// Undo dropping the last num_dropped_in_row elements:
				let trunc_to_length = dropped.len() - num_dropped_in_row;
				dropped.truncate(trunc_to_length);
				read -= num_dropped_in_row;

				let mut num_backtracked = 1;
				write -= 1;

				if FAST_BACKTRACKING {
					// Back-track until we can accept at least one of the recently dropped elements:
					let max_of_dropped = slice[read..(read + num_dropped_in_row + 1)]
						.iter().max_by(|a, b| compare(a, b)).unwrap();

					while 1 <= write && compare(max_of_dropped, &slice[write - 1]) == Ordering::Less {
						num_backtracked += 1;
						write -= 1;
					}
				}

				// Drop the back-tracked elements:
				dropped.extend_from_slice(&slice[write..(write + num_backtracked)]);

				num_dropped_in_row = 0;
			}
		}
	}

	let num_dropped = dropped.len();

	// ------------------------------------------------------------------------
	// Second step: sort the dropped elements:

	dropped.sort_unstable_by(|a, b| compare(a, b));

	// ------------------------------------------------------------------------
	// Third step: merge slice[..write] and `dropped`:

	let mut back = slice.len();

	while let Some(&last_dropped) = dropped.last() {
		while 0 < write && compare(&last_dropped, &slice[write - 1]) == Ordering::Less {
			slice[back - 1] = slice[write - 1];
			back -= 1;
			write -= 1;
		}
		slice[back - 1] = last_dropped;
		back -= 1;
		dropped.pop();
	}

	num_dropped
}

/// UNSTABLE! FOR INTERNAL USE ONLY.
pub fn sort_copy<T: Copy + Ord>(slice: &mut [T]) -> usize {
	sort_copy_by(slice, |a, b| a.cmp(b))
}

// ----------------------------------------------------------------------------

// A note about protecting us from stack unwinding:
//
// If our compare function panics we need to make sure all objects are put back into slice
// so they can be properly destroyed by the caller.
//
// This is done by temporarily bit-copying the data into the dropped vector
// and copying them back if there is a panic.
//
struct DmSorter<'a, T: 'a> {
	/// The slice we are sorting
	slice: &'a mut [T],

	/// Temporary storage of dropped elements.
	dropped: Vec<T>,

	/// Index in self.slice of where to write the next element to keep.
	write: usize,

	// slice[write..(write + dropped.len())] is a gap. The elements can be found in dropped
}

impl<'a, T> Drop for DmSorter<'a, T> {
	fn drop(&mut self) {
		if self.dropped.is_empty() { return; }
		unsafe {
			// This code will only run on stack-unwind (panic).

			// Move back all elements into the slice:
			ptr::copy_nonoverlapping(self.dropped.as_ptr(), &mut self.slice[self.write], self.dropped.len());

			// Make sure the objects aren't destroyed when self.dropped is dropped (avoid-double-free).
			self.dropped.set_len(0);
		}
	}
}

#[inline(always)]
unsafe fn unsafe_push<T>(vec: &mut Vec<T>, value: &T) {
	let old_len = vec.len();
	vec.push(mem::uninitialized::<T>());
	ptr::copy_nonoverlapping(value, vec.get_unchecked_mut(old_len), 1);
}

#[inline(always)]
unsafe fn unsafe_copy<T>(slice: &mut [T], source: usize, dest: usize) {
	ptr::copy_nonoverlapping(slice.get_unchecked(source), slice.get_unchecked_mut(dest), 1);
}

fn sort_move_by<T, F>(slice: &mut [T], mut compare: F)
	where F: FnMut(&T, &T) -> Ordering
{ unsafe {
	if slice.len() < 2 { return; }

	let mut s = DmSorter{
		slice:   slice,
		dropped: Vec::new(),
		write:   0,
	};

	// ------------------------------------------------------------------------

	let mut num_dropped_in_row = 0;
	let mut read = 0;
	let mut iteration = 0;
	let ealy_out_stop = s.slice.len() / EARLY_OUT_TEST_AT;

	while read < s.slice.len() {
		iteration += 1;
		if EARLY_OUT
			&& iteration == ealy_out_stop
			&& s.dropped.len() as f32 > read as f32 * EARLY_OUT_DISORDER_FRACTION {
			// We have seen a lot of the elements and dropped a lot of them.
			// This doesn't look good. Abort.
			ptr::copy_nonoverlapping(s.dropped.as_ptr(), &mut s.slice[s.write], s.dropped.len());
			s.dropped.set_len(0);
			s.slice.sort_unstable_by(|a, b| compare(a, b));
			return;
		}

		if s.write == 0 || compare(s.slice.get_unchecked(read), s.slice.get_unchecked(s.write - 1)) != Ordering::Less {
			// The element is order - keep it:
			unsafe_copy(&mut s.slice, read, s.write);
			read += 1;
			s.write += 1;
			num_dropped_in_row = 0;
		} else {
			if DOUBLE_COMPARISONS
				&& num_dropped_in_row == 0
				&& 2 <= s.write
				&& compare(s.slice.get_unchecked(read), s.slice.get_unchecked(s.write - 2)) != Ordering::Less
			{
				// Quick undo: drop previously accepted element, and overwrite with new one:
				unsafe_push(&mut s.dropped, s.slice.get_unchecked(s.write - 1));
				unsafe_copy(&mut s.slice, read, s.write - 1);
				read += 1;
				continue;
			}

			if num_dropped_in_row < RECENCY {
				// Drop it:
				unsafe_push(&mut s.dropped, s.slice.get_unchecked(read));
				read += 1;
				num_dropped_in_row += 1;
			} else {
				// Undo dropping the last num_dropped_in_row elements:
				let trunc_to_length = s.dropped.len() - num_dropped_in_row;
				s.dropped.set_len(trunc_to_length);
				read -= num_dropped_in_row;

				let mut num_backtracked = 1;
				s.write -= 1;

				if FAST_BACKTRACKING {
					// Back-track until we can accept at least one of the recently dropped elements:
					let max_of_dropped = s.slice[read..(read + num_dropped_in_row + 1)]
						.iter().max_by(|a, b| compare(a, b)).unwrap();

					while 1 <= s.write && compare(max_of_dropped, s.slice.get_unchecked(s.write - 1)) == Ordering::Less {
						num_backtracked += 1;
						s.write -= 1;
					}
				}

				// Append s.slice[read..(read + num_backtracked)] to s.dropped:
				{
					let old_len = s.dropped.len();
					for _ in 0..num_backtracked {
						s.dropped.push(mem::uninitialized::<T>());
					}
					ptr::copy_nonoverlapping(s.slice.get_unchecked(s.write), s.dropped.get_unchecked_mut(old_len), num_backtracked);
				}

				num_dropped_in_row = 0;
			}
		}
	}

	// ------------------------------------------------------------------------

	s.dropped.sort_unstable_by(|a, b| compare(a, b));

	// ------------------------------------------------------------------------
	// Merge:

	let mut back = s.slice.len();

	loop {
		let old_len = s.dropped.len();
		if old_len == 0 { break; }
		{
			let last_dropped = s.dropped.get_unchecked(old_len - 1);
			while 0 < s.write && compare(last_dropped, s.slice.get_unchecked(s.write - 1)) == Ordering::Less {
				unsafe_copy(&mut s.slice, s.write - 1, back - 1);
				back -= 1;
				s.write -= 1;
			}
			ptr::copy_nonoverlapping(last_dropped, s.slice.get_unchecked_mut(back - 1), 1);
		}
		back -= 1;
		s.dropped.set_len(old_len - 1);
	}
} }

// ----------------------------------------------------------------------------

/// Sorts the elements using the given compare function.
/// # Examples
/// ```
/// let mut numbers : Vec<i32> = vec!(0, 1, 6, 7, 2, 3, 4, 5);
/// dmsort::sort_by(&mut numbers, |a, b| b.cmp(a));
/// assert_eq!(numbers, vec!(7, 6, 5, 4, 3, 2, 1, 0));
/// ```
pub fn sort_by<T, F>(slice: &mut [T], compare: F)
	where F: FnMut(&T, &T) -> Ordering
{
	sort_move_by(slice, compare);
}

/// Sorts the elements using the given key function.
/// # Examples
/// ```
/// let mut numbers : Vec<i32> = vec!(0, 1, 6, 7, 2, 3, 4, 5);
/// dmsort::sort_by_key(&mut numbers, |x| -x);
/// assert_eq!(numbers, vec!(7, 6, 5, 4, 3, 2, 1, 0));
/// ```
pub fn sort_by_key<T, K, F>(slice: &mut [T], mut key: F)
	where K: Ord,
	      F: FnMut(&T) -> K
{
	sort_by(slice, |a, b| key(a).cmp(&key(b)));
}

/// Sorts the elements using the Ord trait.
/// # Examples
/// ```
/// let mut numbers : Vec<i32> = vec!(0, 1, 6, 7, 2, 3, 4, 5);
/// dmsort::sort(&mut numbers);
/// assert_eq!(numbers, vec!(0, 1, 2, 3, 4, 5, 6, 7));
/// ```
pub fn sort<T: Ord>(slice: &mut [T]) {
	sort_move_by(slice, |a, b| a.cmp(b));
}

// ----------------------------------------------------------------------------
