// By Emil Ernerfeldt December 2016
// LICENSE:
//   This software is dual-licensed to the public domain and under the following
//   license: you are granted a perpetual, irrevocable license to copy, modify,
//   publish, and distribute this file as you see fit.

use std::cmp::Ordering;
use std::fs::File;
use std::io::Write;
use std::path::Path;

extern crate rand;
use rand::{Rng, SeedableRng, StdRng};

extern crate quickersort;
extern crate time;

// ----------------------------------------------------------------------------

/// Sorts the elements using the given compare function.
/// Expected number of comparisons is O(N + K * log(K)) where K is the number of elements not in order.
/// Expected memory usage is O(K).
/// Works best for when K < 0.2 * N.
/// The out-of-order elements are expected to be randomly distributed (NOT clumped).
fn drop_merge_sort_by<T, F>(list: &mut [T], mut compare: F)
	where T: Copy,
		  F: FnMut(&T, &T) -> Ordering
{
	if list.len() < 2 { return; }

	// ------------------------------------------------------------------------
	// First step: heuristically find the longest non-decreasing subsequence (LNDS).
	// This is done using the methods described in
	// "Item Retention Improvements to Dropsort, a Lossy Sorting Algorithm"
	// by Abram Jackson and Ryan McCulloch
	// (http://micsymposium.org/mics_2011_proceedings/mics2011_submission_13.pdf)
	// The LNDS is shifted into list[..write] while list[write..] will be left unchanged.
	// Elements not part of the LNDS will be put in the "dropped" vector.

	let mut dropped = Vec::new();
	let mut num_dropped_in_row = 0;
	let mut write = 0;
	let mut read  = 0;
	const RECENCY : usize = 8; // Higher = more resilient against long stretches of noise.

	while read < list.len() {
		if 0 < write && compare(&list[read], &list[write - 1]) == Ordering::Less {
			// Out of order. "Drop" the element at read:
			if num_dropped_in_row < RECENCY {
				dropped.push(list[read]);
				read += 1;
				num_dropped_in_row += 1;
			} else {
				/*
				We accepted something num_dropped_in_row elements back that made us drop all RECENCY subsequent items.
				Accepting that element was obviously a mistake - so let's undo it!

				Example problem (RECENCY = 3):
					0 1 10 3 4 5 6   // 10 was accepted. When we get to 5 we reach num_dropped_in-row == RECENCY

				Example worst-case (RECENCY = 3):
					...100 101 102 103 104 1 2 3 4 5 ....
						100-104 was accepted. When we get to 3 we reach num_dropped_in-row == RECENCY
						We drop 104 and reset the read by RECENCY. We restart, and then we drop again.
						This can lead us to backtracking RECENCY number of elements
						as many times as the leading non-decreasing subsequence is long.
				*/

				// Back up and recheck the elements we previously dropped:
				let trunc_to_length = dropped.len() - num_dropped_in_row;
				dropped.truncate(trunc_to_length);
				read -= num_dropped_in_row;

				// Drop the element we mistakingly accepted:
				dropped.push(list[write - 1]);
				write -= 1; // Over-write the dropped element.

				num_dropped_in_row = 0;
			}
		} else {
			// Keep:
			list[write] = list[read];
			read += 1;
			write += 1;
			num_dropped_in_row = 0;
		}
	}

	// ------------------------------------------------------------------------

	dropped.sort_by(|a, b| return compare(a, b));

	// ------------------------------------------------------------------------
	// list[..write] is now sorted, as is "dropped".
	// We now want to merge these into "list".
	// Let us do that from the back, putting the largest elements in place first:

	let mut back = list.len();

	while 0 < back {
		if let Some(&last_dropped) = dropped.last() {
			if 0 < write && compare(&last_dropped, &list[write - 1]) == Ordering::Less {
				list[back - 1] = list[write - 1];
				write -= 1;
			} else {
				list[back - 1] = last_dropped;
				dropped.pop();
			}
		} else {
			// Nothing left in "dropped" - we are done!
			assert!(back == write);
			break;
		}

		back -= 1;
	}
}

fn drop_merge_sort<T>(list: &mut [T]) where T: Copy + Ord
{
	drop_merge_sort_by(list, |a, b| a.cmp(b))
}

// ----------------------------------------------------------------------------

type Element = i32;

