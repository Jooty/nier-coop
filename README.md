<h1 align="center">Nier Together</h1>
<h6 align="center">Online co-op for NieR Replicant ver.1.22474487139 and NieR:Automata on PC.</h6>

---

## About
Play _NieR Replicant ver.1.22474487139_ or _NieR:Automata_ in co-op with another person.

This is a fan project. It is not affiliated with or endorsed by Square Enix.

## Status
Early. Replicant's first half is playable in full; Automata is just starting.

**Mostly tested on Linux through Proton.**

### Replicant
The first half of the story (part 1) is playable in full. Part 2 and the later playthroughs have not been played in co-op yet.

The guest always plays Kainé.

* Movement, jumping, attacking, and SFX
* NPC locations and animations
* Companions, which follow the host
* Combat and most sealed verses
* Enemy projectiles
* Boss fights, which the host starts
* Downed and revive
* Aggro of enemies
* Cutscenes, which are vote-skipped
* Moving between areas
* Doors
* Story progress, which follows the host
* Dialog, which can be entered midway
* Story choices, which the host makes
* Chests and item spots: both players get the items. Gathering spots are per player
* Each player has their own inventory and gold. Key items are shared
* Weapons
* Music
* Weather and time
* Names over players and a party frame

### Automata
Only the prologue is a co-op campaign so far: the guest watches the host play it. After that, free roam with a map choice.

* Movement, attacking, and weapons
* Pod fire and programs
* Some enemies
* Teleporting to your partner

### Bonuses
* Skip starting movies and the main menu. The launcher handles everything for you, including saves
* Migrate an existing save to be a "co-op save". Right-click a co-op save in the launcher to delete it
* The game will no longer pause in the pause menu

### Warning
Story progress and saves are volatile. There are thousands of flags that must be synced and can fall out of order. Your normal save is never touched; co-op uses its own save file.

A list of bugs that may likely happen:
* Cutscenes can sometimes start early or late, and may not be perfectly synced
* The guest's attack power follows the host's equipped weapon
* Some magic and projectiles do not show or hit for both players
* A player can grab a cube the other player is already holding
* Rides, mini-games, puzzle rooms and fixed-camera rooms run separately for each player
* Checkpoint rewind and random rolls are untested
* In Automata, shared enemies stand and move differently on each screen, and the story can fall out of step between the two players

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
