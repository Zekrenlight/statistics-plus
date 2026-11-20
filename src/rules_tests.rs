// SPDX-License-Identifier: MIT
// Statistics+ logic tests: `cargo test --lib`.

use crate::logic as statsplus_logic;
use statsplus_logic::catalog::{custom_format, label_of, normalize_id, spawn_egg_id, FormatKind, View};
use statsplus_logic::filters::{
    apply_groups, filter_mob_rows, general_rows, grouped_indices, matches_mob_filter,
    clamp_group, group_chip_visible, group_options, matches_query, sort_rows,
    source_grouping_available, subject_rows, GroupBy, MobFilter, SortBy, SubjectKind,
};
use statsplus_logic::listing::{
    apply_wheel, clamp_scroll, last_row_fully_visible, max_scroll, pause_stats_rect, row_box,
    shown_rows, thumb, visible_row_rect, wheel_delta, MOB_ROW, ROW,
};
use statsplus_logic::persist::{dump_prefs, load_prefs, Prefs};
use statsplus_logic::snapshot::{delete_from_dir, dump_snapshot, list_dir, load_from_dir, load_snapshot, save_to_dir, Snapshot, SNAPSHOT_KIND};
use statsplus_logic::units::{format_custom, format_distance, format_time, DistUnit, TimeUnit};
use statsplus_logic::close::{apply_take_over, auto_return_to_pause, decide_close, should_drop_claim, toggle_from_claimed, Destination, Toggle};
use statsplus_logic::{merge_ids, snapshot_file_stem, StatMap};
use std::collections::HashMap;

#[test]
fn ids_normalize_and_labels_read_cleanly() {
    assert_eq!(normalize_id("Diamond Sword"), "minecraft:diamond_sword");
    assert_eq!(normalize_id("minecraft:COD"), "minecraft:cod");
    assert_eq!(label_of("minecraft:nautilus_shell"), "Nautilus Shell");
    assert_eq!(spawn_egg_id("minecraft:zombie"), "minecraft:zombie_spawn_egg");
    assert_eq!(custom_format("play_time"), FormatKind::Time);
    assert_eq!(custom_format("damage_dealt"), FormatKind::Tenths);
    assert_eq!(custom_format("mod:mystery"), FormatKind::Default);
}

#[test]
fn distance_and_time_units_convert() {
    assert_eq!(format_distance(100, DistUnit::Cm), "100 cm");
    assert_eq!(format_distance(100, DistUnit::Meters), "1.0 m");
    assert_eq!(format_distance(100_000, DistUnit::Km), "1.00 km");
    assert_eq!(format_time(20, TimeUnit::Ticks), "20 ticks");
    assert_eq!(format_time(20, TimeUnit::Seconds), "1.0 s");
    assert_eq!(format_time(1200, TimeUnit::Minutes), "1.00 min");
    assert_eq!(format_time(72_000, TimeUnit::Hours), "1.00 hr");
    assert_eq!(format_time(1_728_000, TimeUnit::Days), "1.00 d");
    assert_eq!(
        format_custom(FormatKind::Tenths, 15, DistUnit::Cm, TimeUnit::Seconds),
        "1.5"
    );
    assert!(DistUnit::Cm.next() == DistUnit::Meters);
    assert!(TimeUnit::Days.next() == TimeUnit::Ticks);
}

#[test]
fn search_matches_id_or_label() {
    assert!(matches_query("minecraft:diamond", "Diamond", "dia"));
    assert!(matches_query("minecraft:oak_log", "Oak Log", "oak log"));
    assert!(matches_query("minecraft:zombie", "Zombie", "minecraft:zom"));
    assert!(!matches_query("minecraft:diamond", "Diamond", "emerald"));
    assert!(matches_query("minecraft:diamond", "Diamond", "  "));
}

#[test]
fn empty_filter_hides_zero_totals() {
    let mut stats = StatMap::default();
    stats.set("minecraft:custom", "minecraft:jump", 4);
    stats.set("minecraft:custom", "minecraft:deaths", 0);
    let shown = general_rows(&stats, "", false, None);
    assert!(shown.iter().any(|r| r.id == "minecraft:jump" && r.total == 4));
    assert!(!shown.iter().any(|r| r.id == "minecraft:deaths"));
    let all = general_rows(&stats, "", true, None);
    assert!(all.iter().any(|r| r.id == "minecraft:deaths" && r.total == 0));
}

