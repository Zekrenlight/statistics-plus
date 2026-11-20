# Statistics+ — feature specification

This document describes intended in-game behavior only. It is not source code, and it does not name other mods’ types, files, setting keys, or screen layout numbers.

Statistics+ is an optional statistics browser for Sunrise (Minecraft Java, game 26.3). When it is enabled, the pause-menu Statistics button opens this implementation’s screen. The stock Statistics widgets are not drawn: the world stays blurred and dimmed, and only the Statistics+ card appears. The screen shows the statistics the game already holds, lets the player search and sort them, and can write or read a local JSON snapshot. Everything below is the intended behavior of this implementation.

## Opening and closing

- On the pause menu, a left click on the Statistics button (the same place the stock button occupies, including when an extra server-links, additions, or splitscreen row shifts the grid) claims the upcoming statistics screen and lets the game open it. The stock Statistics draw is cancelled: the usual world blur and in-world menu dim remain, then Statistics+ paints its card and consumes input so leftover stock widgets cannot appear or receive clicks.
- There is no keybind. `/statsplus panel` opens the card, or closes it if it is already open.
- Chat commands start with `/statsplus`. Useful subcommands are `panel`, `save`, `load [name]`, `empty`, and `help`. Unknown input prints a short help blurb.
- **Done** on the card, or Escape, closes the card on the next frame (the same hook Sunrise runs every redraw, including while an unpublished singleplayer world is paused). The ordinary client tick hook does not run in that paused state, so close must not wait for a tick. Closing sets the game’s screen to the pause menu if the player arrived from there, or to the world with mouse capture restored otherwise. This does not use the stock Statistics Done widget, which is not drawn. If the claim is still set after the game has left the Statistics screen, it is dropped.
- When the mod is off, Sunrise does not build it in, so the game’s own Statistics screen is left alone.

## Requesting and caching values

- Opening the card asks the game to load local statistics when that is appropriate, and, when a remote session is connected, sends the ordinary client command that requests a statistics update (action 1). It does not invent a private network protocol.
- Painted rows follow a cache copied from the game’s current statistic map (type → key → integer). The cache is refreshed while the card is open, so new values appear live.
- A “Session” row on the General view counts time since the world or server session started on this client. That row uses a Statistics+ id and is not written into snapshots.
- Statistics+ never contacts an author site, news feed, credits page, or PSA. It never reads or writes a machine identifier, a hardware identifier, or a hash of either.

## Views

The card has seven tabs:

- **General** — custom statistics the game already tracks (time played, distances walked, and the rest of the usual custom list), formatted by the public custom-stat kinds (plain count, time, distance, or tenths). One live Session row sits with them.
- **Items** — one row per known item, with used, crafted, broken, picked up, and dropped counts. The displayed total is the sum of those.
- **Blocks** — one row per known block, with mined and used counts.
- **Mobs** — one row per known living entity, with killed and killed-by counts, drawn as a short card. When a matching spawn egg exists, that item is drawn as the icon; otherwise the card shows a letter tile. This version does not render a three-dimensional mob model. A toolbar chip cycles **All**, **Hostile**, and **Passive**. Hostile is the game’s `monster` category; Passive is every other category (`creature`, `ambient`, water creatures, `axolotls`, `misc`). There is no separate Neutral filter, because the game does not expose a Neutral category.
- **Food** — rows for edible items and how they were used.
- **Files** — JSON snapshots already stored in the mod data directory’s `snapshots` folder (`sunrise-mods/.sunrise-mod-data/statsplus/snapshots` relative to the Sunrise game folder). The list is rebuilt when the tab is opened and after a save or delete: each readable file is parsed, unreadable files are skipped with a status error, and the remaining rows are sorted newest first. Each row shows the snapshot name (a local civil date-time such as 2026-11-12 17:04), that date, and a short summary (play time when present). An empty-state message is shown when no snapshots are stored. This is not an operating-system file picker. On this tab the Filled, Sort, and Group chips are hidden; the search field filters snapshot names and the Save chip remains.

## Search, sort, group, and empty rows

