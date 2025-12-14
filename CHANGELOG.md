# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2024-12-14

### Added

- Initial release of the TRMNL BYOS framework
- Core protocol types: `DeviceInfo`, `DisplayResponse`, `SetupResponse`, `LogEntry`
- `TokenAuth` for optional endpoint authentication
- `axum` feature: Axum extractors for `DeviceInfo` and `TokenAuth`
- `render` feature: HTML-to-PNG rendering via Chrome headless
- `schedule` feature: Time-based refresh rate scheduling with YAML config
- Battery percentage calculation helper
- Comprehensive documentation and examples
- TRMNL firmware quirk handling (token URL suffix stripping)

### Technical Details

- Zero unsafe code
- Minimal dependencies (replaced `url` crate with `form_urlencoded`)
- `#[non_exhaustive]` on `Error` enum for API stability
- Feature flags properly isolate optional dependencies
- MSRV: Rust 1.70

[0.1.0]: https://github.com/tsangha/trmnl-rs/releases/tag/v0.1.0
