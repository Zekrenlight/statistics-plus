// SPDX-License-Identifier: MIT
// Statistics+ tabbed card: search, filters, scrollable lists, item icons.

use super::statsplus as hook;
use crate::game::Game;
use mod_statsplus::logic as rules;
use sunrise_client::hud::HudBatch;
use sunrise_client::items::ItemStack;
use mod_api::ScreenView;
use rules::catalog::{spawn_egg_id, FormatKind, View};
use rules::filters::{
    apply_groups, general_rows, grouped_indices, sort_rows, subject_rows, GroupBy, Row, SortBy,
    SubjectKind,
};
use rules::listing::{self, BAR_GAP, BAR_W, FILES_ROW, MOB_ROW, ROW};
use rules::snapshot::SnapshotInfo;
use rules::layout::ToolSet;
use rules::units::{format_custom, DistUnit, TimeUnit};

const CARD: u32 = 0xFF0F1118;
const TAB: u32 = 0xFF161A22;
const CHIP: u32 = 0xFF1E2330;
const TEXT: u32 = 0xFFE8DCC8;
const DIM: u32 = 0xFF8B93A7;
const SEARCH: u32 = 0xFF0A0C11;
const HEAD: u32 = 0xFF161A22;

/// The live accent (`accent` setting, `0xRRGGBB`; Sunrise orange by default), set each paint.
static ACCENT_RGB: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0xF9801D);
/// Takes the accent colour the card draws with (`0xRRGGBB`).
pub fn set_accent(rgb: u32) {
    ACCENT_RGB.store(rgb & 0xFF_FFFF, std::sync::atomic::Ordering::Relaxed);
}
/// The accent, opaque: titles, values, the 2 px top edge.
fn accent() -> u32 {
    0xFF00_0000 | ACCENT_RGB.load(std::sync::atomic::Ordering::Relaxed)
}
/// The card's top edge: the accent.
fn edge() -> u32 {
    accent()
}
/// The selected tab / On chip: the accent darkened into the card.
fn tab_on() -> u32 {
    let a = accent();
    let ch = |s: u32| {
        let x = ((a >> s) & 0xFF) as f32;
        let b = ((CARD >> s) & 0xFF) as f32;
        ((x * 0.42 + b * 0.58).round() as u32).min(255) << s
    };
    0xFF00_0000 | ch(16) | ch(8) | ch(0)
}

pub fn ours(game: &Game<'_>, screen: &ScreenView<'_>) -> bool {
    hook::claimed() && screen.id == "stats"
}

fn hit(p: (f32, f32), r: [f32; 4]) -> bool {
    p.0 >= r[0] && p.0 < r[0] + r[2] && p.1 >= r[1] && p.1 < r[1] + r[3]
}

fn clip_to(batch: &mut HudBatch, start: usize, r: [f32; 4]) {
    if start >= batch.vertices.len() {
        return;
    }
    batch.clip_since(start, r[0], r[1], r[0] + r[2].max(0.0), r[1] + r[3].max(0.0));
}

fn source_available(game: &Game<'_>, view: View) -> bool {
    match view {
        View::Files => false,
        View::General => {
            let mut ids: Vec<String> = rules::CUSTOM_STATS
                .iter()
                .map(|(id, _)| (*id).to_string())
                .collect();
            hook::revise(|s| ids.extend(s.cache.keys_of("minecraft:custom")));
            rules::filters::source_grouping_available(ids.iter().map(|s| s.as_str()))
        }
        View::Items => {
            let ids = hook::item_ids(game);
            rules::filters::source_grouping_available(ids.iter().map(|s| s.as_str()))
        }
        View::Blocks => {
            let ids = hook::block_ids(game);
            rules::filters::source_grouping_available(ids.iter().map(|s| s.as_str()))
        }
        View::Food => {
            let ids = hook::food_ids(game);
            rules::filters::source_grouping_available(ids.iter().map(|s| s.as_str()))
        }
        View::Mobs => {
            let (ids, _) = hook::mob_ids(game);
            rules::filters::source_grouping_available(ids.iter().map(|s| s.as_str()))
        }
    }
}

