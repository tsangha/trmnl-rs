# Contributing to trmnl-rs

Thanks for your interest in contributing to trmnl-rs!

## Getting Started

1. Clone the repo: `git clone https://github.com/tsangha/trmnl-rs`
2. Create a branch: `git checkout -b my-feature`
3. Make your changes and submit a PR

## Development Setup

```bash
# Install Rust (if needed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Build
cargo build

# Run tests
cargo test --all-features

# Run clippy
cargo clippy --all-features -- -D warnings

# Format code
cargo fmt
```

## Testing Feature Combinations

The crate has optional features. Test them all:

```bash
cargo check --no-default-features
cargo check --features axum
cargo check --features render
cargo check --features schedule
cargo check --features full
```

## For the `render` Feature

If testing HTML rendering, you'll need:
- Google Chrome or Chromium installed
- Optionally ImageMagick for color reduction

## Pull Request Guidelines

1. **Run all checks before submitting:**
   ```bash
   cargo fmt
   cargo clippy --all-features -- -D warnings
   cargo test --all-features
   ```

2. **Keep PRs focused** - one feature or fix per PR

3. **Update documentation** if adding new public APIs

4. **Add tests** for new functionality

5. **Follow existing code style** - the codebase uses:
   - `thiserror` for error types
   - Builder patterns with `#[must_use]`
   - Feature flags for optional dependencies

## Reporting Issues

When reporting bugs, please include:
- Rust version (`rustc --version`)
- trmnl-rs version
- Feature flags enabled
- Minimal reproduction case

## Questions?

Open a GitHub issue for questions or discussion.
