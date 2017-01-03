extern crate dmsort;

extern crate pbr;
extern crate quickersort;
extern crate rand;
extern crate time;

use pbr::ProgressBar;
use rand::{Rng, SeedableRng, StdRng};

use std::fs::File;
use std::io::Write;
use std::path::Path;

/// Show fastest of BENCH_BEST_OF:
const BENCH_BEST_OF : usize = 5;

static BENCH_RESOLUTION_START  : usize = 20;
static BENCH_RESOLUTION_END    : usize =  99 * 5;
static BENCH_RESOLUTION_CUTOFF : f32   =   0.01;

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
		let (dmsort_copy_duration_ms,    dmsort_copy_sorted)    = time_sort_ms(&vec, |x| {dmsort::sort_copy(x); ()});
		let (dmsort_move_duration_ms,    dmsort_move_sorted)    = time_sort_ms(&vec, |x| dmsort::sort(x));

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
			let num_dropped_in_row = dmsort::sort_copy(&mut temp);
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
		let (drop_merge_duration_ms, drop_merge_sorted) = time_sort_ms(&vec, |x| dmsort::sort(x));

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


/// Benchmark worst-case input for Drop-Merge sort
fn bench_evil() {
	let evil_input : Vec<_> = (100..1000000).chain(0..100).collect();
	let (std_duration_ms,     std_sorted)     = time_sort_ms(&evil_input, |x| x.sort());
	let (quicker_duration_ms, quicker_sorted) = time_sort_ms(&evil_input, |x| quickersort::sort(x));
	let (drop_duration_ms,    drop_sorted)    = time_sort_ms(&evil_input, |x| dmsort::sort(x));
	// let (drop_duration_ms,    drop_sorted)    = time_sort_ms(&evil_input, |x| {dmsort::sort_copy(x); ()});

	assert_eq!(std_sorted, drop_sorted);
	assert_eq!(std_sorted, quicker_sorted);
	println!("Worst-case input:");
	println!("std::sort:       {} ms", std_duration_ms);
	println!("Quicksort:       {} ms", quicker_duration_ms);
	println!("Drop-Merge sort: {} ms", drop_duration_ms);
}

#[test]
#[ignore]
fn benchmarks() {
	bench_evil();

	let seed: &[_] = &[0];
	let mut rng: StdRng = SeedableRng::from_seed(seed);
	generate_comparison_data_i32(&mut rng, 1000000);
	println!();
	generate_comparison_data_string(&mut rng, 100000);
}