- The search field matches a row when the query is a case-insensitive substring of the row’s id or of its display label. An empty query shows every row that passes the other filters.
- **Filled** hides rows whose total is zero. **All** shows empty rows as well (known items, blocks, mobs, or custom entries with no count yet). The default is Filled.
- Sort cycles **Listed** (the game’s customary order for the view), **Name** (display label, then id), and **Value** (highest total first, then name).
- Group cycles the options that are meaningful on the current tab. On vanilla Minecraft that is **All** (a single list) and, on Mobs, **Kind** (a coarse category). **Source** (by namespace; `minecraft` is labelled Minecraft) is offered automatically only when the current tab’s identifiers include more than one namespace, so it stays hidden on a vanilla world and appears if Sunrise content mods are present. If a tab has only All as a grouping option, the Group chip is hidden and the toolbar reclaims the space. A stored `group=source` preference falls back to All until a second namespace is present. Group headers are labels only; they are not extra sortable rows.
- The chosen view, sort, group, empty-row switch, and units are stored in `statsplus.cfg` (format 1) under the mod data directory.

## Units

- Distance values are stored in centimetres. The distance chip cycles centimetres, metres (value / 100, one decimal), and kilometres (value / 100000, two decimals).
- Time values are stored in ticks. Twenty ticks are one second. The time chip cycles ticks, seconds, minutes, hours, and days.
- Defaults are metres and hours. Only rows whose formatter is distance or time honor the matching chip; plain counts stay integers, and tenths stay one decimal place.
- The combined distance/time chip is shown only on the General tab. Items, Blocks, and Food hide it and give the leftover space to the search field and the remaining chips. The Mobs tab uses that same toolbar slot for All / Hostile / Passive. The Files tab shows only search and Save.

## List layout and scrolling

- The list is a single vertical viewport inside the card. A row is drawn, clickable, or counted as visible when it intersects that viewport, not only when it is fully inside it.
- The maximum scroll offset is the smallest value that lets the last row sit fully inside the same viewport height used for painting, clicking, and wheel or bar movement.
- A scrollbar thumb tracks that range. Dragging the thumb and using the mouse wheel share the same math. Every row, including the last one, must be reachable.

## Layout

- Each frame, the card size is computed from the current scaled screen size. The panel is clamped so a margin remains on every side of the window.
- The toolbar (search and action buttons) shrinks proportionally, with a minimum width. If the buttons still cannot fit beside the search field, they wrap to a second row and the list starts lower. Tab labels shrink or abbreviate when the card is narrow.
- The list viewport height is the space left in the card after the title, tabs, and toolbar. Painting, hit-testing, and scrolling share that height, and the last row stays reachable.
- Hit-testing uses the same computed rectangles as drawing.
- List rows are clipped to the list viewport when drawn. A click or hover only hits the portion of a row that sits inside that viewport, never a part that has scrolled under the toolbar or past the bottom.
- The mouse wheel moves one row per notch. Scroll offsets are clamped to the range that keeps the last row fully reachable.


## Snapshots

- **Save** (the Files Save chip, or `/statsplus save`) writes a JSON file under `<mod data dir>/snapshots`, which is `sunrise-mods/.sunrise-mod-data/statsplus/snapshots` relative to the Sunrise game folder. The directory is created when missing. The file is this implementation’s format: a root object with `format` (1), `kind` (`statsplus-snapshot`), `name`, and a `stats` object whose keys are statistic types and whose values are objects of id → integer. Zero values are omitted. The Session row is not stored. The snapshot always captures the live game statistics, even if a previous snapshot is being viewed. A name collision in the same minute adds a numeric suffix. The panel shows a status line such as “Saved 2026-11-12 17:04”, or the error if the write fails.
- File names are derived from the snapshot name, which is a local civil date-time (`YYYY-MM-DD HH:MM`). They do not include a machine or hardware identifier.
- **Load** (a Files row or its Load button, or `/statsplus load [name]`) reads a file in that format. The name may be the display stamp, the file name, or omitted to mean the newest snapshot. A document that has a `stats` object but no `kind` field is also accepted, so a plain statistics map can be imported. The loaded map replaces the on-screen cache as a read-only view: live game statistics are not written, and the other tabs show the snapshot until the player clicks **Live** on the “Viewing snapshot X — Live” banner (or loads another snapshot). The cache is not overwritten by the ordinary live refresh while a snapshot is being viewed.
- **Delete** on a Files row must be clicked a second time within 3 seconds to confirm. Deleting the snapshot currently being viewed returns to live statistics.
- Snapshots never leave the local data directory through this mod. There is no upload, no author endpoint, and no background network call.

## Out of scope for 1.0.0

- Advancement or goal tracking
- A `/stats` command (the command here is `/statsplus`)
- Three-dimensional mob portraits
- An operating-system file picker
