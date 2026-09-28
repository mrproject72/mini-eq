# TODO

Low-priority backlog. Nothing here blocks a release.

## CI / multi-distro pipeline

CI is currently green on `ubuntu-24.04` after dropping the libadwaita
`v1_7` requirement (WrapBox -> GTK4 FlowBox), so the app builds against
libadwaita 1.5 as shipped by Ubuntu 24.04 LTS.

Ranked by value, all via `container:` jobs (GitHub-hosted runners are
Ubuntu-only):

1. **Containerised PipeWire smoke test — highest value.** All 90 tests are
   unit tests; `pipewire_backend.rs` and `routing.rs` have zero automated
   coverage and are only verified on a dev machine. PipeWire can run in a
   container against a dummy/null sink, which would let CI:
   - create the filter-chain and assert `mini_eq_sink` appears
   - exercise route/unroute and assert streams are handed back to the
     default output (regression test for the "audio stops when the app
     closes" bug)
   - confirm biquad coefficients actually reach the node
2. **Flatpak build job** — Flatpak is the intended distribution model and
   there is no manifest yet. `flatpak-builder` in CI produces the artifact
   and validates the sandbox story.
3. **Fedora** (`fedora:latest`) — largest PipeWire + GNOME desktop overlap;
   exercises the RPM dependency path.
4. **Arch** (`archlinux:latest`) — rolling, so it surfaces libadwaita /
   GTK API drift early, before it reaches users.
5. **Debian stable** (`debian:trixie`) — worthwhile but conservative
   packages make the signal slow-moving.
6. **CachyOS** (`ghcr.io/cachyos/docker`) — Arch-based, so build coverage
   duplicates Arch. Its real differentiator is x86-64-v3/v4 optimised
   packages and a tuned kernel, so the useful job there is a **CPU
   benchmark** (measured %CPU under load vs a stock target) rather than a
   compile check — actual evidence for the project's low-CPU premise.

Skip: Alpine/musl — not a realistic PipeWire desktop target.

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
