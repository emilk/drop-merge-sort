set -e
# rustup default stable
rustup default nightly # Has a faster sort than stable as of January 2017

# export RUST_BACKTRACE=1
cargo test

cargo doc
rm -rf rustdoc/
cp -r target/doc rustdoc

# The slow benchmarks:
time cargo test --release -- --nocapture --ignored
gnuplot plot.gp