#[test]
fn session_row_is_independent_and_searchable() {
    let stats = StatMap::default();
    let rows = general_rows(&stats, "session", true, Some(40));
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, "statsplus:session");
    assert_eq!(rows[0].total, 40);
}

#[test]
fn sort_by_name_and_value() {
    let mut stats = StatMap::default();
    stats.set("minecraft:used", "minecraft:dirt", 2);
    stats.set("minecraft:used", "minecraft:apple", 9);
    stats.set("minecraft:used", "minecraft:stone", 2);
    let ids = vec![
        "minecraft:dirt".into(),
        "minecraft:apple".into(),
        "minecraft:stone".into(),
    ];
    let mut rows = subject_rows(&ids, &stats, SubjectKind::Item, "", true, &HashMap::new());
    sort_rows(&mut rows, SortBy::Name, View::Items);
    let names: Vec<_> = rows.iter().map(|r| r.label.as_str()).collect();
    assert_eq!(names, ["Apple", "Dirt", "Stone"]);
    sort_rows(&mut rows, SortBy::Value, View::Items);
    assert_eq!(rows[0].id, "minecraft:apple");
    assert_eq!(rows[1].total, 2);
}

#[test]
fn general_vanilla_order_puts_play_time_before_jump() {
    let mut stats = StatMap::default();
    stats.set("minecraft:custom", "minecraft:jump", 1);
    stats.set("minecraft:custom", "minecraft:play_time", 1);
    let mut rows = general_rows(&stats, "", false, None);
    sort_rows(&mut rows, SortBy::Vanilla, View::General);
    let ids: Vec<_> = rows.iter().map(|r| r.id.as_str()).collect();
    let play = ids.iter().position(|i| *i == "minecraft:play_time").unwrap();
    let jump = ids.iter().position(|i| *i == "minecraft:jump").unwrap();
    assert!(play < jump);
}

#[test]
fn grouping_by_namespace_and_category() {
    let mut stats = StatMap::default();
    stats.set("minecraft:killed", "minecraft:zombie", 3);
    stats.set("minecraft:killed", "modpack:wisp", 1);
    let ids = vec!["minecraft:zombie".into(), "modpack:wisp".into()];
    let mut cats = HashMap::new();
    cats.insert("minecraft:zombie".into(), "monster".into());
    cats.insert("modpack:wisp".into(), "creature".into());
    let mut rows = subject_rows(&ids, &stats, SubjectKind::Mob, "", false, &cats);
    apply_groups(&mut rows, GroupBy::Namespace);
    let groups = grouped_indices(&rows, GroupBy::Namespace);
    let labels: Vec<_> = groups.iter().map(|(g, _)| g.clone().unwrap()).collect();
    assert!(labels.contains(&"Minecraft".into()));
    assert!(labels.contains(&"modpack".into()));
    apply_groups(&mut rows, GroupBy::Category);
    let kinds = grouped_indices(&rows, GroupBy::Category);
    let labels: Vec<_> = kinds.iter().map(|(g, _)| g.clone().unwrap()).collect();
    assert!(labels.iter().any(|s| s == "Monster"));
    assert!(labels.iter().any(|s| s == "Creature"));
}

#[test]
fn food_is_a_filtered_subject_list() {
    let mut stats = StatMap::default();
    stats.set("minecraft:used", "minecraft:apple", 2);
    stats.set("minecraft:used", "minecraft:stick", 5);
    let food_ids = vec!["minecraft:apple".into()];
    let food = subject_rows(&food_ids, &stats, SubjectKind::Item, "", false, &HashMap::new());
    assert_eq!(food.len(), 1);
    assert_eq!(food[0].id, "minecraft:apple");
}

#[test]
fn tabs_are_general_items_blocks_mobs_food_files() {
    assert_eq!(
        View::ALL,
        [
            View::General,
            View::Items,
            View::Blocks,
            View::Mobs,
            View::Food,
            View::Files
        ]
    );
    assert_eq!(View::ALL.len(), 6);
    assert_eq!(View::Files.as_u8(), 5);
    assert_eq!(View::from_u8(5), View::Files);
    assert_eq!(View::from_u8(6), View::Files);
}

