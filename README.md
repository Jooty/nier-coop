<p align="center">
  <img src="crates/rc_launcher/assets/hicolor/128.png" width="128" alt="Replicant Co-op" />
  <h1 align="center">Replicant Co-op</h1>
  <h6 align="center">Online co-op for NieR Replicant ver.1.22474487139 on PC.</h6>
</p>

---

## About
Both players run their own copy of the game and see each other in the same world. Connections are peer-to-peer: one
player hosts and shares a ticket, the other joins with it. The ticket is the only secret, so share it only with the
person you want to play with. Needs your own copy of the game; it works on Windows, and on Linux through Proton.

This is a fan project. It is not affiliated with or endorsed by Square Enix.

## Status
Early. Played by hand with two copies of the game on one PC; a session between two machines has not been tried yet.

What works today:
- Host and Join from the launcher with a ticket.
- Your partner in your world: position, facing, every animation, their weapon, and their effects and sounds, which
  fade with distance.
- A name tag over your partner and a party frame under your health bar, both in the game's own font and art.
- Your partner on the minimap.
- A log at the bottom left for co-op notices, including item pickups.
- Changing zones together: the guest follows the host into each new area. Only the host changes areas, and only when
  all players are at the door.
- Doors inside an area work for both players.
- Weather, time of day and music follow the host.
- NPCs stand in the same place for both players.
- Dialogue is shared, and you see "in dialogue" while your partner talks.
- Key items go to both players.
- Downed and revive: a lethal hit leaves you downed, and your partner revives you by standing next to you.
- Co-op saves in their own file. Your normal saves are never written while you play co-op, and a guest never saves.
- Host settings that apply to everyone: names over players, the party frame, distance and ping.

What does not work yet:
- Shared enemies and combat: the guest's enemies still act on their own.
- Story progress.
- Chests and opened doors.
- Difficulty settings.
- Player 2 as a companion character.

## Getting started
1. Download the launcher from the GitHub Releases page and run it.
2. Settings tab: press Find to locate your game, or type its folder and press Apply. Setup tab: Install puts the mod
   in place (Host and Join also install or update it for you).
3. Saves tab: pick the co-op save to play. "Copy into co-op" copies one of your normal saves into the co-op file.
4. Play tab. To host, press Host and send the ticket. To join, paste the ticket and press Join.
5. The launcher starts the game and loads your save. Insert opens the in-game menu.

The Settings tab also holds your name, "keep running in the background" and the event log. The host's choices for
names over players, the party frame, distance and ping apply to everyone; a guest sees them greyed out.

When something goes wrong, the Logs tab has Copy all and Export logs, to send with a bug report.

## Building
You need the Rust toolchain pinned in `rust-toolchain.toml`. The hook is a Windows DLL (`x86_64-pc-windows-msvc`);
on Linux, build it with `cargo xwin`. The launcher builds on both Windows and Linux.

## Contributing

### Rust style
- Constants are SCREAMING_SNAKE_CASE and the name says what the constant is. No doc comment or `//` line above a constant.
- No doc comment or `//` line above a private function. If it needs explaining, rename it or put a short comment
  inside the body next to the line it explains.
- Game memory is touched only through `rc_game`. Wire types never contain game structs. `unsafe_code` is denied
  in the workspace; a panic must never unwind into the game.
- Keep dependencies light (see `deny.toml` for the allowed licences).

### Commits and pull requests
- One short subject line with a conventional prefix, lowercase: `fix:`, `feat:`, `perf:`, `refactor:`, `docs:`,
  `chore:`, `ci:` or `test:`.
- One branch per change, cut from `origin/main`. Pull requests are squash-merged, so the PR title is the commit subject.
- Write the PR body in plain English: what changed, why, and how you checked it.

### Before you open a PR
Run these from the repository root:

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

### Notes on the game
- Claims about the game say where they came from and are marked confirmed, web or guess.
- Addresses are image addresses (module base `0x140000000`).

<!-- TODO: GIF of two players in sync -->
