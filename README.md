# Abstract
This is an implementation of a novel [adaptive sorting](https://en.wikipedia.org/wiki/Adaptive_sort) algorithm optimized for nearly-sorted data. Drop-Merge sort is especially useful for when >85% of the data is already in-order, and the unsorted elements are evenly distributed. An example use-case would be re-sorting an already sorted list after minor modifications.

Drop-Merge sort is 2-4 times faster than quicksort in cases where >95% of the data is already in order, while being considerably simpler to implement than other adaptive sorting algorithms.

# Background
The paper [Item Retention Improvements to Dropsort, a Lossy Sorting Algorithm](http://micsymposium.org/mics_2011_proceedings/mics2011_submission_13.pdf) by Abram Jackson and Ryan McCulloch introduced improvements to the lossy, esoteric sorting algorithm known as *Dropsort*. In Dropsort, out-of-order elements are simply "dropped" (i.e. removed). In the paper, Jackson et al. introduced improvements to Dropsort which improved the detection of out-of-order elements so that more elements would be kept, and fewer dropped.

![Example of almost-sorted data](images/example.png).

(Example input, where most of the data is in order.)

Although the paper does not spell it out, it is in fact proposing a fast algorithm for finding an approximate solution to the [longest non-decreasing subsequence](https://en.wikipedia.org/wiki/Longest_increasing_subsequence) problem.

# Drop-Merge sort
The main idea in Drop-Merge sort is to retain the dropped elements in a separate array. These are then sorted using a standard sorting algorithm (such as quicksort) and then merged into the list of elements that where kept. Thus, Drop-Merge sort is a *lossless* sorting algorithm (the normal kind).

The implementation uses the idea of *memory* from the Jackson et al. paper to detect *mistakes* - elements that probably should have been dropped instead of accepted. Consider the following sequence of numbers:

`0 1 12 3 4 5 6 7 8 9`

The naïve Dropsort algorithm will accept `0 1 12` and then drop the remaining values (as they are all smaller than 12). The idea of memory is to detect when we have dropped a certain number of elements in a row. When this happens, we roll-back and drop the last element before the long run of drops. This is a form of back-tracking which solves the above problem. In the case of the input above, the algorithm will drop the `12` and keep all other elements. In this implementation, we consider the dropping of 8 elements in a row to be the cutoff point at which we roll-back and undo.

Drop-Merge sort will keep in-order elements in situ, moving them towards the start of the array. This helps keep the memory use down and the performance up.

# Performance
To test the performance of this algorithm, I generate almost-sorted data like this (pseudo-code):

```
function generate_test_data(length, randomization_factor) -> Vec {
	result = Vec::new()
	for i in 0..length {
		if random_float() < randomization_factor {
			result.push(random_integer_in_range(0, length))
		} else {
			result.push(i)
		}
	}
	return result
}
```

Comparing this to the default Rust sorting algorithm ([Vec::sort](https://doc.rust-lang.org/beta/std/vec/struct.Vec.html#method.sort), a stable sorting algorithm) and [dual-pivot quicksort](https://github.com/notriddle/quickersort) for different randomization factors:

![Comparing Drop-Merge sort](images/comparisons.png)

We can see that all three algorithms manages to exploit almost-sorted data, but Drop-Merge sort wins out when the randomization factor is less than 25% (more than 75% of the elements are in order). It also behaves well when the data becomes more random, and even when the input data is fully random it is only ~30% slower than quicksort.

Here is another view of the data for 0-25% randomization:

![Speedup over quicksort](images/speedup.png)

Here we can see that we get 4x speedup over quicksort when 99.5% of the elements are in order, and a 2x speedup when 93% of the elements are in order.

# Computational complexity
With `N` elements in the array where `K` elements are out-of-order, Drop-Merge sort performs `O(N + K log K)` comparisons and use `O(K)` extra memory.

# Comparison to other adaptive sorting algorithms
An adaptive sorting algorithm is one that can exploit existing order. These algorithms ranges from the complicated to the simple.

On the complicated end there is the famous [Smoothsort](https://en.wikipedia.org/wiki/Smoothsort), which seems, however, to be quite unpopular - probably due to its complexity. I failed to find a good implementation of Smoothsort to compare Drop-Merge sort against. [Timsort](https://en.wikipedia.org/wiki/Timsort) is a more modern and popular adaptive sorting algorithm. It needs long spans of non-decreasing elements to compete with the performance of Drop-Merge sort. The standard Rust sort uses a variant of Timsort, and as you can see from the performance comparisons, Drop-Merge sort wins for the nearly-sorted cases for which it was designed.

On the simple end of the spectrum there are `O(N²)` algorithms that perform extremely well when there are only one or two elements out of place, or the array is very short (a few hundred elements at most). Examples include [Insertion sort](https://en.wikipedia.org/wiki/Insertion_sort) and [Coctail sort](https://en.wikipedia.org/wiki/Cocktail_shaker_sort).

Drop-Merge sort finds an interesting middle-ground – it is reasonably simple (around 50 lines of code), yet manages to perform well for long arrays. Note, however, that Drop-Merge sort depends on another sorting algorithm (e.g. quick-sort) for sorting the out-of-order elements.

# Limitations
Drop-Merge sort is not stable, which means it will not keep the order of equal elements.

Drop-Merge sort does not sort [in-situ](https://en.wikipedia.org/wiki/In-place_algorithm), but will use `O(K)` extra memory, where `K` is the number of elements out-of-order.

There is a worst-case scenario where all elements are in order, except for the last few who are smaller than all the preceding ones. In this case the algorithm will back-track and drop all the initial elements and keep only the last. The behavior of this is still `O(N log N)` comparisons, but it will use `O(N)` memory and be around 4x slower than the standard rust sort.