#[test]
fn mob_category_filter_splits_hostile_and_passive() {
    let mut stats = StatMap::default();
    stats.set("minecraft:killed", "minecraft:zombie", 1);
    stats.set("minecraft:killed", "minecraft:pig", 4);
    stats.set("minecraft:killed", "minecraft:bat", 1);
    stats.set("minecraft:killed", "minecraft:squid", 2);
    let mut cats = HashMap::new();
    cats.insert("minecraft:zombie".into(), "monster".into());
    cats.insert("minecraft:pig".into(), "creature".into());
    cats.insert("minecraft:bat".into(), "ambient".into());
    cats.insert("minecraft:squid".into(), "water_creature".into());
    let ids = vec![
        "minecraft:zombie".into(),
        "minecraft:pig".into(),
        "minecraft:bat".into(),
        "minecraft:squid".into(),
    ];
    let all = subject_rows(&ids, &stats, SubjectKind::Mob, "", false, &cats);
    assert_eq!(all.len(), 4);
    assert!(matches_mob_filter("monster", MobFilter::Hostile));
    assert!(!matches_mob_filter("creature", MobFilter::Hostile));
    assert!(matches_mob_filter("creature", MobFilter::Passive));
    assert!(matches_mob_filter("ambient", MobFilter::Passive));
    assert!(matches_mob_filter("water_creature", MobFilter::Passive));
    assert!(matches_mob_filter("misc", MobFilter::Passive));
    // No separate Neutral bucket: vanilla categories do not provide one.
    let hostile = filter_mob_rows(all.clone(), MobFilter::Hostile);
    assert_eq!(hostile.len(), 1);
    assert_eq!(hostile[0].id, "minecraft:zombie");
    let passive = filter_mob_rows(all.clone(), MobFilter::Passive);
    assert_eq!(passive.len(), 3);
    assert!(!passive.iter().any(|r| r.id.contains("zombie")));
    assert_eq!(filter_mob_rows(all, MobFilter::All).len(), 4);
    assert_eq!(MobFilter::All.next(), MobFilter::Hostile);
    assert_eq!(MobFilter::Hostile.next(), MobFilter::Passive);
    assert_eq!(MobFilter::Passive.next(), MobFilter::All);
}

#[test]
fn snapshot_round_trip_and_vanilla_shaped_load() {
    let mut stats = StatMap::default();
    stats.set("minecraft:custom", "minecraft:play_time", 1234);
    stats.set("minecraft:mined", "minecraft:stone", 9);
    stats.set("minecraft:used", "minecraft:apple", 0);
    let snap = Snapshot::new("day-one", stats.clone());
    let text = dump_snapshot(&snap);
    assert!(text.contains(SNAPSHOT_KIND));
    assert!(text.contains("\"format\": 1"));
    let back = load_snapshot(&text).expect("round trip");
    assert_eq!(back.name, "day-one");
    assert_eq!(back.stats.get("custom", "play_time"), 1234);
    assert_eq!(back.stats.get("minecraft:mined", "stone"), 9);
    assert_eq!(back.stats.get("minecraft:used", "minecraft:apple"), 0);

    let vanilla_like = r#"{
        "stats": {
            "minecraft:custom": { "minecraft:jump": 7 },
            "minecraft:killed": { "minecraft:zombie": 2 }
        }
    }"#;
    let loaded = load_snapshot(vanilla_like).expect("vanilla-shaped stats object");
    assert_eq!(loaded.stats.get("custom", "jump"), 7);
    assert_eq!(loaded.stats.get("killed", "zombie"), 2);
}

#[test]
fn prefs_round_trip() {
    let mut p = Prefs::default();
    p.show_empty = true;
    p.dist = DistUnit::Km;
    p.time = TimeUnit::Minutes;
    p.view = View::Food;
    p.sort = SortBy::Value;
    p.group = GroupBy::Namespace;
    p.mobs = MobFilter::Hostile;
    let text = dump_prefs(&p);
    assert!(text.contains("mobs=hostile"));
    let back = load_prefs(&text);
    assert_eq!(back, p);
    // A previous Hunter tab is opened as Mobs.
    let old = load_prefs("format=1\nview=hunter\n");
    assert_eq!(old.view, View::Mobs);
    assert_eq!(old.mobs, MobFilter::All);
}

