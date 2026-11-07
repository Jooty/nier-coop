<p align="center">
  <img src="crates/rc_launcher/assets/hicolor/128.png" width="128" alt="Replicant Co-op" />
  <h1 align="center">Replicant Co-op</h1>
  <h6 align="center">Online co-op for NieR Replicant ver.1.22474487139 on PC.</h6>
</p>

<p align="center">
  <img src="docs/media/two-players.gif" width="720" alt="Two players in the same world" />
</p>

---

## About
Play _NieR Replicant ver.1.22474487139_ in coop with another person.

This is a fan project. It is not affiliated with or endorsed by Square Enix.

## Status
Early. A full play-through isn't possible yet.

**Not really tested on Windows. Debugging happens with Linux through Proton.**

### What's Synced
* Movement, jumping, attacking, and SFX
* NPC locations and animations
* Combat and sealed verses
* Aggro of enemies
* Cutscenes, which are vote-skipped
* Moving between areas
* Doors
* Dialog, which can be entered midway
* Pickups are client-side, but both of you will see it
* Shops and purchases
* Each player has their own inventory and gold. Key items are shared
* Weapons
* Music
* Weather and time

### Bonuses
* Skip starting movies and the main menu. The launcher handles everything for you, including saves
* Migrate an existing save to be a "coop save"
* The guest can select to play as a different version of Nier, and inevitably Kaine
* The game will no longer pause in the pause menu

### Warning
Story progress and saves are volatile. There are thousands of flags that must be synced and can fall out of order. 

A list of bugs that may likely happen:
* Cutscenes can sometimes start early or late, and may not be perfectly synced
* Cutscenes that are invoked from proximity will only play for the player that initiated it
* Bosses that are invoked from proximity will only spawn for the player that initiated it
* Enemies on the guest will slide after being struck
* Full game crashes are rare and will happen on story moments like a cutscene starting or ending

## Getting started
1. Download the launcher from the GitHub Releases page and run it.
2. Settings: press Find to locate your game, then Install mod.
3. New game: type your name and pick your character. One player presses Host a lobby and sends the ticket, the other
   pastes it and presses Join a lobby. When both are ready, the host presses Start.
4. Continue: each player picks a co-op save slot, then host and join the same way.
5. Insert opens the in-game menu.

When something goes wrong, the footer has Copy log and Export logs, to send with a bug report.

## Building
You need the Rust toolchain pinned in `rust-toolchain.toml`. The hook is a Windows DLL (`x86_64-pc-windows-msvc`);
on Linux, build it with `cargo xwin`. The launcher builds on both Windows and Linux.

## Contributing
Before you open a pull request, run these from the repository root:

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```
