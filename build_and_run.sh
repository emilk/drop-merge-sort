set -e
# rustup default stable
rustup default nightly # Has a faster sort than stable as of January 2017

# export RUST_BACKTRACE=1
cargo test

# Publish documentation in docs/ folder to be compatible with github pages:
cargo doc
rm -rf docs/
cp -r target/doc docs

# The slow benchmarks:
time cargo test --release -- --nocapture --ignored
gnuplot plot.gp
