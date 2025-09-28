# Repository Guidelines

## Project Structure & Module Organization
Source lives under `src/`. `main.rs` wires together recording, Hyprland bindings, and notifications. Supporting modules are split by concern: `audio.rs` handles `cpal` streaming and WAV encoding, `input.rs` manages Hyprland IPC, `notification.rs` wraps swayosd progress toasts, and `error.rs` centralizes error types. Release artifacts land in `target/`. Nix files (`flake.nix`, `flake.lock`) describe the dev shell; adjust them if toolchains change.

## Build, Test, and Development Commands
- `cargo check` — fast validation of the codebase; run before pushing.
- `cargo fmt` / `cargo fmt -- --check` — apply and verify the enforced formatting rules in `rustfmt.toml`.
- `cargo clippy --all-targets --all-features` — lint with Rust’s static analyzer; fix warnings or mark justified exceptions.
- `cargo test` — execute unit and (future) integration tests.
- `cargo run` — run the recorder locally; combine with Hyprland to exercise keybindings.
- `cargo build --release` — produce optimized binaries for distribution.

## Coding Style & Naming Conventions
Follow rustfmt’s defaults plus the repo overrides (80-column width, crate-level import groups, trailing commas). Use 4-space indentation, `snake_case` for modules/functions, `CamelCase` for types, and `SCREAMING_SNAKE_CASE` for constants. Prefer `anyhow::Result` for fallible paths and `tracing` spans for observability. Keep modules focused and prefer top-level `pub(crate)` visibility unless external consumers need access.

## Testing Guidelines
Unit tests live beside implementations using `#[cfg(test)]` modules; integration tests can sit under a future `tests/` directory mirroring executable flows. Name tests after the behavior under scrutiny (e.g., `saves_file_on_enter`). Mock external effects via small helper traits rather than reaching into hardware. Run `cargo test` before every commit, and add regression tests when fixing bugs.

## Commit & Pull Request Guidelines
Commits follow short imperative prefixes (`chore:`, `feat:`, `fix:`, etc.) as seen in history; keep scope narrow and include rationale in the body if needed. PRs should describe user-facing impact, reference issues when applicable, and call out manual testing (e.g., keybinding verification on Hyprland). Attach logs or terminal output when touching audio/input paths so reviewers can reproduce.

## Environment & Tooling Notes
Development assumes a Wayland session with Hyprland, swayosd, and `hyprctl` available. Recordings default to `~/Recordings`; ensure your environment has write access. When updating system-level dependencies, document changes in `INSTALL.md` to assist other agents provisioning the same tooling.