fn frame(game: &Game<'_>, screen: &ScreenView<'_>) -> rules::layout::Layout {
    let (view, viewing) = hook::revise(|s| (View::from_u8(s.tab), s.viewing.is_some()));
    let show_group = rules::filters::group_chip_visible(view, source_available(game, view));
    rules::layout::compute_ex(
        screen.rect[2],
        screen.rect[3],
        ToolSet::for_view(view),
        viewing,
        show_group,
    )
}

fn fill(game: &Game<'_>, batch: &mut HudBatch, r: [f32; 4], c: u32) {
    game.native_fill(batch, r, c);
}

fn ink(game: &Game<'_>, batch: &mut HudBatch, text: &str, pos: [f32; 2], c: u32) {
    game.native_text(batch, text, pos, c, true);
}

fn ink_center(game: &Game<'_>, batch: &mut HudBatch, text: &str, cx: f32, y: f32, c: u32) {
    let w = game.native_text_width(batch, text);
    ink(game, batch, text, [cx - w * 0.5, y], c);
}

fn chip(game: &Game<'_>, batch: &mut HudBatch, r: [f32; 4], label: &str, on: bool, hover: bool, color: u32) {
    let bg = if on {
        color
    } else if hover {
        0xFF2A3142
    } else {
        CHIP
    };
    fill(game, batch, r, bg);
    let label_at = batch.vertices.len();
    ink_center(game, batch, label, r[0] + r[2] * 0.5, r[1] + 3.0, TEXT);
    clip_to(batch, label_at, r);
}

#[derive(Clone)]
enum Line {
    Head(String),
    Row(Row),
    File(SnapshotInfo),
}

fn row_h(view: View) -> f32 {
    if view == View::Mobs {
        MOB_ROW
    } else if view == View::Files {
        FILES_ROW
    } else {
        ROW
    }
}

fn current_lines(game: &Game<'_>) -> (View, Vec<Line>) {
    let (view, query, show_empty, sort, group, mobs, cache) = hook::revise(|s| {
        (
            View::from_u8(s.tab),
            s.query.clone(),
            s.prefs.show_empty,
            s.prefs.sort,
            s.prefs.group,
            s.prefs.mobs,
            s.cache.clone(),
        )
    });
    if view == View::Files {
        let files = hook::revise(|s| s.files.clone());
        let mut lines = Vec::new();
        let q = query.to_ascii_lowercase();
        for info in files {
            let hay = format!("{} {} {} {}", info.name, info.date, info.file_name, info.summary)
                .to_ascii_lowercase();
            if q.is_empty() || hay.contains(&q) {
                lines.push(Line::File(info));
            }
        }
        return (view, lines);
    }
    let mut rows = match view {
        View::General => general_rows(&cache, &query, show_empty, Some(hook::session_ticks())),
        View::Items => {
            let ids = hook::item_ids(game);
            subject_rows(&ids, &cache, SubjectKind::Item, &query, show_empty, &Default::default())
        }
        View::Blocks => {
            let ids = hook::block_ids(game);
            subject_rows(&ids, &cache, SubjectKind::Block, &query, show_empty, &Default::default())
        }
        View::Food => {
            let ids = hook::food_ids(game);
            subject_rows(&ids, &cache, SubjectKind::Item, &query, show_empty, &Default::default())
        }
        View::Mobs => {
            let (ids, cats) = hook::mob_ids(game);
            let rows = subject_rows(&ids, &cache, SubjectKind::Mob, &query, show_empty, &cats);
            rules::filters::filter_mob_rows(rows, mobs)
        }
        View::Files => unreachable!(),
    };
    for row in rows.iter_mut() {
        row.label = hook::label(game, &row.id, row.kind);
    }
    sort_rows(&mut rows, sort, view);
    let group = rules::filters::clamp_group(group, view, source_available(game, view));
    apply_groups(&mut rows, group);
    let mut lines = Vec::new();
    for (head, idxs) in grouped_indices(&rows, group) {
        if let Some(title) = head {
            lines.push(Line::Head(title));
        }
        for i in idxs {
            lines.push(Line::Row(rows[i].clone()));
        }
    }
    (view, lines)
}