#[test]
fn last_row_is_reachable_and_shown() {
    // 21 rows, like a long list whose last entry was previously skipped.
    let total = 21;
    let view_h = 5.0 * ROW - 6.0; // not a whole number of rows
    assert!(max_scroll(total, view_h, ROW) > 0.0);
    assert!(last_row_fully_visible(total, view_h, ROW));
    let scroll = max_scroll(total, view_h, ROW);
    let (start, end) = shown_rows(scroll, total, view_h, ROW);
    assert_eq!(end, total);
    assert!(start < end);
    assert!(end - start >= 4);
    // Mid-scroll still includes a partially visible row at the edge.
    let (s, e) = shown_rows(ROW * 0.5, total, view_h, ROW);
    assert_eq!(s, 0);
    assert!(e > s);
    // Thumb exists only when content overflows.
    assert!(thumb(0.0, total, view_h, 100.0, ROW).is_some());
    assert!(thumb(0.0, 2, 10.0 * ROW, 100.0, ROW).is_none());
    // Scroll clamp keeps a bad offset in range.
    assert_eq!(clamp_scroll(f32::NAN, total, view_h, ROW), 0.0);
    assert_eq!(
        clamp_scroll(10_000.0, total, view_h, ROW),
        max_scroll(total, view_h, ROW)
    );
}

#[test]
fn merge_ids_keeps_registry_then_extras() {
    let registry = vec!["minecraft:dirt".into(), "minecraft:stone".into()];
    let extras = vec!["minecraft:stone".into(), "mod:custom".into()];
    let ids = merge_ids(&registry, extras.into_iter());
    assert_eq!(
        ids,
        vec![
            "minecraft:dirt".to_string(),
            "minecraft:stone".to_string(),
            "mod:custom".to_string()
        ]
    );
}

#[test]
fn snapshot_stem_is_filesystem_safe() {
    assert_eq!(snapshot_file_stem("2026-11-12 13:57:00"), "snapshot-2026-11-12-13-57-00");
}

#[test]
fn pause_stats_button_sits_on_the_vanilla_grid() {
    let r = pause_stats_rect(427.0, 240.0, 0);
    assert_eq!(r[2], 98.0);
    assert_eq!(r[3], 20.0);
    let with_links = pause_stats_rect(427.0, 240.0, 1);
    // Extra rows change the grid origin, so the button moves.
    assert_ne!(r[1], with_links[1]);
}

#[test]
fn pending_close_survives_takeover_and_picks_the_right_screen() {
    // Done/Escape set want_close on an immutable input hook; the next draw
    // must not swallow that flag.
    assert_eq!(apply_take_over(true, true), (true, true));
    assert_eq!(apply_take_over(false, true), (false, true));
    assert_eq!(apply_take_over(false, false), (true, false));
    assert_eq!(apply_take_over(true, false), (true, false));

    assert!(auto_return_to_pause(false, false));
    assert!(!auto_return_to_pause(true, false));
    assert!(auto_return_to_pause(true, true));

    assert_eq!(decide_close(true, true, true), Some(Destination::Pause));
    assert_eq!(decide_close(true, true, false), Some(Destination::Playing));
    assert_eq!(decide_close(true, false, true), None);
    assert_eq!(decide_close(false, true, true), None);

    assert_eq!(toggle_from_claimed(true), Toggle::Close);
    assert_eq!(toggle_from_claimed(false), Toggle::Open);

    // Claim must not linger after the game has already left Statistics.
    assert!(should_drop_claim(true, false));
    assert!(!should_drop_claim(true, true));
    assert!(!should_drop_claim(false, false));
}

