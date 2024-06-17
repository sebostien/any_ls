test:
  cargo test

release: test
  cargo build --release
