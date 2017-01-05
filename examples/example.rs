extern crate dmsort;

fn main() {
	let mut numbers: Vec<i32> = vec![0, 1, 6, 7, 2, 3, 4, 5];

	// Sort with custom key:
	dmsort::sort_by_key(&mut numbers, |x| -x);
	assert_eq!(numbers, vec![7, 6, 5, 4, 3, 2, 1, 0]);

	// Sort with Ord trait:
	dmsort::sort(&mut numbers);
	assert_eq!(numbers, vec![0, 1, 2, 3, 4, 5, 6, 7]);

	// Sort with custom compare:
	dmsort::sort_by(&mut numbers, |a, b| b.cmp(a));
	assert_eq!(numbers, vec![7, 6, 5, 4, 3, 2, 1, 0]);
}
