// By Emil Ernerfeldt December 2016
// LICENSE:
//   This software is dual-licensed to the public domain and under the following
//   license: you are granted a perpetual, irrevocable license to copy, modify,
//   publish, and distribute this file as you see fit.

use std::cmp::Ordering;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::ptr;

extern crate rand;
use rand::{Rng, SeedableRng, StdRng};

extern crate pbr;
use pbr::ProgressBar;

extern crate quickersort;
extern crate time;

// ----------------------------------------------------------------------------

/// This speeds up well-ordered input by quite a lot.
const DOUBLE_COMPARISONS : bool = true;

/// Low RECENCY = faster when there is low disorder (a lot of order).
/// High RECENCY = more resilient against long stretches of noise.
/// If RECENCY is too small we are more dependent on nice data/luck.
const RECENCY : usize = 8;

/// Back-track several elements at once. This is helpful when there are big clumps out-of-order.
const FAST_BACKTRACKING : bool = true;

/// Break early if we notice that the input is not ordered enough.
const EARLY_OUT : bool = true;

/// Test for early-out when we have processed len / EARLY_OUT_TEST_AT elements.
const EARLY_OUT_TEST_AT : usize = 4;

/// If more than this percentage of elements have been dropped, we abort.
const EARLY_OUT_DISORDER_FRACTION : f32 = 0.80;

/// Show fastest of BENCH_BEST_OF:
const BENCH_BEST_OF : usize = 5;

static BENCH_RESOLUTION_START  : usize = 20;
static BENCH_RESOLUTION_END    : usize =  99 * 5;
static BENCH_RESOLUTION_CUTOFF : f32   =   0.01;

// ----------------------------------------------------------------------------

/// This is the readable reference implementation that only works for Copy types.
fn dmsort_copy_by<T, F>(slice: &mut [T], mut compare: F) -> usize
	where T: Copy,
		  F: FnMut(&T, &T) -> Ordering
{
	if slice.len() < 2 { return slice.len(); }

	// ------------------------------------------------------------------------
	// First step: heuristically find the Longest Nondecreasing Subsequence (LNS).
	// The LNS is shifted into slice[..write] while slice[write..] will be left unchanged.
	// Elements not part of the LNS will be put in the "dropped" vector.

	let mut dropped = Vec::new();
	let mut num_dropped_in_row = 0;
	let mut write = 0; // Index of where to write the next element to keep.
	let mut read  = 0; // Index of the input stream.

	while read < slice.len() {
		if EARLY_OUT
			&& read == slice.len() / EARLY_OUT_TEST_AT
			&& dropped.len() as f32 > read as f32 * EARLY_OUT_DISORDER_FRACTION {
			// We have seen a lot of the elements and dropped a lot of them.
			// This doesn't look good. Abort.
			for i in 0..dropped.len() {
				slice[write + i] = dropped[i];
			}
			slice.sort_by(|a, b| compare(a, b));
			return dropped.len() * EARLY_OUT_TEST_AT; // Just an estimate.
		}

		if 1 <= write && compare(&slice[read], &slice[write - 1]) == Ordering::Less {
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

				// Undo the dropping of elements:
				let trunc_to_length = dropped.len() - num_dropped_in_row;
				dropped.truncate(trunc_to_length);
				read -= num_dropped_in_row;

				let mut num_backtracked = 1;
				write -= 1;

				if FAST_BACKTRACKING { // && 1 <= write && compare(&slice[read], &slice[write - 1]) == Ordering::Less {
					// Back-track until we can accept at least one of the recently dropped elements:
					let max_of_dropped = slice[read..(read + num_dropped_in_row + 1)].iter()
						.max_by(|a, b| return compare(a, b)).unwrap();

					while 1 <= write && compare(&max_of_dropped, &slice[write - 1]) == Ordering::Less {
						num_backtracked += 1;
						write -= 1;
					}
				}

				// Drop the back-tracked elements:
				dropped.extend_from_slice(&slice[write..(write + num_backtracked)]);

				num_dropped_in_row = 0;
			}
		} else {
			// Keep:
			slice[write] = slice[read];
			read += 1;
			write += 1;
			num_dropped_in_row = 0;
		}
	}

	let num_dropped = dropped.len();

	// ------------------------------------------------------------------------

	dropped.sort_by(|a, b| return compare(a, b));

	// ------------------------------------------------------------------------
	// slice[..write] is now sorted, as is "dropped".
	// We now want to merge these into "slice".
	// Let us do that from the back, putting the largest elements in place first:

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

fn dmsort_copy<T: Copy + Ord>(slice: &mut [T]) -> usize
{
	dmsort_copy_by(slice, |a, b| a.cmp(b))
}

// ----------------------------------------------------------------------------

