extern crate dmsort;

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::panic;

#[test]
fn simple_tests() {
	fn test_type<T: Clone + PartialEq + Ord + std::fmt::Debug>(unsorted: Vec<T>) {
		let mut dm_sorted = unsorted.clone();
		dmsort::sort(&mut dm_sorted);

		let mut std_sorted = unsorted.clone();
		std_sorted.sort();

		assert_eq!(dm_sorted, std_sorted, "FAIL with input {:?}", unsorted);
	}

	fn test(list: Vec<i32>) {
		test_type(list.iter().map(|&x| format!("{:02}", x)).collect());
		test_type(list);
	}

	test(vec![]);
	test(vec![0]);
	test(vec![0, 1]);
	test(vec![1, 0]);
	test(vec![0, 1, 2]);
	test(vec![0, 2, 1]);
	test(vec![1, 0, 2]);
	test(vec![1, 2, 0]);
	test(vec![2, 0, 1]);
	test(vec![2, 1, 0]);
	test(vec![0, 1, 3, 2, 4, -5, 6, 7, 8, 9]);
	test(vec![0, 1, 10, 3, 4, 5, 6, 7, 8, 9]);
	test(vec![10, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
	test(vec![0, 0, 2, 3, 4, 1, 6, 1, 8, 9]);
	test(vec![20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19]);
	test(vec![20, 21, 2, 23, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19]);
}

#[test]
fn test_unwind() {
	// The purpose of this test is to make sure that if there is a panic in the compare function
	// then we will unwind the stack in such a way that the slice we are sorting
	// contains all elements it had when called (but maybe in a different, partially-sorted order).
	// This is crucial in order to prevent double-frees.
	//
	struct TestSortType<'a> {
		id:          usize,
		dropped:     &'a RefCell<BTreeSet<usize>>,
	}
	impl<'a> Drop for TestSortType<'a> {
		fn drop(&mut self) {
			let did_insert = self.dropped.borrow_mut().insert(self.id);
			assert!(did_insert, "Double-free of {}", self.id);
		}
	}

	for break_after_this_many_comparisons in 0..14 {
		let scheuled_panic_code: String = String::from("This is a scheduled panic");

		let dropped = RefCell::new(BTreeSet::new());

		let catch_result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
			let mut data = [
				TestSortType{id: 0, dropped: &dropped},
				TestSortType{id: 1, dropped: &dropped},
				TestSortType{id: 5, dropped: &dropped},
				TestSortType{id: 6, dropped: &dropped},
				TestSortType{id: 2, dropped: &dropped},
				TestSortType{id: 3, dropped: &dropped},
				TestSortType{id: 4, dropped: &dropped},
			];
			let mut num_comparisons = 0;

			dmsort::sort_by(&mut data, |a, b| {
				if num_comparisons == break_after_this_many_comparisons {
					panic!(scheuled_panic_code.clone());
				}
				num_comparisons += 1;
				a.id.cmp(&b.id)
			});
			panic!("We where supposed to abort after {} comparisons, but we only did {}",
			       break_after_this_many_comparisons, num_comparisons);
		}));

		// Make sure we did panic:
		assert!(catch_result.is_err());

		// Make sure we panicked for the right reason:
		let error = catch_result.err().unwrap();
		assert!(error.is::<String>());
		assert_eq!(*error.downcast_ref::<String>().unwrap(), scheuled_panic_code);

		// Make sure we dropped all objects:
		assert_eq!(dropped.borrow_mut().len(), 7);
	}
}