fn bar_track(list: [f32; 4]) -> [f32; 4] {
    [list[0] + list[2] + BAR_GAP, list[1], BAR_W, list[3]]
}

fn paint_bar(game: &Game<'_>, batch: &mut HudBatch, list: [f32; 4], scroll: f32, total: usize, row: f32, hover: bool) {
    let track = bar_track(list);
    if track[3] <= 0.0 {
        return;
    }
    fill(game, batch, track, SEARCH);
    let Some(thumb) = listing::thumb(scroll, total, list[3], track[3], row) else {
        return;
    };
    fill(
        game,
        batch,
        [track[0], track[1] + thumb.y, track[2], thumb.h],
        if hover { accent() } else { tab_on() },
    );
}

pub fn paint(game: &Game<'_>, batch: &mut HudBatch, screen: &ScreenView<'_>) {
    set_accent(crate::game::statsplus_card::accent(game));
    if !ours(game, screen) {
        return;
    }
    hook::ensure_ready(game);
    hook::refresh_stats(game);
    let ui = frame(game, screen);
    fill(game, batch, [ui.x, ui.y, ui.w, ui.h], CARD);
    fill(game, batch, [ui.x, ui.y, ui.w, 2.0], edge());
    let title_clip = [
        ui.x + 12.0,
        ui.y,
        (ui.done[0] - 4.0 - (ui.x + 12.0)).max(0.0),
        26.0,
    ];
    let title_at = batch.vertices.len();
    ink(game, batch, "Statistics+", [ui.x + 12.0, ui.y + 8.0], accent());
    clip_to(batch, title_at, title_clip);
    let p = (screen.mouse[0], screen.mouse[1]);
    chip(game, batch, ui.done, "Done", hit(p, ui.done), hit(p, ui.done), edge());
    let tab = hook::revise(|s| s.tab);
    for (i, view) in View::ALL.iter().enumerate() {
        let on = tab == view.as_u8();
        fill(game, batch, ui.views[i], if on { tab_on() } else { TAB });
        let label_at = batch.vertices.len();
        ink_center(
            game,
            batch,
            ui.tab_caption(*view),
            ui.views[i][0] + ui.views[i][2] * 0.5,
            ui.views[i][1] + 4.0,
            if on { TEXT } else { DIM },
        );
        clip_to(batch, label_at, ui.views[i]);
    }
    paint_body(game, batch, &ui, p);
}

