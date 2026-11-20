# Statistics+

Version 1.0.0 · Minecraft Java 26.3 · Sunrise API 0.2 · Mod SDK `9425651` · MIT licence

Statistics+ is written in Rust for Sunrise. It needs no Java or Fabric.
It replaces the pause-menu Statistics button with a searchable, sortable live stats screen, and can save or load a JSON snapshot on this computer.

This is an independent implementation. It makes no network requests of its own: no author API, news, credits or announcements. It does not collect, hash or store machine or hardware identifiers. The only traffic is the game's own request-stats command to the server you are playing on.

<img width="1116" height="840" alt="image" src="https://github.com/user-attachments/assets/4a39fe52-0c6e-4f37-8f3a-3073475e3929" />
<img width="1116" height="840" alt="image" src="https://github.com/user-attachments/assets/f471ecca-7304-4ea8-9d6f-de835bd8eb07" />
<img width="1116" height="840" alt="image" src="https://github.com/user-attachments/assets/666b450b-c13a-4dfe-8c74-ed0c1689de60" />
<img width="1116" height="840" alt="image" src="https://github.com/user-attachments/assets/606d5cd7-c8ec-489b-9f0e-10eb8c106313" />
<img width="1116" height="840" alt="image" src="https://github.com/user-attachments/assets/d0870387-e1bf-4149-a02f-c6f6d795303d" />
<img width="1116" height="840" alt="image" src="https://github.com/user-attachments/assets/5e5c02d4-782a-49f9-b7ee-e97b0c207e05" />


## Features

- Click **Statistics** on the pause menu to open Statistics+.
- Open Statistics+ from the pause-menu **Statistics** button or with `/statsplus panel`. Sunrise's Mods screen has no button slot for native mods, so the earlier build's mods-list buttons are gone. Use `/statsplus empty` and `/statsplus save` instead.
- When a server is connected, the client requests a fresh stats update (the ordinary request-stats command). The list then follows the values the game already holds.
- Views: **General**, **Items**, **Blocks**, **Mobs**, **Food**, and **Files**.
- On the Mobs tab, a category chip cycles **All** / **Hostile** / **Passive**. Hostile is the game’s `monster` category; Passive covers the rest (animals, ambient, water, and misc). There is no separate Neutral bucket, because the game does not expose one.
- Search by name or id. Sort by listed order, name, or value. Toggle **Filled** / **All** to hide or show empty rows. Group by kind on the Mobs tab; Source grouping (by namespace) appears automatically when a tab’s statistics include more than one namespace, which keeps vanilla Minecraft on a single list.
- Distance units cycle cm / m / km. Time units cycle ticks / seconds / minutes / hours / days. Twenty game ticks are one second; distance is stored in centimetres.
- Scrollbars share one viewport height for painting, clicks, and scrolling, so every row can be reached, including the last one.
- The mouse wheel moves one row per notch. List rows are clipped to the list viewport, so they cannot draw over the toolbar.
- The card stays inside the window with margins, even in a small or narrow window or at a larger GUI scale. The search field, toolbar, and tabs shrink or wrap to fit, and the list uses the space that remains.
- Items, blocks, and food show item icons. Mobs show a card with a spawn-egg icon when the game has one.
- Optional JSON snapshots live under the mod data directory, in a `snapshots` folder (`sunrise-mods/.sunrise-mod-data/statsplus/snapshots` under the game folder). Save and load from the Files tab, or `/statsplus save` and `/statsplus load [name]`. There is no operating-system file picker, and nothing is uploaded.


## Look and settings

Statistics+ uses the shared Sunrise dark look: a `#0F1118` card with a 2 px accent edge, tabs and
charcoal chips. The accent is Sunrise orange `#F9801D` at first; change it with **Accent colour**
in the settings card (eight presets, or Reset).
The Statistics screen uses the same look.

The settings card opens from SunnyMods' **Settings** button, from Sunrise's own Mods list, or
from the mod's settings key. **Key binds › Rebind** switches to Sunrise's classic page to change keys.

## Install (local)

You need the Sunrise launcher with native mods allowed. Use either way below.

- **From the file:** copy `statsplus-1.0.0.smod` into the instance's `sunrise-native-mods/` folder, next to `options.txt`. Turn it on in the launcher's **Mods** page, or add the line `mod.statsplus:true` to `sunrise-mods.txt`. The launcher then compiles it for that instance (`sunrise-launcher native-build`).
- **From GitHub:** once the repository is public, use **Add from GitHub** on the launcher's Mods page, or run `sunrise-launcher mods add-github https://github.com/Zekrenlight/statistics-plus --trust-native-code`.

Native mods run with full access to your computer. Read the source before you install one.

Then open the pause menu and click **Statistics**, or type `/statsplus panel`.

## Commands

