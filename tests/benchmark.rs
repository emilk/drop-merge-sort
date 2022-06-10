extern crate dmsort;

extern crate gnuplot;
extern crate pbr;
extern crate rand;
extern crate time;

use pbr::ProgressBar;
use rand::{rngs::StdRng, Rng};

static BENCH_RESOLUTION_START: usize = 10;
static BENCH_RESOLUTION_END: usize = 99 * 5;
static BENCH_RESOLUTION_CUTOFF: f32 = 0.01;

// ----------------------------------------------------------------------------

type Integer = i32;

/// Returns a mostly-sorted array with `disorder_factor` fraction of elements with random values.
fn generate_integers(rng: &mut StdRng, length: usize, disorder_factor: f32) -> Vec<Integer> {
	(0..length)
		.map(|i| {
			if rng.gen::<f32>() < disorder_factor {
				#[allow(clippy::unnecessary_cast)]
				rng.gen_range(0 as Integer, length as Integer)
			} else {
				i as Integer
			}
		})
		.collect()
}

fn generate_strings(rng: &mut StdRng, length: usize, disorder_factor: f32) -> Vec<String> {
	generate_integers(rng, length, disorder_factor)
		.iter()
		.map(|&x| format!("{:0100}", x))
		.collect()
}

fn time_sort_ms<T: Clone, Sorter>(num_best_of: usize, unsorted: &[T], mut sorter: Sorter) -> (f32, Vec<T>)
where
	Sorter: FnMut(&mut Vec<T>),
{
	let mut best_ns = None;
	let mut sorted = Vec::new();

	for _ in 0..num_best_of {
		let mut vec_clone: Vec<T> = unsorted.to_vec();
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

	(0..BENCH_RESOLUTION_START)
		.map(|x| remap(x, 0, BENCH_RESOLUTION_START, 0.0, BENCH_RESOLUTION_CUTOFF))
		.chain((0..(BENCH_RESOLUTION_END + 1)).map(|x| remap(x, 0, BENCH_RESOLUTION_END, BENCH_RESOLUTION_CUTOFF, 1.0)))
		.collect()
}

fn benchmark_and_plot<T, G>(
	rng: &mut StdRng,
	num_best_of: usize,
	length: usize,
	length_str: &str,
	element_type_short: &str,
	element_type_long: &str,
	mut generator: G,
) where
	T: std::fmt::Debug + Clone + std::cmp::Ord,
	G: FnMut(&mut StdRng, usize, f32) -> Vec<T>,
{
	let bench_disorders = get_bench_disorders();
	let mut pb = ProgressBar::new(bench_disorders.len() as u64);
	pb.message(&format!("Benchmarking {} {}: ", length_str, element_type_long));

	let mut std_ms_list = vec![];
	let mut pdq_ms_list = vec![];
	let mut dmsort_ms_list = vec![];
	let mut dmsort_speedup_list = vec![];

	for &disorder_factor in &bench_disorders {
		let vec = generator(rng, length, disorder_factor);
		let (std_ms, std_sorted) = time_sort_ms(num_best_of, &vec, |x| x.sort());
		let (pdq_ms, pdq_sorted) = time_sort_ms(num_best_of, &vec, |x| x.sort_unstable());
		let (dmsort_ms, dmsort_sorted) = time_sort_ms(num_best_of, &vec, |x| dmsort::sort(x));

		let fastest_competitor_ms = std_ms.min(pdq_ms);

		assert_eq!(pdq_sorted, std_sorted);
		assert_eq!(dmsort_sorted, std_sorted);

		std_ms_list.push(std_ms);
		pdq_ms_list.push(pdq_ms);
		dmsort_ms_list.push(dmsort_ms);
		dmsort_speedup_list.push(fastest_competitor_ms / dmsort_ms);

		pb.inc();
	}
	println!();

	let disorder_percentages: Vec<f32> = bench_disorders.iter().map(|x| x * 100.0).collect();

	use gnuplot::*;
	{
		let mut figure = Figure::new();
		figure.set_terminal(
			"pngcairo",
			&format!("images/comparisons_{}_{}.png", length_str, element_type_short),
		);
		figure
			.axes2d()
			.set_legend(Graph(1.0), Graph(0.05), &[Placement(AlignRight, AlignBottom)], &[])
			.set_border(false, &[Left, Bottom], &[])
			.set_x_ticks(
				Some((Auto, 0)),
				&[Mirror(false), Format("%.0f %%")],
				&[TextColor("#808080")],
			)
			.set_y_ticks(Some((Auto, 0)), &[Mirror(false)], &[TextColor("#808080")])
			.set_x_grid(true)
			.set_y_grid(true)
			.set_title(&format!("Sorting {} {}", length_str, element_type_long), &[])
			.set_x_label("Disorder", &[])
			.set_y_label("ms", &[])
			.lines(
				&disorder_percentages,
				&std_ms_list,
				&[Caption("Vec::sort"), Color("#FF0000"), LineWidth(2.0)],
			)
			.lines(
				&disorder_percentages,
				&pdq_ms_list,
				&[Caption("pdqsort"), Color("#BBBB00"), LineWidth(2.0)],
			)
			.lines(
				&disorder_percentages,
				&dmsort_ms_list,
				&[Caption("Drop-Merge Sort"), Color("#4444FF"), LineWidth(2.0)],
			);
		figure.show();
	}
	{
		let mut figure = Figure::new();
		figure.set_terminal(
			"pngcairo",
			&format!("images/speedup_{}_{}.png", length_str, element_type_short),
		);
		figure
			.axes2d()
			.set_legend(Graph(1.0), Graph(0.05), &[Placement(AlignRight, AlignBottom)], &[])
			.set_border(false, &[Left, Bottom], &[])
			.set_x_ticks(
				Some((Auto, 0)),
				&[Mirror(false), Format("%.0f %%")],
				&[TextColor("#808080")],
			)
			.set_y_ticks(Some((Auto, 0)), &[Mirror(false)], &[TextColor("#808080")])
			.set_x_grid(true)
			.set_y_grid(true)
			.set_title(
				&format!(
					"Drop-Merge sort speedup when sorting {} {}",
					length_str, element_type_long
				),
				&[],
			)
			.set_x_label("Disorder", &[])
			.set_y_label("Speedup over fastest competitor", &[])
			.set_x_range(Fix(0.0), Fix(50.0))
			.set_y_range(Fix(0.0), Fix(8.0))
			.lines(&[0.0, 100.0], &[1.0, 1.0], &[Color("#606060")])
			.lines(
				&disorder_percentages,
				&dmsort_speedup_list,
				&[Color("#4444FF"), LineWidth(2.0)],
			);
		figure.show();
	}
}

/// Benchmark worst-case input for Drop-Merge sort
#[allow(clippy::stable_sort_primitive)]
fn bench_evil() {
	let evil_input: Vec<_> = (100..1000000).chain(0..100).collect();
	let (std_ms, std_sorted) = time_sort_ms(10, &evil_input, |x| x.sort());
	let (pdq_ms, pdq_sorted) = time_sort_ms(10, &evil_input, |x| x.sort_unstable());
	let (drop_ms, drop_sorted) = time_sort_ms(10, &evil_input, |x| dmsort::sort(x));
	// let (drop_ms,    drop_sorted)    = time_sort_ms(10, &evil_input, |x| {dmsort::sort_copy(x); ()});

	assert_eq!(std_sorted, drop_sorted);
	assert_eq!(std_sorted, pdq_sorted);
	println!("Worst-case input:");
	println!("std::sort:       {} ms", std_ms);
	println!("pdqsort:         {} ms", pdq_ms);
	println!("Drop-Merge sort: {} ms", drop_ms);
}

#[test]
#[ignore]
#[rustfmt::skip]
fn benchmarks() {
	bench_evil();

	use rand::SeedableRng;
	let seed = [0; 32];
	let mut rng: StdRng = StdRng::from_seed(seed);

	benchmark_and_plot(&mut rng, 1000,        100, "100",  "i32",    "32-bit integers",  generate_integers);
	benchmark_and_plot(&mut rng, 1000,        100, "100",  "string", "100-byte strings", generate_strings);
	benchmark_and_plot(&mut rng, 1000,      1_000, "1000", "i32",    "32-bit integers",  generate_integers);
	benchmark_and_plot(&mut rng,  100,      1_000, "1000", "string", "100-byte strings", generate_strings);
	benchmark_and_plot(&mut rng,  100,     10_000, "10k",  "i32",    "32-bit integers",  generate_integers);
	benchmark_and_plot(&mut rng,   10,     10_000, "10k",  "string", "100-byte strings", generate_strings);
	benchmark_and_plot(&mut rng,   10,    100_000, "100k", "i32",    "32-bit integers",  generate_integers);
	benchmark_and_plot(&mut rng,    3,    100_000, "100k", "string", "100-byte strings", generate_strings);
	benchmark_and_plot(&mut rng,    5,  1_000_000, "1M",   "i32",    "32-bit integers",  generate_integers);
	benchmark_and_plot(&mut rng,    1,  1_000_000, "1M",   "string", "100-byte strings", generate_strings);
	benchmark_and_plot(&mut rng,    1, 10_000_000, "10M",  "i32",    "32-bit integers",  generate_integers);
}
