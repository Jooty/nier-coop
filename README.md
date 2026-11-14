<p align="center">
  <img src="crates/nt_launcher/assets/hicolor/128.png" width="128" alt="Nier Together" />
  <h1 align="center">Nier Together</h1>
  <h6 align="center">Online multiplayer for NieR Replicant and Nier:Automata on PC.</h6>
</p>

<p align="center">
  <img src="docs/media/two-players.gif" width="720" alt="Two players in the same world" />
</p>


---

## About
Play _NieR Replicant ver.1.22474487139_ or _NieR:Automata_ in multiplayer.

This is a fan project. It is not affiliated with or endorsed by Square Enix.

## Status
Early. If you're looking for a co-op campaign, it doesn't really exist yet. For Replicant, you can play the first half of the game. For Automata, only the prologue works.

**Mostly tested on Linux through Proton. I do not have a Windows machine, so I cannot test it.**

### Replicant
The first half of the story (part 1) is playable in full. 

The guest always plays Kainé, even when it doesn't make sense in the story. If there's a section where Kainé is an AI, the guest replaces her.

* Movement, jumping, attacking, and SFX
* NPC locations and animations
* Companions, which follow the host
* Combat and most sealed verses
* Enemy projectiles
* Boss fights, which the host starts (11/27)
* Downed and revive
* Cutscenes, which are vote-skipped
* Moving between areas
* Doors
* Story progress
* Dialog
* Story choices, which the host makes
* Chests and item spots: both players get the items. Gathering spots are per player
* Each player has their own inventory and gold. Key items are shared
* Weapons
* Music
* Weather and time

### Automata
Only the prologue is a co-op campaign so far.

The guest plays the real companion slot, so they would play 9S at the start. For the prologue, the guest just spectates the host as 2B.

* Movement, attacking, and weapons
* Pod fire and programs
* Some enemies

### Bonuses
* Skip starting movies and the main menu. The launcher handles everything for you, including saves
* Migrate an existing save to be a "co-op save". Right-click a co-op save in the launcher to delete it
* The game will no longer pause in the pause menu

## Getting started
1. Download the launcher from the GitHub Releases page and run it.
2. Settings: press Find to locate your game, then Install mod.
3. One player presses Host a lobby and sends the ticket, the other pastes it and presses Join a lobby. When both are
   ready, the host presses Start.
4. Insert opens the in-game menu.

When something goes wrong, the Logs tab has Copy log and Export logs, to send with a bug report.

## Building
You need the Rust toolchain pinned in `rust-toolchain.toml`. The hooks are Windows DLLs (`x86_64-pc-windows-msvc`);
on Linux, build them with `cargo xwin`. The launcher builds on both Windows and Linux.

## Contributing
Before you open a pull request, run these from the repository root:

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```
