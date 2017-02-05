set -e
rustup default stable
# rustup default nightly

# export RUST_BACKTRACE=1
cargo test

# Publish documentation in docs/ folder to be compatible with github pages.
# It can be found on https://emilk.github.io/drop-merge-sort/dmsort/index.html
cargo doc && rm -rf docs/ && cp -r target/doc docs

# The slow benchmarks that make the plots:
time cargo test --release -- --nocapture --ignored

rustc --version