#[test]
fn layout_stays_inside_the_panel_at_common_scaled_sizes() {
    use statsplus_logic::layout::{compute, panel_inside_screen, rect_inside, MARGIN};

    const SIZES: &[(f32, f32)] = &[
        (320.0, 240.0),
        (427.0, 240.0),
        (480.0, 270.0),
        (640.0, 360.0),
        (960.0, 540.0),
    ];
    for &(sw, sh) in SIZES {
        let ui = compute(sw, sh);
        assert!(
            panel_inside_screen(ui.panel(), sw, sh),
            "panel {:?} escapes {sw}x{sh}",
            ui.panel()
        );
        let panel = ui.panel();
        for r in ui.hit_rects() {
            assert!(
                rect_inside(r, panel),
                "rect {r:?} outside panel {panel:?} at {sw}x{sh}"
            );
        }
        for r in [ui.search, ui.empty, ui.sort, ui.group, ui.units, ui.list, ui.bar] {
            if r[2] <= 0.0 || r[3] <= 0.0 {
                continue;
            }
            assert!(
                rect_inside(r, ui.body),
                "tool/list {r:?} outside body {:?} at {sw}x{sh}",
                ui.body
            );
        }
        // Shared viewport math still reaches the last row in the leftover list height.
        if ui.list[3] >= ROW {
            let total = 20usize;
            let view_h = ui.list[3];
            let max = max_scroll(total, view_h, ROW);
            assert!(
                last_row_fully_visible(total, view_h, ROW),
                "last row unreachable at {sw}x{sh} view_h={view_h}"
            );
            let (_, end) = shown_rows(max, total, view_h, ROW);
            assert_eq!(end, total);
        }
        assert!(ui.x >= MARGIN - 0.05);
        assert!(ui.y >= MARGIN - 0.05);
        assert!(ui.x + ui.w <= sw - MARGIN + 0.05);
        assert!(ui.y + ui.h <= sh - MARGIN + 0.05);
    }

    let wide = compute(960.0, 540.0);
    assert!((wide.w - 560.0).abs() < 0.05);
    assert!((wide.h - 420.0).abs() < 0.05);
    assert!(!wide.tools_wrapped);
    assert_eq!(wide.tab_compact, 0);
    assert_eq!(wide.tab_caption(View::General), "General");

    let narrow = compute(320.0, 240.0);
    assert!(narrow.w <= 320.0 - MARGIN * 2.0 + 0.05);
    assert!(narrow.h <= 240.0 - MARGIN * 2.0 + 0.05);
    assert!(narrow.tab_compact >= 1);
    assert_ne!(narrow.tab_caption(View::General), "General");

    // Narrower than a single 4-chip toolbar row: buttons wrap, list moves down, still in-bounds.
    use statsplus_logic::layout::{compute_ex, ToolSet};
    let wrap = compute_ex(260.0, 240.0, ToolSet::General, false, true);
    assert!(wrap.tools_wrapped);
    assert!(panel_inside_screen(wrap.panel(), 260.0, 240.0));
    for r in wrap.hit_rects() {
        assert!(rect_inside(r, wrap.panel()), "wrapped rect {r:?} out of {:?}", wrap.panel());
    }
    assert!(wrap.header_h > compute(960.0, 540.0).header_h);
    assert!(wrap.list[1] > wrap.search[1] + wrap.search[3]);
    // Vanilla General hides Group, so the toolbar may stay on one row, but still fits.
    let vanilla = compute(260.0, 240.0);
    assert!(panel_inside_screen(vanilla.panel(), 260.0, 240.0));
    assert!(vanilla.group[2] <= 0.0);
}

#[test]
fn drawn_rows_stay_inside_the_viewport_at_any_scroll() {
    use statsplus_logic::layout::rect_inside;

    let list = [12.0, 40.0, 220.0, 5.0 * ROW - 6.0];
    let total = 21usize;
    let view_h = list[3];
    let max = max_scroll(total, view_h, ROW);
    let offsets = [
        0.0,
        8.0,
        16.0,
        ROW * 0.5,
        ROW,
        ROW + 16.0,
        ROW * 1.5,
        max,
        max + 24.0,
        -4.0,
        f32::NAN,
    ];
    for raw in offsets {
        let scroll = clamp_scroll(raw, total, view_h, ROW);
        let (start, end) = shown_rows(scroll, total, view_h, ROW);
        for i in start..end {
            let unclipped = row_box(list, i, scroll, ROW);
            // A fractional offset may put the unclipped box past the top or bottom.
            if let Some(vis) = visible_row_rect(list, i, scroll, ROW) {
                assert!(
                    rect_inside(vis, list),
                    "visible row {i} {vis:?} escapes {list:?} at scroll={scroll}"
                );
                assert!(vis[3] <= ROW + 0.05);
            } else {
                // No overlap means the unclipped box must sit completely outside.
                let below = unclipped[1] >= list[1] + list[3] - 0.05;
                let above = unclipped[1] + unclipped[3] <= list[1] + 0.05;
                assert!(above || below, "row {i} missing visible rect but overlaps at {scroll}");
            }
        }
        // A point just above the viewport must not hit a visible row.
        let above = (list[0] + 4.0, list[1] - 1.0);
        for i in start..end {
            if let Some(vis) = visible_row_rect(list, i, scroll, ROW) {
                assert!(
                    !(above.0 >= vis[0]
                        && above.0 < vis[0] + vis[2]
                        && above.1 >= vis[1]
                        && above.1 < vis[1] + vis[3]),
                    "click above the list hit row {i} at scroll={scroll}"
                );
            }
        }
    }
    // Last-row reachability is unchanged at a leftover viewport height.
    assert!(last_row_fully_visible(total, view_h, ROW));
}

