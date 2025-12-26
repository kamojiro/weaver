
set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

default:
  @just --list

fmt:
  cargo fmt --all

clippy:
  cargo clippy --workspace --all-targets --all-features -- -D warnings

test:
  cargo test --workspace

check: fmt clippy test

# Auto-fix lints where possible (may edit code)
fix:
  cargo fmt --all
  cargo clippy --workspace --all-targets --all-features --fix --allow-dirty --allow-staged

doc:
  cargo doc --workspace --no-deps

clean:
  cargo clean
