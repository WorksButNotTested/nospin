default: fix fmt build clippy doc build test

build:
  cargo build

fmt:
  cargo fmt
  taplo fmt

fix:
    cargo fix --allow-dirty --allow-staged

clippy:
  cargo clippy

doc:
  cargo doc

test:
  cargo test -F test