There is no keybind. Open the screen from the pause-menu Statistics button, or with a chat command.

Chat commands use the prefix `/statsplus`: `panel`, `save`, `load [name]`, `empty`, and `help`. `/statsplus panel` toggles the screen closed if it is already open.

Settings are stored in `statsplus.cfg` in `sunrise-mods/.sunrise-mod-data/statsplus/` (format version 1). There is no automatic import of another mod's config.

## Menu

The screen is a centered tabbed card, in the same family as Hookline's panel (navy, parchment text, gold accents — not a copy of another stats menu). It shrinks and reflows to stay on screen:

- **General** — custom statistics (time, distance, and the rest of the usual list), plus a live session timer that is not written into snapshots
- **Items** — used, crafted, broken, picked up, and dropped
- **Blocks** — mined and used, with block icons
- **Mobs** — killed and killed-by, as cards with spawn-egg icons when those exist. A category chip filters All, Hostile, or Passive.
- **Food** — edible items and how they were used
- **Files** — snapshots already on disk, listed by name, date, and play time (newest first). **Save** writes a new file (named with the local time, such as 2026-11-12 17:04). **Load** opens a read-only view of the other tabs, with a “Viewing snapshot X — Live” banner to return to live stats. **Delete** asks for a second click within 3 seconds. The Filled, Sort, and Group chips are hidden on this tab; search still filters snapshot names. Distance and time units appear only on General; Items, Blocks, and Food hide that chip and let search and the remaining chips use the space. Mobs keeps its All/Hostile/Passive chip. The panel shows a status line after save, load, or an error, and an empty-state message when nothing has been saved yet

A search field, Filled/All, sort, group, and unit chips sit above the list. **Done** or Escape closes the screen and returns to the pause menu if you opened it from there, or to the world otherwise. `/statsplus panel` toggles the screen closed if it is already open.


## How it fits into Sunrise

Statistics+ takes over the game's own Statistics screen. It reads the stats table, the item and mob registries, and translations, and the typed mod API has none of them. So the game side is native:

- **Support modules**, placed beside `native_hooks/client_tick_post.rs`:
  - `src/native/session.rs`: the stats cache, snapshots and commands
  - `src/native/panel.rs`: the card
- **A cancellable `HEAD` mixin on `Game::draw_stats`**: hides the stock widgets.
- **`HEAD` mixins** on these Sunrise hook points:
  - `native_command` (cancellable)
  - `native_client_tick_post`
  - `native_render_frame_pre`
  - `native_screen_render_post`
  - the screen mouse, wheel, key and char `_pre` hooks

The rules in `src/logic.rs` are plain Rust and are tested with `cargo test --lib`. A mod with a mixin works only on the exact build it was made for.


## Building

```sh
cargo sunrise-mod           # fetches the pinned SDK into .sunrise/ (gitignored)
cargo sunrise-mod check     # type-checks against the pinned SDK
cargo test --lib            # unit tests, no game needed
cargo sunrise-mod build     # target/sunrise-mod/statsplus.smod
```

To move to a newer SDK, run `cargo sunrise-mod update --sunrise <build>` (it rewrites the
`sunrise` and `mixins` lines in `sunrise-mod.toml`), then check and rebuild.

Install the `.smod` with sunrise-launcher (**Add from file**), or point **Add from GitHub** at
this repository. Native mods run with full access to your computer; read the source first.

## Compatibility

| Piece | Pin |
| --- | --- |
| Minecraft Java | 26.3 |
| Sunrise API | 0.2 |
| Mod SDK (`[native].sunrise`) | `94256510f74e6843754e0a93fd9e4da7e45b1056` (`9425651`, 19 Nov 2026) |
| Mixins (`[native].mixins`) | `e59da9094f0c2fdda6c627fe9eff22dec6a906f0` |

Statistics+ works on its own and alongside the other Sunrise mods by Zekrenlight. Its settings-card
hooks act only on its own Options page, so several mods can be installed together without any
conflicting replacements. A mod with native hooks works only on the exact SDK build it was made for.

## Building

```sh
cargo sunrise-mod           # fetches the pinned SDK into .sunrise/ (gitignored)
cargo sunrise-mod check     # type-checks against the pinned SDK
cargo test --lib            # unit tests, no game needed
cargo sunrise-mod build     # target/sunrise-mod/statsplus.smod
```

To move to a newer SDK, run `cargo sunrise-mod update --sunrise <build>` (it rewrites the
`sunrise` and `mixins` lines in `sunrise-mod.toml`), then check and rebuild.

Install the `.smod` with sunrise-launcher (**Add from file**), or point **Add from GitHub** at
this repository. Native mods run with full access to your computer; read the source first.

## License

MIT © 2026 Zekrenlight. See [LICENSE](LICENSE).