fn paint_body(game: &Game<'_>, batch: &mut HudBatch, ui: &rules::layout::Layout, p: (f32, f32)) {
    let (query, focus, show_empty, sort, group, dist, time, note, view, mobs) = hook::revise(|s| {
        (
            s.query.clone(),
            s.query_focus,
            s.prefs.show_empty,
            s.prefs.sort,
            s.prefs.group,
            s.prefs.dist,
            s.prefs.time,
            s.note.clone(),
            View::from_u8(s.tab),
            s.prefs.mobs,
        )
    });
    fill(game, batch, ui.search, SEARCH);
    let hint = if query.is_empty() {
        if focus {
            "_"
        } else if view == View::Files {
            if ui.search[2] < 120.0 { "Search..." } else { "Search snapshots..." }
        } else {
            if ui.search[2] < 120.0 { "Search..." } else { "Search statistics..." }
        }
    } else if focus {
        let mut t = query.clone();
        t.push('_');
        // boxed below
        hook::revise(|s| s.query.clone());
        ""
    } else {
        ""
    };
    let shown = if query.is_empty() {
        hint.to_string()
    } else if focus {
        format!("{query}_")
    } else {
        query.clone()
    };
    let search_at = batch.vertices.len();
    ink(
        game,
        batch,
        &shown,
        [ui.search[0] + 4.0, ui.search[1] + 3.0],
        if query.is_empty() && !focus { DIM } else { TEXT },
    );
    clip_to(batch, search_at, ui.search);
    if view != View::Files {
        let empty_l = if show_empty { "All" } else { "Filled" };
        chip(game, batch, ui.empty, empty_l, show_empty, hit(p, ui.empty), tab_on());
        chip(
            game,
            batch,
            ui.sort,
            &if ui.compact_tools { sort.label().to_string() } else { format!("Sort {}", sort.label()) },
            false,
            hit(p, ui.sort),
            tab_on(),
        );
        if ui.group[2] > 0.0 && ui.group[3] > 0.0 {
            let group = rules::filters::clamp_group(group, view, source_available(game, view));
            chip(
                game,
                batch,
                ui.group,
                &if ui.compact_tools { group.label().to_string() } else { format!("Group {}", group.label()) },
                group != GroupBy::All,
                hit(p, ui.group),
                tab_on(),
            );
        }
    }
    if ui.units[2] > 0.0 && ui.units[3] > 0.0 {
        let unit_l = if view == View::Mobs {
            if ui.compact_tools {
                mobs.label().to_string()
            } else {
                format!("Mobs {}", mobs.label())
            }
        } else if view == View::Files {
            "Save".to_string()
        } else {
            format!("{} / {}", dist.label(), time.label())
        };
        chip(
            game,
            batch,
            ui.units,
            &unit_l,
            view == View::Files || (view == View::Mobs && mobs != rules::filters::MobFilter::All),
            hit(p, ui.units),
            tab_on(),
        );
    }

    let viewing_name = hook::revise(|s| s.viewing.clone());
    if ui.banner[3] > 0.0 {
        if let Some(name) = viewing_name.as_ref() {
            fill(game, batch, ui.banner, HEAD);
            let label = format!("Viewing snapshot {name}");
            let text_r = [
                ui.banner[0],
                ui.banner[1],
                (ui.live[0] - ui.banner[0] - 4.0).max(1.0),
                ui.banner[3],
            ];
            let at = batch.vertices.len();
            ink(game, batch, &label, [ui.banner[0] + 6.0, ui.banner[1] + 4.0], accent());
            clip_to(batch, at, text_r);
            chip(game, batch, ui.live, "Live", hit(p, ui.live), hit(p, ui.live), tab_on());
        }
    }

    let (view, lines) = current_lines(game);
    let row = row_h(view);
    let list = ui.list;
    let total = lines.len();
    let scroll = hook::revise(|s| listing::clamp_scroll(s.scroll, total, list[3], row));
    hook::revise(|s| s.scroll = scroll);
    let (start, end) = listing::shown_rows(scroll, total, list[3], row);
    let list_at = batch.vertices.len();
    for i in start..end {
        let Some(vis) = listing::visible_row_rect(list, i, scroll, row) else {
            continue;
        };
        let y = list[1] + listing::row_top_sized(i, scroll, row);
        let r = [list[0], y, list[2], row - 2.0];
        let over = hit(p, vis);
        match &lines[i] {
            Line::Head(title) => {
                fill(game, batch, r, HEAD);
                ink(game, batch, title, [r[0] + 6.0, r[1] + 5.0], accent());
            }
            Line::Row(row_data) => paint_row(game, batch, r, row_data, view, dist, time, over),
            Line::File(info) => paint_file_row(game, batch, r, info, over),
        }
    }
    if total == 0 {
        let msg = if view == View::Files {
            if query.is_empty() {
                "No snapshots yet. Click Save to keep a local copy."
            } else {
                "No snapshots match the search."
            }
        } else {
            "No statistics to show yet."
        };
        ink(game, batch, msg, [list[0] + 8.0, list[1] + 12.0], DIM);
    }
    clip_to(batch, list_at, list);
    paint_bar(game, batch, list, scroll, total, row, bar_hover(p, list, scroll, total, row));
    if !note.is_empty() {
        let note_at = batch.vertices.len();
        let note_r = if ui.status[3] > 0.0 {
            ui.status
        } else {
            [ui.body[0], ui.y + ui.h - 16.0, ui.body[2], 16.0]
        };
        let err = hook::revise(|s| s.note_error);
        let color = if err { 0xFFE08A8A } else { accent() };
        ink(game, batch, &note, [note_r[0], note_r[1] + 1.0], color);
        clip_to(batch, note_at, note_r);
    }
}

