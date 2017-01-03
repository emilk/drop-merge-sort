set -e
# rustup default stable
rustup default nightly
# export RUST_BACKTRACE=1
cargo test

# The slow benchmarks:
time cargo test --release -- --nocapture --ignored
gnuplot plot.gp