/*
A note about protecting us from stack unwinding:

If our compare function panics we need to make sure all objects are put back into slice
so they can be properly destroyed by the caller.

This is done by temporarily bit-copying the data into the dropped vector
and copying them back if there is a panic.
*/
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

unsafe fn unsafe_push<T>(vec : &mut Vec<T>, value: &T) {
	let old_len = vec.len();
	vec.push(std::mem::uninitialized::<T>());
	ptr::copy_nonoverlapping(value, vec.get_unchecked_mut(old_len), 1);
}

unsafe fn unsafe_copy<T>(slice: &mut [T], source: usize, dest: usize) {
	ptr::copy_nonoverlapping(slice.get_unchecked(source), slice.get_unchecked_mut(dest), 1);
}

fn dmsort_move_by<T, F>(slice: &mut [T], mut compare: F)
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

	while read < s.slice.len() {
		if EARLY_OUT
			&& read == s.slice.len() / EARLY_OUT_TEST_AT
			&& s.dropped.len() as f32 > read as f32 * EARLY_OUT_DISORDER_FRACTION {
			// We have seen a lot of the elements and dropped a lot of them.
			// This doesn't look good. Abort.
			ptr::copy_nonoverlapping(s.dropped.as_ptr(), &mut s.slice[s.write], s.dropped.len());
			s.dropped.set_len(0);
			s.slice.sort_by(|a, b| compare(a, b));
			return;
		}

		if 1 <= s.write
			&& compare(s.slice.get_unchecked(read), s.slice.get_unchecked(s.write - 1)) == Ordering::Less
		{
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
				unsafe_push(&mut s.dropped, s.slice.get_unchecked(read));
				read += 1;
				num_dropped_in_row += 1;
			} else {
				// Undo the dropping of elements:
				let trunc_to_length = s.dropped.len() - num_dropped_in_row;
				s.dropped.set_len(trunc_to_length);
				read -= num_dropped_in_row;

				let mut num_backtracked = 1;
				s.write -= 1;

				if FAST_BACKTRACKING {
					// Back-track until we can accept at least one of the recently dropped elements:
					let max_of_dropped = s.slice[read..(read + num_dropped_in_row + 1)].iter()
						.max_by(|a, b| return compare(a, b)).unwrap();

					while 1 <= s.write && compare(&max_of_dropped, s.slice.get_unchecked(s.write - 1)) == Ordering::Less {
						num_backtracked += 1;
						s.write -= 1;
					}
				}

				// Append s.slice[read..(read + num_backtracked)] to s.dropped
				{
					let old_len = s.dropped.len();
					for _ in 0..num_backtracked {
						s.dropped.push(std::mem::uninitialized::<T>());
					}
					ptr::copy_nonoverlapping(s.slice.get_unchecked(s.write), s.dropped.get_unchecked_mut(old_len), num_backtracked);
				}

				num_dropped_in_row = 0;
			}
		} else {
			unsafe_copy(&mut s.slice, read, s.write);
			read += 1;
			s.write += 1;
			num_dropped_in_row = 0;
		}
	}

	// ------------------------------------------------------------------------

	s.dropped.sort_by(|a, b| compare(a, b));

	// ------------------------------------------------------------------------
	// s.slice[..s.write] is now sorted, as is "s.dropped".
	// We now want to merge these into "s.slice".
	// Let us do that from the back, putting the largest elements in place first:

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

fn dmsort_move<T: Ord>(slice: &mut [T])
{
	dmsort_move_by(slice, |a, b| a.cmp(b))
}

// ----------------------------------------------------------------------------

/// Sorts the elements using the given compare function.
/// Expected number of comparisons is O(N + K * log(K)) where K is the number of elements not in order.
/// Expected memory usage is O(K).
/// Works best for when K < 0.2 * N.
/// The out-of-order elements are expected to be randomly distributed (NOT clumped).
fn dmsort_by<T, F>(slice: &mut [T], compare: F)
	where T: Copy,
		  F: FnMut(&T, &T) -> Ordering
{
	dmsort_move_by(slice, compare);
}

/// Sorts the elements using the given compare function.
/// Expected number of comparisons is O(N + K * log(K)) where K is the number of elements not in order.
/// Expected memory usage is O(K).
/// Works best for when K < 0.2 * N.
/// The out-of-order elements are expected to be randomly distributed (NOT clumped).
fn dmsort<T: Ord>(slice: &mut [T])
{
	dmsort_move_by(slice, |a, b| a.cmp(b));
}

// ----------------------------------------------------------------------------

type Integer = i32;

/// Returns a mostly-sorted array with disorder_factor fraction of elements with random values.
fn generate_integers(rng: &mut rand::StdRng, length: usize, disorder_factor: f32) -> Vec<Integer> {
	let mut result = Vec::with_capacity(length);
	for i in 0..length {
		if rng.next_f32() < disorder_factor {
			result.push(rng.gen_range(0 as Integer, length as Integer));
		} else {
			result.push(i as Integer);
		}
	}
	result
}

