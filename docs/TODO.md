# TODO

Low-priority backlog. Nothing here blocks a release.

## CI / release flow

The current `.github/workflows/ci.yml` runs `fmt`, `clippy -D warnings`,
`check --release`, `test --lib` and `build --release` on push and PR to
`main`. Gaps worth closing eventually:

- **Branch protection on `main`** — require the CI check to pass and at
  least one review before merge. Right now nothing enforces it.
- **Release artifacts** — the release job builds but uploads nothing. Add
  `actions/upload-artifact` for the release binary, and ideally a
  `.deb`/Flatpak bundle.
- **Tag-triggered releases** — no workflow on `push: tags: [v*]`, so
  tagging does nothing.
- **No integration test on live PipeWire** — all 90 tests are unit tests.
  The backend (`pipewire_backend.rs`, `routing.rs`) is only verified on a
  developer machine. A containerised PipeWire smoke test would catch real
  regressions.
- **No coverage reporting.**

## Version numbering and changelog

The crate has sat at `0.9.0` for the whole rewrite with no changelog
discipline. Needs:

- a real version scheme from a numbered first release
- `CHANGELOG.md` (Keep a Changelog format), back-filled from
  `docs/*-updates.md`
- README updated with the version history

## Feature gaps

- **Preset lifecycle** — no revert/reapply, import/export/delete, file
  monitoring, fallback presets, or output-preset linking.
- **Fader bottom residual clip** at some window sizes (see Known Issues).
- **Flatpak packaging** and **GNOME Shell extension**.
- **Config migration** — the app ID and config directory were renamed to
  `io.github.mrproject72.mini_eq_rr` / `~/.config/mini-eq-rr`. Existing
  settings under the old `~/.config/mini-eq` are **not** picked up. If
  that matters, add a one-shot migration on first run.