#[test]
fn wheel_moves_one_row_per_notch() {
    assert_eq!(wheel_delta(1.0, ROW), ROW);
    assert_eq!(wheel_delta(-1.0, MOB_ROW), -MOB_ROW);
    assert_eq!(wheel_delta(0.0, ROW), 0.0);
    assert_eq!(wheel_delta(f32::NAN, ROW), 0.0);

    // The hook reports scroll the way the game does: a notch toward later
    // rows is dy = -1, and the offset is `scroll - dy * row`.
    let total = 20usize;
    let view_h = 5.0 * ROW - 6.0;
    let one = apply_wheel(0.0, -1.0, total, view_h, ROW);
    assert!((one - ROW).abs() < 0.01, "first notch should be one row, got {one}");
    // The new first row sits flush with the viewport top (no overlap above).
    let list = [0.0, 50.0, 200.0, view_h];
    assert!(visible_row_rect(list, 0, one, ROW).is_none());
    let vis = visible_row_rect(list, 1, one, ROW).expect("row 1 visible after one notch");
    assert!((vis[1] - list[1]).abs() < 0.05);
    assert!((vis[3] - ROW).abs() < 0.05);

    // A second notch stays aligned; the old 16px step would not.
    let two = apply_wheel(one, -1.0, total, view_h, ROW);
    assert!((two - 2.0 * ROW).abs() < 0.01);
    let vis = visible_row_rect(list, 2, two, ROW).expect("row 2 visible after two notches");
    assert!((vis[1] - list[1]).abs() < 0.05);

    // Clamped at the ends; last row stays fully reachable.
    let top = apply_wheel(0.0, 1.0, total, view_h, ROW);
    assert_eq!(top, 0.0);
    let end = apply_wheel(10_000.0, -1.0, total, view_h, ROW);
    assert_eq!(end, max_scroll(total, view_h, ROW));
    assert!(last_row_fully_visible(total, view_h, ROW));
}