fn generate_strings(rng: &mut rand::StdRng, length: usize, disorder_factor: f32) -> Vec<String> {
	generate_integers(rng, length, disorder_factor).iter().map(|&x| format!("{:0100}", x)).collect()
}

fn time_sort_ms<T: Clone, Sorter>(unsorted: &Vec<T>, mut sorter: Sorter) -> (f32, Vec<T>)
	where Sorter: FnMut(&mut Vec<T>)
{
	let mut best_ns = None;
	let mut sorted = Vec::new();

	for _ in 0..BENCH_BEST_OF {
		let mut vec_clone : Vec<T> = unsorted.clone();
		let start_time_ns = time::precise_time_ns();
		sorter(&mut vec_clone);
		let duration_ns = time::precise_time_ns() - start_time_ns;
		if best_ns == None || duration_ns < best_ns.unwrap() {
			best_ns = Some(duration_ns);
		}
		sorted = vec_clone;
	}

	(best_ns.unwrap() as f32 / 1000000.0, sorted)
}

/// Benchmark at these disorders:
fn get_bench_disorders() -> Vec<f32> {
	fn remap(x: usize, in_min: usize, in_max: usize, out_min: f32, out_max: f32) -> f32 {
		out_min + (out_max - out_min) * ((x - in_min) as f32) / ((in_max - in_min) as f32)
	}

	return
		(0..BENCH_RESOLUTION_START).map(    |x| remap(x, 0, BENCH_RESOLUTION_START, 0.0, BENCH_RESOLUTION_CUTOFF)).chain(
		(0..(BENCH_RESOLUTION_END + 1)).map(|x| remap(x, 0, BENCH_RESOLUTION_END,   BENCH_RESOLUTION_CUTOFF, 1.0))
		).collect();
}

fn generate_comparison_data_i32(rng: &mut rand::StdRng, length: usize) {
	let bench_disorders = get_bench_disorders();
	let mut pb = ProgressBar::new(bench_disorders.len() as u64);
	pb.message("Benchmarking integers: ");
	let mut std_file             = File::create(&Path::new("data/i32/std_sort.data")).unwrap();
	let mut quicker_file         = File::create(&Path::new("data/i32/quicker_sort.data")).unwrap();
	let mut dmsort_copy_file     = File::create(&Path::new("data/i32/dmsort_copy_sort.data")).unwrap();
	let mut dmsort_move_file     = File::create(&Path::new("data/i32/dmsort_move_sort.data")).unwrap();
	let mut copy_speedup_file    = File::create(&Path::new("data/i32/dmsort_copy_speedup.data")).unwrap();
	let mut move_speedup_file    = File::create(&Path::new("data/i32/dmsort_move_speedup.data")).unwrap();
	let mut num_dropped_file     = File::create(&Path::new("data/i32/num_dropped.data")).unwrap();

	for disorder_factor in bench_disorders {
		let vec = generate_integers(rng, length, disorder_factor);
		let (std_duration_ms,            std_sorted)            = time_sort_ms(&vec, |x| x.sort());
		let (quicker_duration_ms,        quicker_sorted)        = time_sort_ms(&vec, |x| quickersort::sort(x));
		let (dmsort_copy_duration_ms,    dmsort_copy_sorted)    = time_sort_ms(&vec, |x| {dmsort_copy(x);    ()});
		let (dmsort_move_duration_ms,    dmsort_move_sorted)    = time_sort_ms(&vec, |x| dmsort_move(x));

		let fastest_competitor_ms = std_duration_ms.min(quicker_duration_ms);

		assert_eq!(dmsort_copy_sorted,    std_sorted);
		assert_eq!(dmsort_move_sorted,    std_sorted);
		assert_eq!(quicker_sorted,        std_sorted);

		write!(std_file,             "{} {}\n", disorder_factor * 100.0, std_duration_ms).unwrap();
		write!(quicker_file,         "{} {}\n", disorder_factor * 100.0, quicker_duration_ms).unwrap();
		write!(dmsort_copy_file,     "{} {}\n", disorder_factor * 100.0, dmsort_copy_duration_ms).unwrap();
		write!(dmsort_move_file,     "{} {}\n", disorder_factor * 100.0, dmsort_move_duration_ms).unwrap();
		write!(copy_speedup_file,    "{} {}\n", disorder_factor * 100.0, fastest_competitor_ms / dmsort_copy_duration_ms).unwrap();
		write!(move_speedup_file,    "{} {}\n", disorder_factor * 100.0, fastest_competitor_ms / dmsort_move_duration_ms).unwrap();

		{
			let mut temp = vec.clone();
			let num_dropped_in_row = dmsort_copy(&mut temp);
			let num_dropped = (num_dropped_in_row as f64) / (temp.len() as f64);
			write!(num_dropped_file, "{} {}\n", disorder_factor * 100.0, num_dropped * 100.0).unwrap();
		}

		pb.inc();
	}
}

