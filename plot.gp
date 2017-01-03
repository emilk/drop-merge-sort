# thanks to http://www.gnuplotting.org/attractive-plots/

set term pngcairo size 640, 480 # unlike 'png' we get anti-aliasing

# define axis
# remove border on top and right and set color to gray
set style line 11 lc rgb '#808080' lt 1
set border 3 back ls 11
set tics nomirror

# define grid
set style line 12 lc rgb '#808080' lt 0 lw 1
set grid back ls 12

set style line 1 lc rgb '#FF0000' pt 0 ps 1 lt 1 lw 2
set style line 2 lc rgb '#00BB00' pt 0 ps 1 lt 1 lw 2
set style line 3 lc rgb '#4444FF' pt 0 ps 1 lt 1 lw 2
set style line 4 lc rgb '#800080' pt 0 ps 1 lt 1 lw 2
set style line 5 lc rgb '#606060' pt 0 ps 1 lt 1 lw 2

set key bottom

set xlabel "Disorder"
set xtics format "%.0f%%"

# Generate the comparison graphs:
set xrange [0:100]
set yrange [0:80]
set output "images/comparisons_i32.png"
set title "Sorting one million semi-ordered 32-bit integers"
set ylabel "ms"
plot 'data/i32/std_sort.data'            u 1:2 t 'Vec::sort'               w lp ls 1, \
     'data/i32/quicker_sort.data'        u 1:2 t 'Quicksort'               w lp ls 2, \
     'data/i32/dmsort_move_sort.data'    u 1:2 t 'Drop-Merge sort'         w lp ls 3

set output "images/comparisons_string.png"
set title "Sorting 100 000 semi-ordered 100-character strings"
set ylabel "ms"
plot 'data/string/std_sort.data'       u 1:2 t 'Vec::sort'        w lp ls 1, \
     'data/string/quicker_sort.data'   u 1:2 t 'Quicksort'        w lp ls 2, \
     'data/string/drop_merge_sort.data' u 1:2 t 'Drop-Merge sort' w lp ls 3

# Generate the speedup graph:
set xrange [0:50]
set yrange [0:8]
set nokey

set output "images/speedup_i32_dmsort_copy.png"
set title "Speedup over fastest competitor sorting one million 32-bit integers"
set ylabel "speedup factor"
plot 'data/i32/dmsort_copy_speedup.data' u 1:2 t 'speedup' w lp ls 1

set output "images/speedup_i32_dmsort_move.png"
set title "Speedup over fastest competitor sorting one million 32-bit integers"
set ylabel "speedup factor"
plot 'data/i32/dmsort_move_speedup.data' u 1:2 t 'speedup' w lp ls 1

set output "images/speedup_string.png"
set title "Speedup over fastest competitor sorting 100 000 semi-ordered 100-character strings"
set ylabel "speedup factor"
plot 'data/string/speedup_over_quickersort.data' u 1:2 t 'speedup' w lp ls 1

# Generate the num_dropped graph:
set output "images/num_dropped.png"
set title "How many out-of-order elements where dropped"
set ylabel "percentage"
set xtics format "%.0f%%"
set ytics format "%.0f%%"
set xrange [0:100]
set yrange [0:100]
set term pngcairo size 640, 640
set nokey
plot 'data/i32/num_dropped.data' u 1:2 t 'num_dropped' w lp ls 1

# Generate a bar-graph for an example almost-sorted data:
set output "images/example.png"
unset title
unset xrange
unset yrange
set term pngcairo size 320, 240
set boxwidth 0.6
set style fill solid 1.00
unset border
set xlabel
set ylabel
unset tics
plot 'data/example.data' u 2:xticlabels(1) with boxes lt rgb "#406090"