fn paint_file_row(
    game: &Game<'_>,
    batch: &mut HudBatch,
    r: [f32; 4],
    info: &SnapshotInfo,
    hover: bool,
) {
    fill(game, batch, r, if hover { 0xFF2A3142 } else { 0xFF1E2330 });
    let (load_r, del_r) = rules::snapshot::files_row_buttons(r);
    let text_w = (load_r[0] - r[0] - 8.0).max(1.0);
    let text_clip = [r[0], r[1], text_w, r[3]];
    let at = batch.vertices.len();
    ink(game, batch, &info.name, [r[0] + 6.0, r[1] + 3.0], TEXT);
    ink(game, batch, &info.summary, [r[0] + 6.0, r[1] + 16.0], DIM);
    clip_to(batch, at, text_clip);
    let confirm = hook::confirming_delete(&info.file_name);
    chip(game, batch, load_r, "Load", false, false, tab_on());
    chip(
        game,
        batch,
        del_r,
        if confirm { "Confirm?" } else { "Delete" },
        confirm,
        false,
        if confirm { 0xFF8A3A3A } else { CHIP },
    );
    let _ = hover;
}

fn paint_row(
    game: &Game<'_>,
    batch: &mut HudBatch,
    r: [f32; 4],
    row: &Row,
    view: View,
    dist: DistUnit,
    time: TimeUnit,
    hover: bool,
) {
    fill(game, batch, r, if hover { 0xFF2A3142 } else { 0xFF1E2330 });
    let mut text_x = r[0] + 6.0;
    if matches!(view, View::Items | View::Blocks | View::Food) {
        let stack = ItemStack::new(&row.id, 1);
        if game.res.items().get(&row.id).is_some() {
            game.native_gui_draw_item_with(batch, &stack, [r[0] + 2.0, r[1] + 2.0], None);
            text_x = r[0] + 22.0;
        }
    } else if view == View::Mobs {
        let egg = spawn_egg_id(&row.id);
        if game.res.items().get(&egg).is_some() {
            let stack = ItemStack::new(&egg, 1);
            game.native_gui_draw_item_with(batch, &stack, [r[0] + 2.0, r[1] + 4.0], None);
            text_x = r[0] + 22.0;
        }
    }
    ink(game, batch, &row.label, [text_x, r[1] + 5.0], TEXT);
    let value = if row.id == "statsplus:session" || view == View::General {
        let kind = if row.id == "statsplus:session" {
            FormatKind::Time
        } else {
            rules::custom_format(&row.id)
        };
        format_custom(kind, row.total, dist, time)
    } else if view == View::Files {
        "Load".to_string()
    } else if !row.extra.is_empty() {
        row.extra
            .iter()
            .filter(|(_, n)| *n != 0 || hook::revise(|s| s.prefs.show_empty))
            .map(|(k, n)| format!("{k} {n}"))
            .collect::<Vec<_>>()
            .join("   ")
    } else {
        row.total.to_string()
    };
    let vw = game.native_text_width(batch, &value);
    ink(game, batch, &value, [r[0] + r[2] - vw - 8.0, r[1] + 5.0], DIM);
}

fn bar_hover(p: (f32, f32), list: [f32; 4], scroll: f32, total: usize, row: f32) -> bool {
    let track = bar_track(list);
    let Some(thumb) = listing::thumb(scroll, total, list[3], track[3], row) else {
        return hit(p, track);
    };
    hit(p, [track[0], track[1] + thumb.y, track[2], thumb.h])
}

