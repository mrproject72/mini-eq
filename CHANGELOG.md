# Changelog

All notable changes to mini-eq will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Rust rewrite project skeleton (Phase 1)
- Core module with biquad coefficient calculation (Phase 2)
- 10-band EQ configuration with 12 filter types
- Unit tests for biquad coefficient calculation
- CLI with clap (background, auto-route, headless, import-apo, check-deps)
- GitHub issue and PR templates
- CI workflow for automated testing
- CONTRIBUTING.md with contribution guidelines
- SECURITY.md for vulnerability reporting

### Changed

- Migrated from Python to Rust for 30% CPU reduction

## [0.1.0] - 2026-09-18

### Added

- Initial Rust rewrite
- Project structure with 30 modules
- Core EQ constants and biquad filter types
- PipeWire filter-chain architecture design
- GTK4 + Libadwaita UI planning
