# Contributing

## Rust style

- Constants are SCREAMING_SNAKE_CASE and the name says what the constant is. No doc comment or `//` line above a constant.
- No doc comment or `//` line above a private function. If it needs explaining, rename it or put a short comment
  inside the body next to the line it explains.
- Game memory is touched only through `rc_game`. Wire types never contain game structs. `unsafe_code` is denied
  in the workspace; a panic must never unwind into the game.
- Keep dependencies light (see `deny.toml` for the allowed licences).

## Commits and pull requests

- One short subject line with a conventional prefix, lowercase: `fix:`, `feat:`, `perf:`, `refactor:`, `docs:`,
  `chore:`, `ci:` or `test:`.
- One branch per change, cut from `origin/main`. Pull requests are squash-merged, so the PR title is the commit subject.
- Write the PR body in plain English: what changed, why, and how you checked it.

## Before you open a PR

Run these from the repository root:

```
cargo fmt --all -- --check
python3 scripts/check_comments.py
python3 scripts/check_style.py
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Notes on the game

- Claims about the game say where they came from and are marked confirmed, web or guess.
- Addresses are image addresses (module base `0x140000000`).
