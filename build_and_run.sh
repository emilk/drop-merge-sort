set -e
rustup default nightly
export RUST_BACKTRACE=1
cargo test
time cargo run --release
gnuplot plot.gp