/// Returns a mostly-sorted array with randomization_factor fraction of elements with random values.
fn generate_test_data(rng: &mut rand::StdRng, length: usize, randomization_factor: f32) -> Vec<Element> {
	let mut result = Vec::with_capacity(length);
	for i in 0..length {
		if rng.next_f32() < randomization_factor {
			result.push(rng.gen_range(0 as Element, length as Element));
		} else {
			result.push(i as Element);
		}
	}
	result
}

fn time_sort_ms<Sorter>(unsorted: &Vec<Element>, mut sorter: Sorter) -> (f32, Vec<Element>)
	where Sorter: FnMut(&mut Vec<Element>)
{
	let mut best_ns = None;
	let mut sorted = Vec::new();

	// Return fastest of five:
	for _ in 0..5 {
		let mut vec_clone = unsorted.clone();
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

fn generate_comparison_data(rng: &mut rand::StdRng, length: usize) {
	let mut std_file     = File::create(&Path::new("data/std_sort.data")).unwrap();
	let mut quicker_file = File::create(&Path::new("data/quicker_sort.data")).unwrap();
	let mut drop_file    = File::create(&Path::new("data/drop_merge_sort.data")).unwrap();

	for i in 0i32..1001i32 {
		let randomization_factor = (i as f32) / 1000.0;
		let vec = generate_test_data(rng, length, randomization_factor);
		let (std_duration_ms,     std_sorted)     = time_sort_ms(&vec, |x| x.sort());
		let (quicker_duration_ms, quicker_sorted) = time_sort_ms(&vec, |x| quickersort::sort(x));
		let (drop_duration_ms,    drop_sorted)    = time_sort_ms(&vec, |x| drop_merge_sort(x));

		assert_eq!(std_sorted, drop_sorted);
		assert_eq!(quicker_sorted, drop_sorted);

		write!(std_file,     "{} {}\n", randomization_factor * 100.0, std_duration_ms).unwrap();
		write!(quicker_file, "{} {}\n", randomization_factor * 100.0, quicker_duration_ms).unwrap();
		write!(drop_file,    "{} {}\n", randomization_factor * 100.0, drop_duration_ms).unwrap();
	}
}

fn generate_speedup_data(rng: &mut rand::StdRng, length: usize) {
	let mut file = File::create(&Path::new("data/speedup_over_quickersort.data")).unwrap();

	for i in 0i32..251i32 {
		let randomization_factor = (i as f32) / 1000.0;
		let vec = generate_test_data(rng, length, randomization_factor);
		let (quicker_duration_ms, quicker_sorted) = time_sort_ms(&vec, |x| quickersort::sort(x));
		let (drop_duration_ms,    drop_sorted)    = time_sort_ms(&vec, |x| drop_merge_sort(x));

		assert_eq!(quicker_sorted, drop_sorted);

		write!(file, "{} {}\n", randomization_factor * 100.0, quicker_duration_ms / drop_duration_ms).unwrap();
	}
}

#[test]
fn run_tests() {
	fn test(mut list: Vec<Element>) {
		let mut std_sorted = list.clone();
		std_sorted.sort();

		println!();
		println!("IN:  {:?}", list);
		drop_merge_sort(&mut list);
		println!("OUT: {:?}", list);

		if list != std_sorted {
			panic!("FAIL with input {:?}", list);
		}
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
	let (std_duration_ms, std_sorted)         = time_sort_ms(&evil_input, |x| x.sort());
	let (quicker_duration_ms, quicker_sorted) = time_sort_ms(&evil_input, |x| quickersort::sort(x));
	let (drop_duration_ms,    drop_sorted)    = time_sort_ms(&evil_input, |x| drop_merge_sort(x));

	assert_eq!(std_sorted, drop_sorted);
	assert_eq!(quicker_sorted, drop_sorted);
	println!("Worst-case input:");
	println!("std::sort:       {} ms", std_duration_ms);
	println!("Quicksort:       {} ms", quicker_duration_ms);
	println!("Drop-Merge sort: {} ms", drop_duration_ms);
}

fn main() {
	bench_evil();
	let seed: &[_] = &[0];
	let mut rng: StdRng = SeedableRng::from_seed(seed);
	generate_comparison_data(&mut rng, 1000000); // ~15 min runtime
	generate_speedup_data(&mut rng, 1000000); // ~1 min runtime
}