fn click_bar(p: (f32, f32), list: [f32; 4], total: usize, scroll: f32, row: f32) -> bool {
    let track = bar_track(list);
    if !hit(p, track) {
        return false;
    }
    let local = p.1 - track[1];
    hook::revise(|s| {
        let view = list[3];
        if let Some(thumb) = listing::thumb(scroll, total, view, track[3], row) {
            if local >= thumb.y && local < thumb.y + thumb.h {
                s.bar_grab = Some(local - thumb.y);
                return;
            }
        }
        s.scroll = listing::scroll_at_track(local, total, view, track[3], row);
        if let Some(thumb) = listing::thumb(s.scroll, total, view, track[3], row) {
            s.bar_grab = Some((local - thumb.y).clamp(0.0, thumb.h));
        }
    });
    true
}

pub fn click(game: &Game<'_>, screen: &ScreenView<'_>, button: i32) -> bool {
    if hook::intercept_pause(game, screen) {
        return true;
    }
    if !ours(game, screen) || button != 0 {
        return false;
    }
    let ui = frame(game, screen);
    let p = (screen.mouse[0], screen.mouse[1]);
    if hit(p, ui.done) {
        hook::revise(|s| s.want_close = true);
        return true;
    }
    for (i, view) in View::ALL.iter().enumerate() {
        if hit(p, ui.views[i]) {
            hook::revise(|s| {
                let leaving_or_entering_files =
                    View::from_u8(s.tab) == View::Files || *view == View::Files;
                if leaving_or_entering_files {
                    s.query.clear();
                }
                s.tab = view.as_u8();
                s.prefs.view = *view;
                s.scroll = 0.0;
                s.query_focus = false;
                if *view == View::Files {
                    s.confirm_delete = None;
                }
            });
            hook::write_prefs(game);
            if *view == View::Files {
                hook::refresh_files(game);
            }
            return true;
        }
    }
    if hit(p, ui.search) {
        hook::revise(|s| s.query_focus = true);
        return true;
    }
    hook::revise(|s| s.query_focus = false);
    if ui.live[3] > 0.0 && hit(p, ui.live) {
        hook::return_to_live(game);
        return true;
    }
    if hit(p, ui.empty) && hook::revise(|s| View::from_u8(s.tab) != View::Files) {
        hook::revise(|s| {
            s.prefs.show_empty = !s.prefs.show_empty;
            s.scroll = 0.0;
        });
        hook::write_prefs(game);
        return true;
    }
    if hit(p, ui.sort) && hook::revise(|s| View::from_u8(s.tab) != View::Files) {
        hook::revise(|s| {
            s.prefs.sort = s.prefs.sort.next();
            s.scroll = 0.0;
        });
        hook::write_prefs(game);
        return true;
    }
    if hit(p, ui.group) && ui.group[2] > 0.0 && hook::revise(|s| View::from_u8(s.tab) != View::Files) {
        let view = hook::revise(|s| View::from_u8(s.tab));
        let source = source_available(game, view);
        hook::revise(|s| {
            s.prefs.group = s.prefs.group.next(view, source);
            s.scroll = 0.0;
        });
        hook::write_prefs(game);
        return true;
    }
    if hit(p, ui.units) {
        let view = hook::revise(|s| View::from_u8(s.tab));
        if view == View::Mobs {
            hook::revise(|s| {
                s.prefs.mobs = s.prefs.mobs.next();
                s.scroll = 0.0;
            });
            hook::write_prefs(game);
        } else if view == View::Files {
            match hook::save_snapshot(game) {
                Ok(_) => {}
                Err(e) => hook::revise(|s| s.note = format!("Save failed: {e}")),
            }
        } else if view == View::General {
            hook::revise(|s| {
                // Cycle distance on left half, time on right half, or both if general click.
                if p.0 < ui.units[0] + ui.units[2] * 0.5 {
                    s.prefs.dist = s.prefs.dist.next();
                } else {
                    s.prefs.time = s.prefs.time.next();
                }
            });
            hook::write_prefs(game);
        }
        return true;
    }
    let (view, lines) = current_lines(game);
    let row = row_h(view);
    let list = ui.list;
    let total = lines.len();
    let scroll = hook::revise(|s| listing::clamp_scroll(s.scroll, total, list[3], row));
    if click_bar(p, list, total, scroll, row) {
        return true;
    }
    if hit(p, list) && view == View::Files {
        let (start, end) = listing::shown_rows(scroll, total, list[3], row);
        for i in start..end {
            let Some(vis) = listing::visible_row_rect(list, i, scroll, row) else {
                continue;
            };
            if !hit(p, vis) {
                continue;
            }
            if let Line::File(info) = &lines[i] {
                let y = list[1] + listing::row_top_sized(i, scroll, row);
                let r = [list[0], y, list[2], row - 2.0];
                let (load_r, del_r) = rules::snapshot::files_row_buttons(r);
                if hit(p, del_r) {
                    hook::request_delete(&info.file_name);
                    return true;
                }
                let _ = hook::load_named_snapshot(game, &info.file_name);
                if hook::is_viewing() {
                    hook::revise(|s| {
                        s.tab = View::General.as_u8();
                        s.scroll = 0.0;
                    });
                }
            }
            return true;
        }
    }
    true
}

