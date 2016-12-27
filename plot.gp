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

set style line 1 lc rgb '#AA0000' pt 0 ps 1 lt 1 lw 2
set style line 2 lc rgb '#00AA00' pt  0 ps 1 lt 1 lw 2
set style line 3 lc rgb '#0000AA' pt  0 ps 1 lt 1 lw 2

set yrange [0:]

set key bottom

set xlabel "Randomization"
set xtics format "%.0f%%"

# Generate the comparison graphs:
set output "images/comparisons.png"
set ylabel "ms to sort a million semi-ordered integers"
plot 'data/std_sort.data'        u 1:2 t 'Vec::sort'       w lp ls 1, \
     'data/quicker_sort.data'    u 1:2 t 'Quicksort'       w lp ls 2, \
     'data/drop_merge_sort.data' u 1:2 t 'Drop-Merge sort' w lp ls 3

# Generate the speedup graph:
set output "images/speedup.png"
set ylabel "How much faster drop-merge sort is over quicksort"
set nokey
plot 'data/speedup_over_quickersort.data' u 1:2 t 'speedup' w lp ls 1

# Generate a bar-graph for an example almost-sorted data:
set output "images/example.png"
set term pngcairo size 320, 240
set boxwidth 0.6
set style fill solid 1.00
unset border
set xlabel
set ylabel
unset tics
plot 'data/example.data' u 2:xticlabels(1) with boxes lt rgb "#406090"
