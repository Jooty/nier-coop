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
Early. Played by hand with two copies of the game on one PC; a session between two machines has not been tried yet.

What works today:
- Host and Join from the launcher, with an invite and an optional password.
- Your partner in your world: position, facing, every animation, their weapon, and their effects and sounds, which
  fade with distance.
- A name tag over your partner and a party frame under your health bar, both in the game's own font and art.
- A log at the bottom left for co-op notices.
- Staying together: the guest follows the host into each new area. Only the host changes areas, and only with
  everyone nearby. Doors inside an area work for both players.
- Downed and revive: a lethal hit leaves you downed, and your partner revives you by standing next to you.
- Co-op saves in their own file. Your normal saves are never written while you play co-op.
- Host settings that apply to everyone: names over players, the party frame, distance and ping.

What does not work yet:
- Enemies are only partly shared: the guest's enemies still act on their own.
- Weather, time of day and music are each player's own.
- NPCs, dialogue, items, chests and story progress are each player's own.

### What's next
- The world in sync: weather and time of day, music, moving between areas together, your partner on the minimap.
- NPCs, shared dialogue, items and chests, with key items going to both players.
- Shared enemies and combat.
- Player 2 as a companion character.

## Getting started
1. Get the launcher and run it.
2. Settings tab: press Find to locate your game, or type its folder and press Apply. Setup tab: Install puts the mod
   in place (Host and Join also install or update it for you).
3. Saves tab: pick the co-op save to play. "Copy into co-op" copies one of your normal saves into the co-op file.
4. Play tab. To host, set a password if you want one, press Host and send the invite. To join, paste the invite and
   the password and press Join.
5. The launcher starts the game and loads your save. Insert opens the in-game menu.

The Settings tab also holds your name, "keep running in the background" and the event log. The host's choices for
names over players, the party frame, distance and ping apply to everyone; a guest sees them greyed out. The guest's
game does not save unless "Guest keeps saving" is ticked.

When something goes wrong, the Logs tab has Copy all and Export logs, to send with a bug report.

## Building
You need the Rust toolchain pinned in `rust-toolchain.toml`. The hook is a Windows DLL (`x86_64-pc-windows-msvc`);
on Linux, build it with `cargo xwin`. The launcher builds on both Windows and Linux. On Linux,
`scripts/install-launcher` installs the launcher, its icon and a menu entry.

<!-- TODO: GIF of two players in sync -->

## Licence
AGPL-3.0-or-later. See [LICENSE](LICENSE).

## Credits
See [CREDITS.md](CREDITS.md).