pub fn wheel(game: &Game<'_>, screen: &ScreenView<'_>, scroll: [f64; 2]) -> bool {
    if !ours(game, screen) {
        return false;
    }
    let dy = if scroll[1].abs() > scroll[0].abs() {
        scroll[1]
    } else {
        scroll[0]
    };
    if dy == 0.0 {
        return true;
    }
    let ui = frame(game, screen);
    let (view, lines) = current_lines(game);
    let row = row_h(view);
    let list = ui.list;
    hook::revise(|s| {
        s.scroll = listing::apply_wheel(s.scroll, dy as f32, lines.len(), list[3], row);
    });
    true
}

pub fn drag(game: &Game<'_>, screen: &ScreenView<'_>, button: i32) -> bool {
    if !ours(game, screen) || button != 0 {
        return false;
    }
    let grab = hook::revise(|s| s.bar_grab);
    let Some(off) = grab else {
        return false;
    };
    let ui = frame(game, screen);
    let p = (screen.mouse[0], screen.mouse[1]);
    let (view, lines) = current_lines(game);
    let row = row_h(view);
    let list = ui.list;
    hook::revise(|s| {
        let track = bar_track(list);
        s.scroll = listing::scroll_from_drag(p.1 - track[1], off, lines.len(), list[3], track[3], row);
    });
    true
}

pub fn release(game: &Game<'_>, screen: &ScreenView<'_>, button: i32) -> bool {
    if !ours(game, screen) {
        return false;
    }
    hook::revise(|s| {
        s.bar_grab = None;
    });
    let _ = (game, button);
    true
}

pub fn key(game: &Game<'_>, screen: &ScreenView<'_>, key: &str) -> bool {
    if !ours(game, screen) {
        return false;
    }
    if key == "key.keyboard.escape" {
        hook::revise(|s| s.want_close = true);
        return true;
    }
    let focus = hook::revise(|s| s.query_focus);
    if !focus {
        return true;
    }
    if key == "key.keyboard.backspace" {
        hook::revise(|s| {
            s.query.pop();
            s.scroll = 0.0;
        });
        return true;
    }
    let _ = game;
    true
}

pub fn typed(game: &Game<'_>, screen: &ScreenView<'_>, c: char) -> bool {
    if !ours(game, screen) {
        return false;
    }
    let focus = hook::revise(|s| s.query_focus);
    if !focus {
        return true;
    }
    if !c.is_control() && c != '§' {
        hook::revise(|s| {
            if s.query.len() < 32 {
                s.query.push(c);
                s.scroll = 0.0;
            }
        });
    }
    let _ = game;
    true
}

pub fn close_if_asked(game: &mut Game<'_>) {
    hook::close_if_asked(game);
}
