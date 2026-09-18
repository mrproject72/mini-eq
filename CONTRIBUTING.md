# Contributing to mini-eq

Thank you for your interest in contributing to mini-eq! This document provides guidelines and instructions for contributing.

## Getting Started

### Prerequisites

- Rust 1.98.1 or later
- GTK4 development libraries
- Libadwaita development libraries
- PipeWire development libraries
- `pkg-config`

### Build Instructions

```bash
# Clone the repository
git clone git@github.com:mrproject72/mini-eq.git
cd mini-eq

# Install system dependencies (if not already installed)
sudo apt-get install -y libgtk-4-dev libadwaita-1-dev libgraphene-1.0-dev libpipewire-0.3-dev libspa-0.2-dev pkg-config

# Build
cargo build --release

# Run
cargo run --release

# Test
cargo test --lib
```

### Development Build

```bash
# Check for compilation errors
cargo check --release

# Run tests
cargo test

# Format code
cargo fmt

# Lint
cargo clippy --release -- -D warnings
```

## How to Contribute

### Reporting Bugs

Use the [Bug Report template](https://github.com/mrproject72/mini-eq/issues/new?template=bug_report.yml). Include:

- mini-eq version or commit hash
- Steps to reproduce
- Expected vs actual behavior
- Relevant log output (`--verbose` flag)
- System information (PipeWire version, GTK version, OS)

### Suggesting Features

Use the [Feature Request template](https://github.com/mrproject72/mini-eq/issues/new?template=feature_request.yml). Include:

- Feature title and description
- Motivation and problem it solves
- Alternatives considered
- Additional context

### Submitting Pull Requests

1. **Fork** the repository
2. **Create a feature branch** from `main`:
   ```bash
   git checkout -b feature/your-feature-name
   ```
3. **Make your changes** following the coding conventions below
4. **Test thoroughly**:
   ```bash
   cargo check --release
   cargo test --lib
   cargo clippy --release -- -D warnings
   cargo fmt -- --check
   ```
5. **Commit** with a clear message:
   ```bash
   git commit -m "feat: add your feature description"
   ```
6. **Push** to your fork:
   ```bash
   git push origin feature/your-feature-name
   ```
7. **Open a Pull Request** using the PR template

## Coding Conventions

### Rust Style

- Follow [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `cargo fmt` to format code
- Use `cargo clippy` to catch common issues
- All code must pass `cargo clippy --release -- -D warnings`

### Module Organization

- Each module corresponds to a specific concern (see `src/lib.rs`)
- Core logic in `src/core.rs`
- PipeWire integration in `src/pipewire_*.rs`
- UI components in `src/window_*.rs`
- Add module declarations to `src/lib.rs`

### Naming Conventions

- Functions and variables: `snake_case`
- Types and structs: `PascalCase`
- Constants: `UPPER_SNAKE_CASE`
- Modules: `snake_case`

### Error Handling

- Use `thiserror` for error types
- Use `anyhow` for application-level errors
- Never panic on expected error conditions
- Provide meaningful error messages

## Project Structure

```
src/
├── main.rs              # Entry point with CLI
├── core.rs              # EQ constants, biquad coefficients
├── pipewire_backend.rs  # PipeWire connection
├── pipewire_routes.rs   # Output route detection
├── pipewire_stream_router.rs  # Stream routing
├── filter_chain.rs      # DSP filter-chain configuration
├── window.rs            # Main GTK window
├── ...                  # Other modules
```

## Development Phases

- **Phase 1**: Project skeleton ✅
- **Phase 2**: Core module (biquad coefficients) ✅
- **Phase 3**: PipeWire backend 🔄
- **Phase 4**: GTK4/Libadwaita UI
- **Phase 5**: Analyzer (FFT, LUFS)
- **Phase 6**: AutoEq/APO presets
- **Phase 7**: Background mode, D-Bus, desktop integration
- **Phase 8**: Testing, Flatpak packaging, performance

## Commit Message Format

Use [conventional commits](https://www.conventionalcommits.org/):

- `feat:` new feature
- `fix:` bug fix
- `docs:` documentation changes
- `style:` formatting changes (no logic changes)
- `refactor:` code changes without fixing bugs or adding features
- `test:` adding or modifying tests
- `chore:` maintenance tasks

Examples:
```
feat: add PipeWire virtual sink creation
fix: correct biquad coefficient calculation for HiShelf
docs: update AGENTS.md with build instructions
```

## Code of Conduct

- Be respectful and constructive in all interactions
- Follow the project's coding conventions
- Test your changes thoroughly before submitting
- Help others with questions and code reviews

## License

By contributing, you agree that your contributions will be licensed under the [GPL-3.0-or-later license](LICENSE).

## Questions?

Feel free to open an issue for questions about contributing. Use the [Bug Report template](https://github.com/mrproject72/mini-eq/issues/new?template=bug_report.yml) and label it as `question`.