fn generate_comparison_data_string(rng: &mut rand::StdRng, length: usize) {
	let bench_disorders = get_bench_disorders();
	let mut pb = ProgressBar::new(bench_disorders.len() as u64);
	pb.message("Benchmarking strings: ");
	let mut std_file        = File::create(&Path::new("data/string/std_sort.data")).unwrap();
	let mut quicker_file    = File::create(&Path::new("data/string/quicker_sort.data")).unwrap();
	let mut drop_merge_file = File::create(&Path::new("data/string/drop_merge_sort.data")).unwrap();
	let mut speedup_file    = File::create(&Path::new("data/string/speedup_over_quickersort.data")).unwrap();

	for disorder_factor in bench_disorders {
		let vec = generate_strings(rng, length, disorder_factor);
		let (std_duration_ms,        std_sorted)        = time_sort_ms(&vec, |x| x.sort());
		let (quicker_duration_ms,    quicker_sorted)    = time_sort_ms(&vec, |x| quickersort::sort(x));
		let (drop_merge_duration_ms, drop_merge_sorted) = time_sort_ms(&vec, |x| dmsort(x));

		let fastest_competitor_ms = std_duration_ms.min(quicker_duration_ms);

		assert_eq!(std_sorted, drop_merge_sorted);
		assert_eq!(quicker_sorted, drop_merge_sorted);

		write!(std_file,        "{} {}\n", disorder_factor * 100.0, std_duration_ms).unwrap();
		write!(quicker_file,    "{} {}\n", disorder_factor * 100.0, quicker_duration_ms).unwrap();
		write!(drop_merge_file, "{} {}\n", disorder_factor * 100.0, drop_merge_duration_ms).unwrap();
		write!(speedup_file,    "{} {}\n", disorder_factor * 100.0, fastest_competitor_ms / drop_merge_duration_ms).unwrap();

		pb.inc();
	}
}

#[test]
fn run_tests() {
	fn test_type<T: Clone + PartialEq + Ord + std::fmt::Debug>(unsorted: Vec<T>) {
		let mut dm_sorted = unsorted.clone();
		dmsort(&mut dm_sorted);

		let mut std_sorted = unsorted.clone();
		std_sorted.sort();

		if dm_sorted != std_sorted {
			panic!("FAIL with input {:?}", unsorted);
		}
	}

	fn test(list: Vec<Integer>) {
		test_type(list.iter().map(|&x| format!("{:02}", x)).collect());
		test_type(list);
	}

	test(vec!());
	test(vec!(0));
	test(vec!(0, 1));
	test(vec!(1, 0));
	test(vec!(0, 1, 2));
	test(vec!(0, 2, 1));
	test(vec!(1, 0, 2));
	test(vec!(1, 2, 0));
	test(vec!(2, 0, 1));
	test(vec!(2, 1, 0));
	test(vec!(0, 1, 3, 2, 4, -5, 6, 7, 8, 9));
	test(vec!(0, 1, 10, 3, 4, 5, 6, 7, 8, 9));
	test(vec!(10, 1, 2, 3, 4, 5, 6, 7, 8, 9));
	test(vec!(0, 0, 2, 3, 4, 1, 6, 1, 8, 9));
	test(vec!(20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19));
	test(vec!(20, 21, 2, 23, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19));
}

/// Benchmark worst-case input for Drop-Merge sort
fn bench_evil() {
	let evil_input : Vec<_> = (100..1000000).chain(0..100).collect();
	let (std_duration_ms,     std_sorted)     = time_sort_ms(&evil_input, |x| x.sort());
	let (quicker_duration_ms, quicker_sorted) = time_sort_ms(&evil_input, |x| quickersort::sort(x));
	let (drop_duration_ms,    drop_sorted)    = time_sort_ms(&evil_input, |x| dmsort(x));
	// let (drop_duration_ms,    drop_sorted)    = time_sort_ms(&evil_input, |x| {dmsort_copy(x); ()});

	assert_eq!(std_sorted, drop_sorted);
	assert_eq!(std_sorted, quicker_sorted);
	println!("Worst-case input:");
	println!("std::sort:       {} ms", std_duration_ms);
	println!("Quicksort:       {} ms", quicker_duration_ms);
	println!("Drop-Merge sort: {} ms", drop_duration_ms);
}

fn main() {
	bench_evil();

	let seed: &[_] = &[0];
	let mut rng: StdRng = SeedableRng::from_seed(seed);

	generate_comparison_data_i32(&mut rng, 1000000);
	generate_comparison_data_string(&mut rng, 100000);
}
