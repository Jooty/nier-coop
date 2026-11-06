<p align="center">
  <img src="crates/rc_launcher/assets/hicolor/128.png" width="128" alt="Replicant Co-op" />
  <h1 align="center">Replicant Co-op</h1>
  <h6 align="center">Online co-op for NieR Replicant ver.1.22474487139 on PC.</h6>
</p>

---

## About
Both players run their own copy of the game and see each other in the same world. Connections are peer-to-peer: one
player hosts and shares an invite, the other joins with it. An optional password keeps strangers out. There is no
server.

Replicant Co-op is bring-your-own-game. Nothing from the game ships with the mod, so you need your own copy. It works
on Windows, and on Linux through Proton.

This is a fan project. It is not affiliated with or endorsed by Square Enix.

## Status
Early. It is built and tested on one machine, and a full session between two machines has not been tried by hand yet.

What works today:
- Host and Join from the launcher, with an invite and an optional password.
- Installing and removing the mod, with the game folder found for you.
- The other player shows up as a puppet that moves and plays the same motion, and goes away when they leave the area
  or disconnect.
- A small co-op panel with the link state, ping and distance to the other player.

What does not work yet:
- Enemies and combat are not shared.
- Players have to be in the same area. Nothing moves you to the host when they change area.
- Your co-op session uses your own save.

### What's next
- Shared enemies and combat.
- Following the host between areas.
- Co-op saves kept separate from your own.
- A name tag over your partner.
- Player 2 as a companion character.

## Getting started
1. Get the launcher and run it. See [the launcher](docs/on-hand/launcher.md).
2. Press Find the game, or point it at your game folder. It installs the mod for you.
3. Host and share the invite, or Join and paste the invite you were given.
4. Start the game from the launcher.

The full steps for two players are in [co-op](docs/on-hand/co-op.md).

## Building
You need the Rust toolchain pinned in `rust-toolchain.toml`. The hook is a Windows DLL (`x86_64-pc-windows-msvc`);
on Linux, build it with `cargo xwin`. The launcher builds on both Windows and Linux.

<!-- TODO: GIF of two players in sync -->

## Licence
AGPL-3.0-or-later. See [LICENSE](LICENSE).

## Credits
See [CREDITS.md](CREDITS.md).