#[test]
fn snapshot_save_list_load_round_trip_in_a_temp_dir() {
    let dir = temp_snap_dir();
    let mut stats = StatMap::default();
    stats.set("minecraft:custom", "minecraft:play_time", 20 * 60 * 90);
    stats.set("minecraft:mined", "minecraft:stone", 4);
    let info = save_to_dir(&dir, "2026-11-12 17:04", stats.clone()).expect("save");
    assert_eq!(info.name, "2026-11-12 17:04");
    assert!(info.file_name.starts_with("snapshot-2026-11-12-17-04"));
    assert!(info.file_name.ends_with(".json"));
    assert!(info.summary.to_ascii_lowercase().contains("play time"), "summary was {}", info.summary);
    let listed = list_dir(&dir);
    assert!(listed.errors.is_empty(), "unexpected errors: {:?}", listed.errors);
    assert_eq!(listed.infos.len(), 1);
    assert_eq!(listed.infos[0].name, "2026-11-12 17:04");
    let loaded = load_from_dir(&dir, "2026-11-12 17:04").expect("load by display name");
    assert_eq!(loaded.stats.get("custom", "play_time"), 20 * 60 * 90);
    assert_eq!(loaded.stats.get("minecraft:mined", "stone"), 4);
    let loaded2 = load_from_dir(&dir, &info.file_name).expect("load by file name");
    assert_eq!(loaded2.name, "2026-11-12 17:04");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn snapshot_name_collision_gets_a_suffix() {
    let dir = temp_snap_dir();
    let mut stats = StatMap::default();
    stats.set("minecraft:custom", "minecraft:jump", 1);
    let a = save_to_dir(&dir, "2026-11-12 17:04", stats.clone()).expect("first");
    let b = save_to_dir(&dir, "2026-11-12 17:04", stats).expect("second");
    assert_eq!(a.file_name, "snapshot-2026-11-12-17-04.json");
    assert_eq!(b.file_name, "snapshot-2026-11-12-17-04-2.json");
    let listed = list_dir(&dir);
    assert_eq!(listed.infos.len(), 2);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn snapshot_delete_removes_the_file() {
    let dir = temp_snap_dir();
    let mut stats = StatMap::default();
    stats.set("minecraft:custom", "minecraft:jump", 3);
    let info = save_to_dir(&dir, "to-delete", stats).expect("save");
    delete_from_dir(&dir, &info.file_name).expect("delete");
    let listed = list_dir(&dir);
    assert!(listed.infos.is_empty());
    assert!(listed.errors.is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn corrupt_snapshot_is_skipped_with_an_error() {
    let dir = temp_snap_dir();
    let mut stats = StatMap::default();
    stats.set("minecraft:custom", "minecraft:jump", 1);
    save_to_dir(&dir, "good", stats).expect("save");
    std::fs::write(dir.join("broken.json"), "{ this is not a snapshot").unwrap();
    std::fs::write(dir.join("also-bad.json"), r#"{"format":1}"#).unwrap();
    let listed = list_dir(&dir);
    assert_eq!(listed.infos.len(), 1, "good snapshot should remain, got {:?}", listed.infos);
    assert_eq!(listed.infos[0].name, "good");
    assert!(
        listed.errors.iter().any(|e| e.contains("broken.json")),
        "missing broken.json error: {:?}",
        listed.errors
    );
    assert!(
        listed.errors.iter().any(|e| e.contains("also-bad.json")),
        "missing also-bad.json error: {:?}",
        listed.errors
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn files_toolbar_hides_unused_chips_and_reserves_a_viewing_banner() {
    use statsplus_logic::layout::{compute_ex, panel_inside_screen, rect_inside, ToolSet, BANNER_H};

    let files = compute_ex(960.0, 540.0, ToolSet::Files, false, false);
    assert!(files.empty[2] <= 0.0 && files.sort[2] <= 0.0 && files.group[2] <= 0.0);
    assert!(files.units[2] > 0.0 && files.search[2] > 0.0);
    assert!(files.banner[3] <= 0.0);
    assert!(panel_inside_screen(files.panel(), 960.0, 540.0));
    for r in files.hit_rects() {
        assert!(rect_inside(r, files.panel()), "files rect {r:?} outside {:?}", files.panel());
    }

    let viewing = compute_ex(960.0, 540.0, ToolSet::Files, true, false);
    assert!(viewing.banner[3] >= BANNER_H - 0.05);
    assert!(viewing.live[2] > 0.0);
    assert!(viewing.list[1] + 0.01 >= viewing.banner[1] + viewing.banner[3]);
    assert!(viewing.list[1] > files.list[1]);
    assert!(panel_inside_screen(viewing.panel(), 960.0, 540.0));
    for r in viewing.hit_rects() {
        assert!(rect_inside(r, viewing.panel()), "viewing rect {r:?} outside {:?}", viewing.panel());
    }

    // A narrow Files toolbar (search + Save only) still stays in bounds.
    let narrow = compute_ex(260.0, 240.0, ToolSet::Files, false, false);
    assert!(panel_inside_screen(narrow.panel(), 260.0, 240.0));
    assert!(narrow.units[2] > 0.0);
    for r in narrow.hit_rects() {
        assert!(rect_inside(r, narrow.panel()), "narrow files rect {r:?} outside {:?}", narrow.panel());
    }
}


#[test]
fn toolbar_chips_match_the_tab() {
    use statsplus_logic::layout::{compute_ex, ToolSet};

    let shown = |r: [f32; 4]| r[2] > 0.5 && r[3] > 0.5;
    let at = |view: View, show_group: bool| {
        compute_ex(960.0, 540.0, ToolSet::for_view(view), false, show_group)
    };

    // Vanilla: Group is hidden except on Mobs (All / Kind).
    let gen = at(View::General, false);
    assert!(shown(gen.search) && shown(gen.empty) && shown(gen.sort));
    assert!(shown(gen.units), "General should show the distance/time units chip");
    assert!(!shown(gen.group), "vanilla General should hide the Group chip");

    for view in [View::Items, View::Blocks, View::Food] {
        let ui = at(view, false);
        assert!(shown(ui.search) && shown(ui.empty) && shown(ui.sort), "{view:?}");
        assert!(!shown(ui.units), "units chip must be hidden on {view:?}");
        assert!(!shown(ui.group), "vanilla {view:?} should hide the Group chip");
        // Freed space goes to the remaining search field / chips (no units, no group).
        assert!(
            ui.search[2] + 0.01 >= gen.search[2] && ui.empty[2] + 0.01 >= gen.empty[2],
            "{view:?} should reuse hidden-chip space (search {} vs {}, chip {} vs {})",
            ui.search[2],
            gen.search[2],
            ui.empty[2],
            gen.empty[2]
        );
    }

    let mobs = at(View::Mobs, true);
    assert!(shown(mobs.search) && shown(mobs.empty) && shown(mobs.sort) && shown(mobs.group));
    assert!(shown(mobs.units), "Mobs should keep the All/Hostile/Passive chip");

    let files = at(View::Files, false);
    assert!(shown(files.search), "Files should keep search");
    assert!(shown(files.units), "Files should keep Save");
    assert!(!shown(files.empty) && !shown(files.sort) && !shown(files.group));

    // With a second namespace, Group reappears on General and subject tabs.
    let gen_src = at(View::General, true);
    assert!(shown(gen_src.group), "Source-available General should show Group");
    assert!(shown(gen_src.units));
    for view in [View::Items, View::Blocks, View::Food] {
        let ui = at(view, true);
        assert!(shown(ui.group), "{view:?} should show Group when Source is available");
        assert!(!shown(ui.units));
    }
}


#[test]
fn source_grouping_is_offered_only_with_multiple_namespaces() {
    assert!(!source_grouping_available(["minecraft:stone", "minecraft:dirt"]));
    assert!(!source_grouping_available(["othermod:widget"]));
    assert!(!source_grouping_available(Vec::<&str>::new()));
    assert!(source_grouping_available(["minecraft:stone", "othermod:widget"]));

    // Vanilla: General has only All, so the chip is hidden; Mobs cycles All / Kind.
    assert_eq!(group_options(View::General, false), &[GroupBy::All]);
    assert!(!group_chip_visible(View::General, false));
    assert!(!group_chip_visible(View::Items, false));
    assert_eq!(group_options(View::Mobs, false), &[GroupBy::All, GroupBy::Category]);
    assert!(group_chip_visible(View::Mobs, false));
    assert_eq!(GroupBy::All.next(View::Mobs, false), GroupBy::Category);
    assert_eq!(GroupBy::Category.next(View::Mobs, false), GroupBy::All);
    // A stale Source pref is treated as All, so cycling moves to Kind.
    assert_eq!(GroupBy::Namespace.next(View::Mobs, false), GroupBy::Category);
    assert_eq!(GroupBy::All.next(View::Items, false), GroupBy::All);

    // With a second namespace, Source joins the cycle.
    assert_eq!(group_options(View::Items, true), &[GroupBy::All, GroupBy::Namespace]);
    assert!(group_chip_visible(View::Items, true));
    assert_eq!(GroupBy::All.next(View::Items, true), GroupBy::Namespace);
    assert_eq!(GroupBy::Namespace.next(View::Items, true), GroupBy::All);
    assert_eq!(
        group_options(View::Mobs, true),
        &[GroupBy::All, GroupBy::Category, GroupBy::Namespace]
    );
    assert_eq!(GroupBy::All.next(View::Mobs, true), GroupBy::Category);
    assert_eq!(GroupBy::Category.next(View::Mobs, true), GroupBy::Namespace);
    assert_eq!(GroupBy::Namespace.next(View::Mobs, true), GroupBy::All);
}

#[test]
fn stale_source_pref_falls_back_when_source_is_unavailable() {
    let stale = load_prefs("format=1\ngroup=source\n");
    assert_eq!(stale.group, GroupBy::Namespace);
    assert_eq!(clamp_group(stale.group, View::Items, false), GroupBy::All);
    assert_eq!(clamp_group(stale.group, View::General, false), GroupBy::All);
    assert_eq!(clamp_group(stale.group, View::Mobs, false), GroupBy::All);
    // It comes back if a second namespace appears.
    assert_eq!(clamp_group(stale.group, View::Items, true), GroupBy::Namespace);
    assert_eq!(clamp_group(stale.group, View::Mobs, true), GroupBy::Namespace);
    // Kind is not meaningful off the Mobs tab.
    assert_eq!(clamp_group(GroupBy::Category, View::Items, true), GroupBy::All);
    assert_eq!(clamp_group(GroupBy::Category, View::Mobs, false), GroupBy::Category);
}

fn temp_snap_dir() -> std::path::PathBuf {
    let p = std::env::temp_dir().join(format!(
        "statsplus-snap-test-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&p).unwrap();
    p
}
