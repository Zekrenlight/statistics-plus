// SPDX-License-Identifier: MIT
// Pure rules for Statistics+ (no game runtime).
// Sorting, grouping, units, filtering, listing math, and snapshot files.

use std::collections::{BTreeMap, BTreeSet};

pub use catalog::{
    custom_format, label_of, normalize_id, namespace_of, path_of, FormatKind, View, CUSTOM_STATS,
};
pub use filters::{
    clamp_group, group_chip_visible, group_label, group_options, matches_query,
    sort_rows, source_grouping_available, GroupBy, Row, SortBy, SubjectKind,
    SOURCE_GROUPING_ENABLED,
};
pub use persist::{dump_prefs, load_prefs, Prefs, FILE_NAME, FORMAT};
pub use snapshot::{dump_snapshot, load_from_dir, load_snapshot, resolve_snapshot, save_to_dir, delete_from_dir, list_dir, Snapshot, SnapshotInfo, SnapshotList, SNAPSHOT_KIND};
pub use units::{format_custom, format_distance, format_tenths, format_time, DistUnit, TimeUnit};

/// Namespaced stat type -> subject id -> count.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StatMap {
    pub values: BTreeMap<String, BTreeMap<String, i32>>,
}

impl StatMap {
    pub fn get(&self, ty: &str, key: &str) -> i32 {
        let ty = normalize_id(ty);
        let key = normalize_id(key);
        self.values
            .get(&ty)
            .and_then(|m| m.get(&key))
            .copied()
            .unwrap_or(0)
    }

    pub fn set(&mut self, ty: &str, key: &str, value: i32) {
        let ty = normalize_id(ty);
        let key = normalize_id(key);
        if value == 0 {
            if let Some(m) = self.values.get_mut(&ty) {
                m.remove(&key);
                if m.is_empty() {
                    self.values.remove(&ty);
                }
            }
        } else {
            self.values.entry(ty).or_default().insert(key, value);
        }
    }

    pub fn keys_of(&self, ty: &str) -> BTreeSet<String> {
        let ty = normalize_id(ty);
        self.values
            .get(&ty)
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default()
    }

    pub fn types() -> &'static [&'static str] {
        &[
            "minecraft:custom",
            "minecraft:mined",
            "minecraft:crafted",
            "minecraft:used",
            "minecraft:broken",
            "minecraft:picked_up",
            "minecraft:dropped",
            "minecraft:killed",
            "minecraft:killed_by",
        ]
    }

    pub fn item_total(&self, id: &str) -> i32 {
        self.get("minecraft:used", id)
            + self.get("minecraft:crafted", id)
            + self.get("minecraft:broken", id)
            + self.get("minecraft:picked_up", id)
            + self.get("minecraft:dropped", id)
    }

    pub fn block_total(&self, id: &str) -> i32 {
        self.get("minecraft:mined", id) + self.item_total(id)
    }

    pub fn mob_total(&self, id: &str) -> i32 {
        self.get("minecraft:killed", id) + self.get("minecraft:killed_by", id)
    }
}

pub mod catalog {
    use super::units::{DistUnit, TimeUnit};

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum View {
        General,
        Items,
        Blocks,
        Mobs,
        Food,
        Files,
    }

    impl View {
        pub const ALL: [View; 6] = [
            View::General,
            View::Items,
            View::Blocks,
            View::Mobs,
            View::Food,
            View::Files,
        ];

        pub fn as_u8(self) -> u8 {
            match self {
                View::General => 0,
                View::Items => 1,
                View::Blocks => 2,
                View::Mobs => 3,
                View::Food => 4,
                View::Files => 5,
            }
        }

        pub fn from_u8(v: u8) -> Self {
            match v {
                1 => View::Items,
                2 => View::Blocks,
                3 => View::Mobs,
                4 => View::Food,
                5 | 6 => View::Files,
                _ => View::General,
            }
        }

        pub fn label(self) -> &'static str {
            match self {
                View::General => "General",
                View::Items => "Items",
                View::Blocks => "Blocks",
                View::Food => "Food",
                View::Mobs => "Mobs",
                View::Files => "Files",
            }
        }

        pub fn short(self) -> &'static str {
            self.label()
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum FormatKind {
        Default,
        Time,
        Distance,
        Tenths,
    }

    /// Vanilla custom-stat formatters, in roughly the order the game lists them.
    pub const CUSTOM_STATS: &[(&str, FormatKind)] = &[
        ("minecraft:leave_game", FormatKind::Default),
        ("minecraft:play_time", FormatKind::Time),
        ("minecraft:total_world_time", FormatKind::Time),
        ("minecraft:time_since_death", FormatKind::Time),
        ("minecraft:time_since_rest", FormatKind::Time),
        ("minecraft:sneak_time", FormatKind::Time),
        ("minecraft:walk_one_cm", FormatKind::Distance),
        ("minecraft:crouch_one_cm", FormatKind::Distance),
        ("minecraft:sprint_one_cm", FormatKind::Distance),
        ("minecraft:walk_on_water_one_cm", FormatKind::Distance),
        ("minecraft:fall_one_cm", FormatKind::Distance),
        ("minecraft:climb_one_cm", FormatKind::Distance),
        ("minecraft:fly_one_cm", FormatKind::Distance),
        ("minecraft:walk_under_water_one_cm", FormatKind::Distance),
        ("minecraft:minecart_one_cm", FormatKind::Distance),
        ("minecraft:boat_one_cm", FormatKind::Distance),
        ("minecraft:pig_one_cm", FormatKind::Distance),
        ("minecraft:happy_ghast_one_cm", FormatKind::Distance),
        ("minecraft:horse_one_cm", FormatKind::Distance),
        ("minecraft:aviate_one_cm", FormatKind::Distance),
        ("minecraft:swim_one_cm", FormatKind::Distance),
        ("minecraft:strider_one_cm", FormatKind::Distance),
        ("minecraft:nautilus_one_cm", FormatKind::Distance),
        ("minecraft:jump", FormatKind::Default),
        ("minecraft:drop", FormatKind::Default),
        ("minecraft:damage_dealt", FormatKind::Tenths),
        ("minecraft:damage_dealt_absorbed", FormatKind::Tenths),
        ("minecraft:damage_dealt_resisted", FormatKind::Tenths),
        ("minecraft:damage_taken", FormatKind::Tenths),
        ("minecraft:damage_blocked_by_shield", FormatKind::Tenths),
        ("minecraft:damage_absorbed", FormatKind::Tenths),
        ("minecraft:damage_resisted", FormatKind::Tenths),
        ("minecraft:deaths", FormatKind::Default),
        ("minecraft:mob_kills", FormatKind::Default),
        ("minecraft:animals_bred", FormatKind::Default),
        ("minecraft:player_kills", FormatKind::Default),
        ("minecraft:fish_caught", FormatKind::Default),
        ("minecraft:talked_to_villager", FormatKind::Default),
        ("minecraft:traded_with_villager", FormatKind::Default),
        ("minecraft:eat_cake_slice", FormatKind::Default),
        ("minecraft:fill_cauldron", FormatKind::Default),
        ("minecraft:use_cauldron", FormatKind::Default),
        ("minecraft:clean_armor", FormatKind::Default),
        ("minecraft:clean_banner", FormatKind::Default),
        ("minecraft:clean_shulker_box", FormatKind::Default),
        ("minecraft:interact_with_brewingstand", FormatKind::Default),
        ("minecraft:interact_with_beacon", FormatKind::Default),
        ("minecraft:inspect_dropper", FormatKind::Default),
        ("minecraft:inspect_hopper", FormatKind::Default),
        ("minecraft:inspect_dispenser", FormatKind::Default),
        ("minecraft:play_noteblock", FormatKind::Default),
        ("minecraft:tune_noteblock", FormatKind::Default),
        ("minecraft:pot_flower", FormatKind::Default),
        ("minecraft:trigger_trapped_chest", FormatKind::Default),
        ("minecraft:open_enderchest", FormatKind::Default),
        ("minecraft:enchant_item", FormatKind::Default),
        ("minecraft:play_record", FormatKind::Default),
        ("minecraft:interact_with_furnace", FormatKind::Default),
        ("minecraft:interact_with_crafting_table", FormatKind::Default),
        ("minecraft:open_chest", FormatKind::Default),
        ("minecraft:sleep_in_bed", FormatKind::Default),
        ("minecraft:sleep_in_straw_bed", FormatKind::Default),
        ("minecraft:open_shulker_box", FormatKind::Default),
        ("minecraft:open_barrel", FormatKind::Default),
        ("minecraft:interact_with_blast_furnace", FormatKind::Default),
        ("minecraft:interact_with_smoker", FormatKind::Default),
        ("minecraft:interact_with_lectern", FormatKind::Default),
        ("minecraft:interact_with_campfire", FormatKind::Default),
        ("minecraft:interact_with_cartography_table", FormatKind::Default),
        ("minecraft:interact_with_loom", FormatKind::Default),
        ("minecraft:interact_with_stonecutter", FormatKind::Default),
        ("minecraft:interact_with_anvil", FormatKind::Default),
        ("minecraft:interact_with_grindstone", FormatKind::Default),
        ("minecraft:interact_with_smithing_table", FormatKind::Default),
        ("minecraft:bell_ring", FormatKind::Default),
        ("minecraft:raid_trigger", FormatKind::Default),
        ("minecraft:raid_win", FormatKind::Default),
        ("minecraft:interact_with_flower_pot", FormatKind::Default),
        ("minecraft:target_hit", FormatKind::Default),
        ("minecraft:interact_with_generic_block", FormatKind::Default),
    ];

    pub fn custom_format(id: &str) -> FormatKind {
        let id = normalize_id(id);
        CUSTOM_STATS
            .iter()
            .find(|(k, _)| *k == id)
            .map(|(_, k)| *k)
            .unwrap_or(FormatKind::Default)
    }

    pub fn custom_index(id: &str) -> usize {
        let id = normalize_id(id);
        CUSTOM_STATS
            .iter()
            .position(|(k, _)| *k == id)
            .unwrap_or(CUSTOM_STATS.len() + 1)
    }

    pub fn normalize_id(raw: &str) -> String {
        let s = raw.trim().to_ascii_lowercase().replace(' ', "_");
        if s.is_empty() {
            return s;
        }
        if s.contains(':') {
            s
        } else {
            format!("minecraft:{s}")
        }
    }

    pub fn namespace_of(id: &str) -> &str {
        id.split_once(':').map(|(n, _)| n).unwrap_or("minecraft")
    }

    pub fn path_of(id: &str) -> &str {
        id.split_once(':').map(|(_, p)| p).unwrap_or(id)
    }

    pub fn label_of(id: &str) -> String {
        let path = path_of(id);
        let mut out = String::new();
        for (i, part) in path.split('_').filter(|p| !p.is_empty()).enumerate() {
            if i > 0 {
                out.push(' ');
            }
            let mut chars = part.chars();
            if let Some(c) = chars.next() {
                out.extend(c.to_uppercase());
                out.extend(chars);
            }
        }
        if out.is_empty() {
            id.to_string()
        } else {
            out
        }
    }

    pub fn spawn_egg_id(entity: &str) -> String {
        format!("minecraft:{}_spawn_egg", path_of(entity))
    }

    pub fn format_stat(id: &str, value: i32, dist: DistUnit, time: TimeUnit) -> String {
        super::format_custom(custom_format(id), value, dist, time)
    }
}

pub mod units {
    use super::catalog::FormatKind;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum DistUnit {
        Cm,
        Meters,
        Km,
    }

    impl DistUnit {
        pub fn next(self) -> Self {
            match self {
                DistUnit::Cm => DistUnit::Meters,
                DistUnit::Meters => DistUnit::Km,
                DistUnit::Km => DistUnit::Cm,
            }
        }
        pub fn label(self) -> &'static str {
            match self {
                DistUnit::Cm => "cm",
                DistUnit::Meters => "m",
                DistUnit::Km => "km",
            }
        }
        pub fn from_name(s: &str) -> Self {
            match s.trim().to_ascii_lowercase().as_str() {
                "m" | "meter" | "meters" => DistUnit::Meters,
                "km" | "kilometer" | "kilometers" => DistUnit::Km,
                _ => DistUnit::Cm,
            }
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum TimeUnit {
        Ticks,
        Seconds,
        Minutes,
        Hours,
        Days,
    }

    impl TimeUnit {
        pub fn next(self) -> Self {
            match self {
                TimeUnit::Ticks => TimeUnit::Seconds,
                TimeUnit::Seconds => TimeUnit::Minutes,
                TimeUnit::Minutes => TimeUnit::Hours,
                TimeUnit::Hours => TimeUnit::Days,
                TimeUnit::Days => TimeUnit::Ticks,
            }
        }
        pub fn label(self) -> &'static str {
            match self {
                TimeUnit::Ticks => "ticks",
                TimeUnit::Seconds => "sec",
                TimeUnit::Minutes => "min",
                TimeUnit::Hours => "hr",
                TimeUnit::Days => "days",
            }
        }
        pub fn from_name(s: &str) -> Self {
            match s.trim().to_ascii_lowercase().as_str() {
                "tick" | "ticks" => TimeUnit::Ticks,
                "min" | "minute" | "minutes" => TimeUnit::Minutes,
                "hr" | "hour" | "hours" => TimeUnit::Hours,
                "day" | "days" => TimeUnit::Days,
                _ => TimeUnit::Seconds,
            }
        }
    }

    pub fn format_distance(cm: i32, unit: DistUnit) -> String {
        let cm = cm.max(0) as f64;
        match unit {
            DistUnit::Cm => format!("{} cm", cm as i64),
            DistUnit::Meters => format!("{:.1} m", cm / 100.0),
            DistUnit::Km => format!("{:.2} km", cm / 100_000.0),
        }
    }

    /// Game time is 20 ticks per second.
    pub fn format_time(ticks: i32, unit: TimeUnit) -> String {
        let ticks = ticks.max(0) as f64;
        match unit {
            TimeUnit::Ticks => format!("{} ticks", ticks as i64),
            TimeUnit::Seconds => format!("{:.1} s", ticks / 20.0),
            TimeUnit::Minutes => format!("{:.2} min", ticks / 20.0 / 60.0),
            TimeUnit::Hours => format!("{:.2} hr", ticks / 20.0 / 3600.0),
            TimeUnit::Days => format!("{:.2} d", ticks / 20.0 / 86400.0),
        }
    }

    pub fn format_tenths(value: i32) -> String {
        format!("{:.1}", value as f64 / 10.0)
    }

    pub fn format_custom(kind: FormatKind, value: i32, dist: DistUnit, time: TimeUnit) -> String {
        match kind {
            FormatKind::Default => value.to_string(),
            FormatKind::Time => format_time(value, time),
            FormatKind::Distance => format_distance(value, dist),
            FormatKind::Tenths => format_tenths(value),
        }
    }
}

pub mod filters {
    use super::catalog::{custom_index, label_of, namespace_of, View};
    use super::StatMap;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum SortBy {
        Vanilla,
        Name,
        Value,
    }

    impl SortBy {
        pub fn next(self) -> Self {
            match self {
                SortBy::Vanilla => SortBy::Name,
                SortBy::Name => SortBy::Value,
                SortBy::Value => SortBy::Vanilla,
            }
        }
        pub fn label(self) -> &'static str {
            match self {
                SortBy::Vanilla => "Listed",
                SortBy::Name => "Name",
                SortBy::Value => "Value",
            }
        }
        pub fn from_name(s: &str) -> Self {
            match s.trim().to_ascii_lowercase().as_str() {
                "name" => SortBy::Name,
                "value" | "total" => SortBy::Value,
                _ => SortBy::Vanilla,
            }
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum GroupBy {
        All,
        Namespace,
        Category,
    }

    /// Offer Source grouping in the UI when the current tab's ids span more
    /// than one namespace. Set to `false` to hide Source even if content mods
    /// are present; the grouping code itself stays compiled.
    pub const SOURCE_GROUPING_ENABLED: bool = true;

    pub fn more_than_one_namespace<'a, I>(ids: I) -> bool
    where
        I: IntoIterator<Item = &'a str>,
    {
        let mut first: Option<&str> = None;
        for id in ids {
            let ns = namespace_of(id);
            if ns.is_empty() {
                continue;
            }
            match first {
                None => first = Some(ns),
                Some(seen) if seen != ns => return true,
                Some(_) => {}
            }
        }
        false
    }

    pub fn source_grouping_available<'a, I>(ids: I) -> bool
    where
        I: IntoIterator<Item = &'a str>,
    {
        SOURCE_GROUPING_ENABLED && more_than_one_namespace(ids)
    }

    pub fn group_options(view: View, source_available: bool) -> &'static [GroupBy] {
        match view {
            View::Files => &[GroupBy::All],
            View::Mobs => {
                if source_available {
                    &[GroupBy::All, GroupBy::Category, GroupBy::Namespace]
                } else {
                    &[GroupBy::All, GroupBy::Category]
                }
            }
            _ => {
                if source_available {
                    &[GroupBy::All, GroupBy::Namespace]
                } else {
                    &[GroupBy::All]
                }
            }
        }
    }

    pub fn group_chip_visible(view: View, source_available: bool) -> bool {
        group_options(view, source_available).len() > 1
    }

    pub fn clamp_group(group: GroupBy, view: View, source_available: bool) -> GroupBy {
        let opts = group_options(view, source_available);
        if opts.contains(&group) {
            group
        } else {
            GroupBy::All
        }
    }

    impl GroupBy {
        pub fn next(self, view: View, source_available: bool) -> Self {
            let opts = group_options(view, source_available);
            let cur = clamp_group(self, view, source_available);
            let i = opts.iter().position(|&g| g == cur).unwrap_or(0);
            opts[(i + 1) % opts.len()]
        }
        pub fn label(self) -> &'static str {
            match self {
                GroupBy::All => "All",
                GroupBy::Namespace => "Source",
                GroupBy::Category => "Kind",
            }
        }
        pub fn from_name(s: &str) -> Self {
            match s.trim().to_ascii_lowercase().as_str() {
                "source" | "namespace" | "mod" => GroupBy::Namespace,
                "kind" | "category" => GroupBy::Category,
                _ => GroupBy::All,
            }
        }
    }

    /// The game's entity categories do not have a separate Neutral bucket
    /// (`monster` vs `creature` / `ambient` / water / `misc`). Hostile is
    /// `monster`; everything else is treated as Passive.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum MobFilter {
        All,
        Hostile,
        Passive,
    }

    impl MobFilter {
        pub fn next(self) -> Self {
            match self {
                MobFilter::All => MobFilter::Hostile,
                MobFilter::Hostile => MobFilter::Passive,
                MobFilter::Passive => MobFilter::All,
            }
        }
        pub fn label(self) -> &'static str {
            match self {
                MobFilter::All => "All",
                MobFilter::Hostile => "Hostile",
                MobFilter::Passive => "Passive",
            }
        }
        pub fn from_name(s: &str) -> Self {
            match s.trim().to_ascii_lowercase().as_str() {
                "hostile" | "monster" => MobFilter::Hostile,
                "passive" | "creature" => MobFilter::Passive,
                _ => MobFilter::All,
            }
        }
        pub fn as_name(self) -> &'static str {
            match self {
                MobFilter::All => "all",
                MobFilter::Hostile => "hostile",
                MobFilter::Passive => "passive",
            }
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum SubjectKind {
        Custom,
        Item,
        Block,
        Mob,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct Row {
        pub id: String,
        pub label: String,
        pub total: i32,
        pub group: String,
        pub kind: SubjectKind,
        pub category: String,
        pub extra: Vec<(String, i32)>,
    }

    pub fn matches_query(id: &str, label: &str, query: &str) -> bool {
        let q = query.trim().to_ascii_lowercase();
        if q.is_empty() {
            return true;
        }
        id.to_ascii_lowercase().contains(&q) || label.to_ascii_lowercase().contains(&q)
    }

    pub fn group_label(group: GroupBy, id: &str, category: &str) -> String {
        match group {
            GroupBy::All => String::new(),
            GroupBy::Namespace => {
                let ns = namespace_of(id);
                if ns == "minecraft" {
                    "Minecraft".into()
                } else {
                    ns.to_string()
                }
            }
            GroupBy::Category => {
                if category.is_empty() {
                    "Other".into()
                } else {
                    title_words(category)
                }
            }
        }
    }

    fn title_words(s: &str) -> String {
        label_of(&format!("minecraft:{s}"))
    }

    pub fn sort_rows(rows: &mut [Row], sort: SortBy, view: View) {
        match sort {
            SortBy::Name => rows.sort_by(|a, b| a.label.to_ascii_lowercase().cmp(&b.label.to_ascii_lowercase()).then(a.id.cmp(&b.id))),
            SortBy::Value => rows.sort_by(|a, b| b.total.cmp(&a.total).then(a.label.cmp(&b.label))),
            SortBy::Vanilla => {
                if view == View::General {
                    rows.sort_by_key(|r| (custom_index(&r.id), r.id.clone()));
                } else {
                    rows.sort_by(|a, b| a.id.cmp(&b.id));
                }
            }
        }
    }

    /// Build general rows: known custom stats plus any extras in the map, and an optional session row.
    pub fn general_rows(
        stats: &StatMap,
        query: &str,
        show_empty: bool,
        session_ticks: Option<i32>,
    ) -> Vec<Row> {
        let mut seen = std::collections::HashSet::new();
        let mut rows = Vec::new();
        if let Some(ticks) = session_ticks {
            let label = "Session".to_string();
            if matches_query("statsplus:session", &label, query) && (show_empty || ticks > 0) {
                rows.push(Row {
                    id: "statsplus:session".into(),
                    label,
                    total: ticks,
                    group: String::new(),
                    kind: SubjectKind::Custom,
                    category: String::new(),
                    extra: Vec::new(),
                });
            }
        }
        for (id, _) in super::catalog::CUSTOM_STATS {
            seen.insert((*id).to_string());
            let total = stats.get("minecraft:custom", id);
            if !show_empty && total == 0 {
                continue;
            }
            let label = label_of(id);
            if !matches_query(id, &label, query) {
                continue;
            }
            rows.push(Row {
                id: (*id).to_string(),
                label,
                total,
                group: String::new(),
                kind: SubjectKind::Custom,
                category: String::new(),
                extra: Vec::new(),
            });
        }
        for key in stats.keys_of("minecraft:custom") {
            if seen.contains(&key) {
                continue;
            }
            let total = stats.get("minecraft:custom", &key);
            if !show_empty && total == 0 {
                continue;
            }
            let label = label_of(&key);
            if !matches_query(&key, &label, query) {
                continue;
            }
            rows.push(Row {
                id: key,
                label,
                total,
                group: String::new(),
                kind: SubjectKind::Custom,
                category: String::new(),
                extra: Vec::new(),
            });
        }
        rows
    }

    pub fn subject_rows(
        ids: &[String],
        stats: &StatMap,
        kind: SubjectKind,
        query: &str,
        show_empty: bool,
        categories: &std::collections::HashMap<String, String>,
    ) -> Vec<Row> {
        let mut rows = Vec::new();
        for id in ids {
            let (total, extra) = match kind {
                SubjectKind::Item => {
                    let used = stats.get("minecraft:used", id);
                    let crafted = stats.get("minecraft:crafted", id);
                    let broken = stats.get("minecraft:broken", id);
                    let picked = stats.get("minecraft:picked_up", id);
                    let dropped = stats.get("minecraft:dropped", id);
                    (
                        used + crafted + broken + picked + dropped,
                        vec![
                            ("Used".into(), used),
                            ("Crafted".into(), crafted),
                            ("Broken".into(), broken),
                            ("Picked up".into(), picked),
                            ("Dropped".into(), dropped),
                        ],
                    )
                }
                SubjectKind::Block => {
                    let mined = stats.get("minecraft:mined", id);
                    let used = stats.get("minecraft:used", id);
                    (
                        mined + used,
                        vec![("Mined".into(), mined), ("Used".into(), used)],
                    )
                }
                SubjectKind::Mob => {
                    let killed = stats.get("minecraft:killed", id);
                    let by = stats.get("minecraft:killed_by", id);
                    (
                        killed + by,
                        vec![("Killed".into(), killed), ("Killed by".into(), by)],
                    )
                }
                SubjectKind::Custom => (stats.get("minecraft:custom", id), Vec::new()),
            };
            if !show_empty && total == 0 {
                continue;
            }
            let label = label_of(id);
            if !matches_query(id, &label, query) {
                continue;
            }
            let category = categories.get(id).cloned().unwrap_or_default();
            rows.push(Row {
                id: id.clone(),
                label,
                total,
                group: String::new(),
                kind,
                category,
                extra,
            });
        }
        rows
    }

    pub fn category_is_hostile(category: &str) -> bool {
        category.eq_ignore_ascii_case("monster")
    }

    pub fn matches_mob_filter(category: &str, filter: MobFilter) -> bool {
        match filter {
            MobFilter::All => true,
            MobFilter::Hostile => category_is_hostile(category),
            MobFilter::Passive => !category_is_hostile(category),
        }
    }

    pub fn filter_mob_rows(rows: Vec<Row>, filter: MobFilter) -> Vec<Row> {
        rows.into_iter()
            .filter(|r| matches_mob_filter(&r.category, filter))
            .collect()
    }

    pub fn apply_groups(rows: &mut [Row], group: GroupBy) {
        for row in rows.iter_mut() {
            row.group = group_label(group, &row.id, &row.category);
        }
    }

    /// Stable display order: group headers in first-seen order after sorting within groups.
    pub fn grouped_indices(rows: &[Row], group: GroupBy) -> Vec<(Option<String>, Vec<usize>)> {
        if group == GroupBy::All {
            return vec![(None, (0..rows.len()).collect())];
        }
        let mut order: Vec<String> = Vec::new();
        let mut buckets: std::collections::BTreeMap<String, Vec<usize>> =
            std::collections::BTreeMap::new();
        for (i, row) in rows.iter().enumerate() {
            let g = if row.group.is_empty() {
                "Other".to_string()
            } else {
                row.group.clone()
            };
            if !buckets.contains_key(&g) {
                order.push(g.clone());
            }
            buckets.entry(g).or_default().push(i);
        }
        order
            .into_iter()
            .map(|g| {
                let idxs = buckets.remove(&g).unwrap_or_default();
                (Some(g), idxs)
            })
            .collect()
    }
}

pub mod listing {
    pub const ROW: f32 = 24.0;
    pub const MOB_ROW: f32 = 28.0;
    pub const FILES_ROW: f32 = 32.0;
    pub const BAR_W: f32 = 6.0;
    pub const BAR_GAP: f32 = 2.0;
    pub const THUMB_MIN: f32 = 16.0;
    /// Search + filter chips + gap. Must match the panel's header height.
    pub const LIST_HEADER: f32 = 42.0;
    pub const PANEL_W_MIN: f32 = 360.0;
    pub const PANEL_W_MAX: f32 = 560.0;
    pub const PANEL_H_MIN: f32 = 260.0;
    pub const PANEL_H_MAX: f32 = 420.0;
    pub const BODY_INSET: f32 = 76.0;

    pub fn body_h(panel_h: f32) -> f32 {
        panel_h.clamp(PANEL_H_MIN, PANEL_H_MAX) - BODY_INSET
    }

    pub fn view_h(body_h: f32, header: f32) -> f32 {
        (body_h - header).max(0.0)
    }

    pub fn list_h(body_h: f32) -> f32 {
        view_h(body_h, LIST_HEADER)
    }

    pub fn content_h(total: usize, row: f32) -> f32 {
        total as f32 * row
    }

    pub fn max_scroll(total: usize, view_h: f32, row: f32) -> f32 {
        (content_h(total, row) - view_h).max(0.0)
    }

    pub fn clamp_scroll(scroll: f32, total: usize, view_h: f32, row: f32) -> f32 {
        if !scroll.is_finite() {
            return 0.0;
        }
        scroll.clamp(0.0, max_scroll(total, view_h, row))
    }

    /// Rows that intersect the viewport, including a partially visible last row.
    pub fn shown_rows(scroll: f32, total: usize, view_h: f32, row: f32) -> (usize, usize) {
        if total == 0 || view_h <= 0.0 || row <= 0.0 {
            return (0, 0);
        }
        let scroll = clamp_scroll(scroll, total, view_h, row);
        let top = scroll;
        let bot = scroll + view_h;
        let mut start = None;
        let mut end = 0;
        for i in 0..total {
            let y0 = i as f32 * row;
            let y1 = y0 + row;
            if y1 > top && y0 < bot {
                if start.is_none() {
                    start = Some(i);
                }
                end = i + 1;
            }
        }
        (start.unwrap_or(0), end)
    }

    /// The last row can be scrolled fully into view.
    pub fn last_row_fully_visible(total: usize, view_h: f32, row: f32) -> bool {
        if total == 0 {
            return true;
        }
        if content_h(total, row) <= view_h + 0.01 {
            let (_, end) = shown_rows(0.0, total, view_h, row);
            return end == total;
        }
        let scroll = max_scroll(total, view_h, row);
        let y0 = row_top(total - 1, scroll);
        y0 >= -0.01 && y0 + row <= view_h + 0.01
    }

    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct Thumb {
        pub y: f32,
        pub h: f32,
    }

    pub fn thumb(scroll: f32, total: usize, view_h: f32, track_h: f32, row: f32) -> Option<Thumb> {
        if total == 0 || track_h <= 0.0 || content_h(total, row) <= view_h + 0.01 {
            return None;
        }
        let thumb_h = (view_h / content_h(total, row) * track_h).clamp(THUMB_MIN, track_h);
        let max = max_scroll(total, view_h, row);
        let scroll = clamp_scroll(scroll, total, view_h, row);
        let travel = (track_h - thumb_h).max(0.0);
        let y = if max <= 0.0 { 0.0 } else { (scroll / max) * travel };
        Some(Thumb { y, h: thumb_h })
    }

    pub fn scroll_at_track(local_y: f32, total: usize, view_h: f32, track_h: f32, row: f32) -> f32 {
        let Some(t) = thumb(0.0, total, view_h, track_h, row) else {
            return 0.0;
        };
        let travel = (track_h - t.h).max(0.0);
        if travel <= 0.0 {
            return 0.0;
        }
        let y = (local_y - t.h * 0.5).clamp(0.0, travel);
        (y / travel) * max_scroll(total, view_h, row)
    }

    pub fn scroll_from_drag(
        local_y: f32,
        grab: f32,
        total: usize,
        view_h: f32,
        track_h: f32,
        row: f32,
    ) -> f32 {
        let Some(t) = thumb(0.0, total, view_h, track_h, row) else {
            return 0.0;
        };
        let travel = (track_h - t.h).max(0.0);
        if travel <= 0.0 {
            return 0.0;
        }
        let y = (local_y - grab).clamp(0.0, travel);
        (y / travel) * max_scroll(total, view_h, row)
    }

    pub fn row_top(index: usize, scroll: f32) -> f32 {
        index as f32 * ROW - scroll
    }

    pub fn row_top_sized(index: usize, scroll: f32, row: f32) -> f32 {
        index as f32 * row - scroll
    }

    /// Intersection of two `[x, y, w, h]` rects, or `None` when they do not overlap.
    pub fn intersect_rects(a: [f32; 4], b: [f32; 4]) -> Option<[f32; 4]> {
        let x0 = a[0].max(b[0]);
        let y0 = a[1].max(b[1]);
        let x1 = (a[0] + a[2]).min(b[0] + b[2]);
        let y1 = (a[1] + a[3]).min(b[1] + b[3]);
        if x1 > x0 && y1 > y0 {
            Some([x0, y0, x1 - x0, y1 - y0])
        } else {
            None
        }
    }

    /// The row's unclipped box in screen space (may extend past the viewport).
    pub fn row_box(list: [f32; 4], index: usize, scroll: f32, row: f32) -> [f32; 4] {
        [list[0], list[1] + row_top_sized(index, scroll, row), list[2].max(0.0), row.max(0.0)]
    }

    /// The portion of a row that sits inside the list viewport, if any.
    /// Drawn and clickable row rects must use this, never the unclipped box.
    pub fn visible_row_rect(list: [f32; 4], index: usize, scroll: f32, row: f32) -> Option<[f32; 4]> {
        intersect_rects(row_box(list, index, scroll, row), list)
    }

    /// Wheel motion as the game reports it: with discrete scrolling a notch is
    /// ±1; a touchpad is pixels / 30. One notch moves one whole row.
    pub fn wheel_delta(dy: f32, row: f32) -> f32 {
        if !dy.is_finite() || !row.is_finite() {
            return 0.0;
        }
        dy * row
    }

    pub fn apply_wheel(scroll: f32, dy: f32, total: usize, view_h: f32, row: f32) -> f32 {
        clamp_scroll(scroll - wheel_delta(dy, row), total, view_h, row)
    }

    /// Pause-menu Statistics button, vanilla grid math.
    /// `extra_rows` is 1 if a links/additions row is present, plus one for each registered pause row.
    pub fn pause_stats_rect(w: f32, h: f32, extra_rows: u8) -> [f32; 4] {
        let grid_w = 212.0;
        let grid_h = 166.0 + 24.0 * f32::from(extra_rows);
        let x0 = ((w - grid_w) * 0.5).floor();
        let y0 = ((h - grid_h) * 0.25).floor();
        [x0 + 110.0, y0 + 74.0, 98.0, 20.0]
    }
}

pub mod layout {
    use super::catalog::View;
    use super::listing::{self, BAR_GAP, BAR_W};

    pub const MARGIN: f32 = 8.0;
    pub const PAD: f32 = 10.0;
    pub const TITLE_TOP: f32 = 8.0;
    pub const TITLE_TO_TABS: f32 = 28.0;
    pub const TAB_H: f32 = 18.0;
    pub const TAB_GAP: f32 = 2.0;
    pub const TAB_TO_BODY: f32 = 6.0;
    pub const BOTTOM_PAD: f32 = 16.0;
    pub const CHIP_H: f32 = 16.0;
    pub const CHIP_GAP: f32 = 2.0;
    pub const SEARCH_CHIP_GAP: f32 = 4.0;
    pub const TOOL_ROW_GAP: f32 = 4.0;
    pub const TOOL_PAD_AFTER: f32 = 6.0;
    pub const CHIP_W_IDEAL: f32 = 70.0;
    pub const CHIP_W_MIN: f32 = 44.0;
    pub const SEARCH_W_MIN: f32 = 64.0;
    pub const BANNER_H: f32 = 18.0;
    pub const LIVE_W: f32 = 44.0;
    pub const DONE_W: f32 = 52.0;
    pub const DONE_H: f32 = 16.0;
    pub const MIN_LIST: f32 = 24.0;
    const EPS: f32 = 0.05;

    #[derive(Clone, Debug)]
    pub struct Layout {
        pub x: f32,
        pub y: f32,
        pub w: f32,
        pub h: f32,
        pub done: [f32; 4],
        pub views: [[f32; 4]; 6],
        pub tab_compact: u8,
        pub body: [f32; 4],
        pub search: [f32; 4],
        pub empty: [f32; 4],
        pub sort: [f32; 4],
        pub group: [f32; 4],
        pub units: [f32; 4],
        pub tools_wrapped: bool,
        pub compact_tools: bool,
        pub list: [f32; 4],
        pub bar: [f32; 4],
        pub header_h: f32,
        pub banner: [f32; 4],
        pub live: [f32; 4],
        pub status: [f32; 4],
    }

    impl Layout {
        pub fn panel(&self) -> [f32; 4] {
            [self.x, self.y, self.w, self.h]
        }

        pub fn tab_caption(&self, view: View) -> &'static str {
            tab_caption(view, self.views[0][2])
        }

        pub fn hit_rects(&self) -> Vec<[f32; 4]> {
            let mut v = vec![self.panel(), self.done, self.body, self.search];
            for r in [
                self.empty,
                self.sort,
                self.group,
                self.units,
                self.list,
                self.bar,
                self.banner,
                self.live,
                self.status,
            ] {
                if r[2] > 0.0 && r[3] > 0.0 {
                    v.push(r);
                }
            }
            v.extend_from_slice(&self.views);
            v
        }
    }

    pub fn tab_caption(view: View, tab_w: f32) -> &'static str {
        if tab_w >= 56.0 {
            view.label()
        } else if tab_w >= 32.0 {
            match view {
                View::General => "Gen",
                View::Items => "Items",
                View::Blocks => "Blk",
                View::Mobs => "Mobs",
                View::Food => "Food",
                View::Files => "Files",
            }
        } else {
            match view {
                View::General => "G",
                View::Items => "I",
                View::Blocks => "B",
                View::Mobs => "M",
                View::Food => "Fd",
                View::Files => "Fl",
            }
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum ToolSet {
        /// Search, Filled, Sort, Group, and the distance/time chip.
        General,
        /// Search, Filled, Sort, Group — no units chip (Items, Blocks, Food).
        Subjects,
        /// Search, Filled, Sort, Group, and the All/Hostile/Passive chip.
        Mobs,
        /// Search and Save only.
        Files,
    }

    impl ToolSet {
        pub fn for_view(view: super::View) -> Self {
            match view {
                super::View::General => ToolSet::General,
                super::View::Items | super::View::Blocks | super::View::Food => ToolSet::Subjects,
                super::View::Mobs => ToolSet::Mobs,
                super::View::Files => ToolSet::Files,
            }
        }

        pub fn slots(self, show_group: bool) -> ToolSlots {
            match self {
                ToolSet::Files => ToolSlots {
                    empty: false,
                    sort: false,
                    group: false,
                    units: true,
                },
                ToolSet::General => ToolSlots {
                    empty: true,
                    sort: true,
                    group: show_group,
                    units: true,
                },
                ToolSet::Subjects => ToolSlots {
                    empty: true,
                    sort: true,
                    group: show_group,
                    units: false,
                },
                ToolSet::Mobs => ToolSlots {
                    empty: true,
                    sort: true,
                    group: show_group,
                    units: true,
                },
            }
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct ToolSlots {
        pub empty: bool,
        pub sort: bool,
        pub group: bool,
        pub units: bool,
    }

    impl ToolSlots {
        pub fn count(self) -> usize {
            [self.empty, self.sort, self.group, self.units]
                .into_iter()
                .filter(|v| *v)
                .count()
        }
    }

    pub fn compute(sw: f32, sh: f32) -> Layout {
        // Vanilla General: no Source grouping, so the Group chip is hidden.
        compute_ex(sw, sh, ToolSet::General, false, false)
    }

    pub fn compute_ex(sw: f32, sh: f32, tools: ToolSet, viewing: bool, show_group: bool) -> Layout {
        let sw = sw.max(1.0);
        let sh = sh.max(1.0);
        let max_w = (sw - MARGIN * 2.0).max(1.0);
        let max_h = (sh - MARGIN * 2.0).max(1.0);
        let want_w = (sw * 0.70).clamp(listing::PANEL_W_MIN.min(max_w), listing::PANEL_W_MAX);
        let want_h = (sh * 0.78).clamp(listing::PANEL_H_MIN.min(max_h), listing::PANEL_H_MAX);
        let w = want_w.min(max_w);
        let h = want_h.min(max_h);
        let x = ((sw - w) * 0.5).clamp(MARGIN, (sw - w - MARGIN).max(MARGIN));
        let y = ((sh - h) * 0.5).clamp(MARGIN, (sh - h - MARGIN).max(MARGIN));
        // Final clamp so the panel never crosses the margin box.
        let x = x.min((sw - MARGIN - w).max(MARGIN));
        let y = y.min((sh - MARGIN - h).max(MARGIN));

        let done_w = DONE_W.min((w - PAD * 2.0).max(24.0));
        let done = [x + w - PAD - done_w, y + TITLE_TOP, done_w, DONE_H];

        let inner_x = x + PAD;
        let inner_w = (w - PAD * 2.0).max(1.0);
        let tab_y = y + TITLE_TO_TABS;
        let tab_n = View::ALL.len() as f32;
        let tab_slot = inner_w / tab_n;
        let tab_w = (tab_slot - TAB_GAP).max(1.0);
        let mut views = [[0.0; 4]; 6];
        for i in 0..View::ALL.len() {
            views[i] = [inner_x + tab_slot * i as f32, tab_y, tab_w, TAB_H];
        }
        let tab_compact = if tab_w >= 56.0 {
            0
        } else if tab_w >= 32.0 {
            1
        } else {
            2
        };

        let top = TITLE_TO_TABS + TAB_H + TAB_TO_BODY;
        let bottom = BOTTOM_PAD.min((h - top).max(0.0));
        let avail = (h - top - bottom).max(0.0);
        let body = [inner_x, y + top, inner_w, avail];

        let slots = tools.slots(show_group);
        let n_chips = slots.count().max(1);
        let gaps = SEARCH_CHIP_GAP + CHIP_GAP * (n_chips.saturating_sub(1) as f32);
        let min_one = SEARCH_W_MIN + CHIP_W_MIN * n_chips as f32 + gaps;
        let need_wrap = inner_w + 0.01 < min_one;
        let header_one = CHIP_H + TOOL_PAD_AFTER;
        let header_two = CHIP_H + TOOL_ROW_GAP + CHIP_H + TOOL_PAD_AFTER;
        let wrap = need_wrap && header_two + MIN_LIST <= avail + 0.01;
        let header_h = if wrap { header_two } else { header_one }.min(avail);

        let (search, empty, sort, group, units, compact_tools) =
            place_toolbar(inner_x, body[1], inner_w, wrap, slots);

        let banner_h = if viewing { BANNER_H + 2.0 } else { 0.0 };
        let banner_y = body[1] + header_h;
        let list_top = banner_y + banner_h;
        let list_h = (body[1] + body[3] - list_top).max(0.0);
        let bar_space = BAR_W + BAR_GAP;
        let list_w = (inner_w - bar_space).max(0.0);
        let list = [inner_x, list_top, list_w, list_h];
        let bar = [list[0] + list[2] + BAR_GAP, list[1], BAR_W.min(inner_w), list_h];

        let done = clamp_into(done, [x, y, w, h]);
        let mut views = views;
        for v in &mut views {
            *v = clamp_into(*v, [x, y, w, h]);
        }
        let body = clamp_into(body, [x, y, w, h]);
        let search = clamp_into(search, body);
        let empty = clamp_into(empty, body);
        let sort = clamp_into(sort, body);
        let group = clamp_into(group, body);
        let units = clamp_into(units, body);
        let list = clamp_into(list, body);
        let bar = clamp_into(bar, body);
        let (banner, live) = if viewing {
            let banner = clamp_into(
                [inner_x, banner_y, inner_w, BANNER_H],
                body,
            );
            let live_w = LIVE_W.min((banner[2] * 0.3).max(28.0));
            let live = clamp_into(
                [banner[0] + banner[2] - live_w, banner[1], live_w, banner[3]],
                banner,
            );
            (banner, live)
        } else {
            ([0.0; 4], [0.0; 4])
        };
        let status = [inner_x, y + h - bottom, inner_w, bottom.max(0.0)];
        Layout {
            x,
            y,
            w,
            h,
            done,
            views,
            tab_compact,
            body,
            search,
            empty,
            sort,
            group,
            units,
            tools_wrapped: wrap,
            compact_tools,
            list,
            bar,
            header_h,
            banner,
            live,
            status,
        }
    }

    fn place_toolbar(
        x: f32,
        y: f32,
        width: f32,
        wrap: bool,
        slots: ToolSlots,
    ) -> ([f32; 4], [f32; 4], [f32; 4], [f32; 4], [f32; 4], bool) {
        let n = slots.count().max(1);
        let n_f = n as f32;
        let gaps_chips = CHIP_GAP * (n - 1) as f32;
        let laid = |y_chips: f32, search: [f32; 4], chip_w: f32, start_x: f32, compact: bool| {
            let mut cx = start_x;
            let mut next = || {
                let r = [cx, y_chips, chip_w, CHIP_H];
                cx += chip_w + CHIP_GAP;
                r
            };
            let empty = if slots.empty { next() } else { [0.0; 4] };
            let sort = if slots.sort { next() } else { [0.0; 4] };
            let group = if slots.group { next() } else { [0.0; 4] };
            let units = if slots.units { next() } else { [0.0; 4] };
            (search, empty, sort, group, units, compact)
        };
        if wrap {
            let search = [x, y, width.max(1.0), CHIP_H];
            let chip_w = ((width - gaps_chips) / n_f).max(1.0).min(CHIP_W_IDEAL);
            let y2 = y + CHIP_H + TOOL_ROW_GAP;
            return laid(y2, search, chip_w, x, true);
        }
        let gaps = SEARCH_CHIP_GAP + gaps_chips;
        let chip_budget = (width - gaps - SEARCH_W_MIN).max(0.0);
        let chip_w = (chip_budget / n_f).clamp(CHIP_W_MIN.min(chip_budget / n_f), CHIP_W_IDEAL);
        let search_w = (width - (chip_w * n_f + gaps_chips + SEARCH_CHIP_GAP)).max(1.0);
        let search = [x, y, search_w, CHIP_H];
        laid(y, search, chip_w, x + search_w + SEARCH_CHIP_GAP, chip_w < 58.0)
    }

    pub fn rect_inside(inner: [f32; 4], outer: [f32; 4]) -> bool {
        inner[2] >= -EPS
            && inner[3] >= -EPS
            && inner[0] >= outer[0] - EPS
            && inner[1] >= outer[1] - EPS
            && inner[0] + inner[2] <= outer[0] + outer[2] + EPS
            && inner[1] + inner[3] <= outer[1] + outer[3] + EPS
    }

    fn clamp_into(r: [f32; 4], outer: [f32; 4]) -> [f32; 4] {
        let x1 = r[0].max(outer[0]);
        let y1 = r[1].max(outer[1]);
        let x2 = (r[0] + r[2].max(0.0)).min(outer[0] + outer[2]).max(x1);
        let y2 = (r[1] + r[3].max(0.0)).min(outer[1] + outer[3]).max(y1);
        [x1, y1, x2 - x1, y2 - y1]
    }

    pub fn panel_inside_screen(panel: [f32; 4], sw: f32, sh: f32) -> bool {
        panel[2] > 0.0
            && panel[3] > 0.0
            && panel[0] >= MARGIN - EPS
            && panel[1] >= MARGIN - EPS
            && panel[0] + panel[2] <= sw - MARGIN + EPS
            && panel[1] + panel[3] <= sh - MARGIN + EPS
    }
}

pub mod snapshot {
    use super::catalog::normalize_id;
    use super::StatMap;

    pub const SNAPSHOT_KIND: &str = "statsplus-snapshot";
    pub const SNAPSHOT_FORMAT: u32 = 1;

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct Snapshot {
        pub format: u32,
        pub kind: String,
        pub name: String,
        pub stats: StatMap,
    }

    impl Snapshot {
        pub fn new(name: impl Into<String>, stats: StatMap) -> Self {
            Self {
                format: SNAPSHOT_FORMAT,
                kind: SNAPSHOT_KIND.into(),
                name: name.into(),
                stats,
            }
        }
    }

    pub fn dump_snapshot(s: &Snapshot) -> String {
        let mut out = String::new();
        out.push_str("{\n");
        out.push_str(&format!("  \"format\": {},\n", s.format.max(SNAPSHOT_FORMAT)));
        out.push_str(&format!("  \"kind\": \"{}\",\n", escape(&s.kind)));
        out.push_str(&format!("  \"name\": \"{}\",\n", escape(&s.name)));
        out.push_str("  \"stats\": {\n");
        let types: Vec<_> = s.stats.values.iter().collect();
        for (ti, (ty, keys)) in types.iter().enumerate() {
            out.push_str(&format!("    \"{}\": {{\n", escape(ty)));
            let entries: Vec<_> = keys.iter().collect();
            for (ki, (k, v)) in entries.iter().enumerate() {
                let comma = if ki + 1 == entries.len() { "" } else { "," };
                out.push_str(&format!("      \"{}\": {}{}\n", escape(k), v, comma));
            }
            let comma = if ti + 1 == types.len() { "" } else { "," };
            out.push_str(&format!("    }}{}\n", comma));
        }
        out.push_str("  }\n");
        out.push_str("}\n");
        out
    }

    pub fn load_snapshot(text: &str) -> Result<Snapshot, String> {
        let v = parse_json(text)?;
        v.as_object().ok_or("snapshot must be a JSON object")?;
        let format = v
            .get("format")
            .and_then(Json::as_u32)
            .unwrap_or(SNAPSHOT_FORMAT);
        let kind = v
            .get("kind")
            .and_then(Json::as_str)
            .unwrap_or(SNAPSHOT_KIND)
            .to_string();
        let name = v
            .get("name")
            .and_then(Json::as_str)
            .unwrap_or("")
            .to_string();
        let stats_val = v
            .get("stats")
            .ok_or("missing stats object")?;
        let stats = parse_stats(stats_val)?;
        Ok(Snapshot {
            format,
            kind,
            name,
            stats,
        })
    }

    fn parse_stats(v: &Json) -> Result<StatMap, String> {
        let obj = v.as_object().ok_or("stats must be an object")?;
        let mut stats = StatMap::default();
        for (ty, keys) in obj {
            let typ = normalize_id(ty);
            let map = keys.as_object().ok_or_else(|| format!("stat type {ty} must be an object"))?;
            for (key, val) in map {
                let n = val
                    .as_i32()
                    .ok_or_else(|| format!("stat {ty}/{key} must be an integer"))?;
                if n != 0 {
                    stats
                        .values
                        .entry(typ.clone())
                        .or_default()
                        .insert(normalize_id(key), n);
                }
            }
        }
        Ok(stats)
    }

    fn escape(s: &str) -> String {
        let mut o = String::new();
        for c in s.chars() {
            match c {
                '"' => o.push_str("\\\""),
                '\\' => o.push_str("\\\\"),
                '\n' => o.push_str("\\n"),
                '\r' => o.push_str("\\r"),
                '\t' => o.push_str("\\t"),
                c if c.is_control() => o.push_str(&format!("\\u{:04x}", c as u32)),
                c => o.push(c),
            }
        }
        o
    }

    #[derive(Clone, Debug)]
    #[allow(dead_code)] // Parsed in full; only some values are read back.
    enum Json {
        Null,
        Bool(bool),
        Num(f64),
        Str(String),
        Arr(Vec<Json>),
        Obj(Vec<(String, Json)>),
    }

    impl Json {
        fn as_object(&self) -> Option<&[(String, Json)]> {
            match self {
                Json::Obj(v) => Some(v),
                _ => None,
            }
        }
        fn get(&self, key: &str) -> Option<&Json> {
            self.as_object()?
                .iter()
                .find(|(k, _)| k == key)
                .map(|(_, v)| v)
        }
        fn as_str(&self) -> Option<&str> {
            match self {
                Json::Str(s) => Some(s),
                _ => None,
            }
        }
        fn as_u32(&self) -> Option<u32> {
            self.as_i32().map(|n| n.max(0) as u32)
        }
        fn as_i32(&self) -> Option<i32> {
            match self {
                Json::Num(n) if n.is_finite() && n.fract() == 0.0 => Some(*n as i32),
                _ => None,
            }
        }
    }

    fn parse_json(text: &str) -> Result<Json, String> {
        let mut p = Parser {
            s: text.as_bytes(),
            i: 0,
        };
        let v = p.parse_value()?;
        p.skip();
        if p.i != p.s.len() {
            return Err("trailing data in snapshot".into());
        }
        Ok(v)
    }

    struct Parser<'a> {
        s: &'a [u8],
        i: usize,
    }

    impl Parser<'_> {
        fn skip(&mut self) {
            while self.i < self.s.len() && self.s[self.i].is_ascii_whitespace() {
                self.i += 1;
            }
        }
        fn peek(&mut self) -> Option<u8> {
            self.skip();
            self.s.get(self.i).copied()
        }
        fn bump(&mut self) -> Result<u8, String> {
            self.skip();
            let c = self.s.get(self.i).copied().ok_or("unexpected end of JSON")?;
            self.i += 1;
            Ok(c)
        }
        fn parse_value(&mut self) -> Result<Json, String> {
            match self.peek() {
                Some(b'{') => self.parse_object(),
                Some(b'[') => self.parse_array(),
                Some(b'"') => Ok(Json::Str(self.parse_string()?)),
                Some(b't') => self.eat("true").map(|_| Json::Bool(true)),
                Some(b'f') => self.eat("false").map(|_| Json::Bool(false)),
                Some(b'n') => self.eat("null").map(|_| Json::Null),
                Some(b'-') | Some(b'0'..=b'9') => self.parse_number(),
                Some(c) => Err(format!("unexpected '{c}' in JSON")),
                None => Err("unexpected end of JSON".into()),
            }
        }
        fn eat(&mut self, lit: &str) -> Result<(), String> {
            self.skip();
            if self.s[self.i..].starts_with(lit.as_bytes()) {
                self.i += lit.len();
                Ok(())
            } else {
                Err(format!("expected {lit}"))
            }
        }
        fn parse_object(&mut self) -> Result<Json, String> {
            if self.bump()? != b'{' {
                return Err("expected object".into());
            }
            let mut fields = Vec::new();
            loop {
                self.skip();
                if self.peek() == Some(b'}') {
                    self.i += 1;
                    break;
                }
                if !fields.is_empty() {
                    if self.bump()? != b',' {
                        return Err("expected comma in object".into());
                    }
                }
                let k = self.parse_string()?;
                if self.bump()? != b':' {
                    return Err("expected colon".into());
                }
                let v = self.parse_value()?;
                fields.push((k, v));
            }
            Ok(Json::Obj(fields))
        }
        fn parse_array(&mut self) -> Result<Json, String> {
            if self.bump()? != b'[' {
                return Err("expected array".into());
            }
            let mut arr = Vec::new();
            loop {
                self.skip();
                if self.peek() == Some(b']') {
                    self.i += 1;
                    break;
                }
                if !arr.is_empty() {
                    if self.bump()? != b',' {
                        return Err("expected comma in array".into());
                    }
                }
                arr.push(self.parse_value()?);
            }
            Ok(Json::Arr(arr))
        }
        fn parse_string(&mut self) -> Result<String, String> {
            if self.bump()? != b'"' {
                return Err("expected string".into());
            }
            let mut o = String::new();
            while self.i < self.s.len() {
                let c = self.s[self.i];
                self.i += 1;
                match c {
                    b'"' => return Ok(o),
                    b'\\' => {
                        let e = self.s.get(self.i).copied().ok_or("bad escape")?;
                        self.i += 1;
                        match e {
                            b'"' => o.push('"'),
                            b'\\' => o.push('\\'),
                            b'/' => o.push('/'),
                            b'n' => o.push('\n'),
                            b'r' => o.push('\r'),
                            b't' => o.push('\t'),
                            b'u' => {
                                if self.i + 4 > self.s.len() {
                                    return Err("bad unicode escape".into());
                                }
                                let hex = std::str::from_utf8(&self.s[self.i..self.i + 4])
                                    .map_err(|_| "bad unicode escape")?;
                                let code = u32::from_str_radix(hex, 16)
                                    .map_err(|_| "bad unicode escape")?;
                                o.push(char::from_u32(code).ok_or("bad unicode escape")?);
                                self.i += 4;
                            }
                            _ => return Err("bad escape".into()),
                        }
                    }
                    c if c < 0x20 => return Err("raw control in string".into()),
                    c => o.push(c as char),
                }
            }
            Err("unterminated string".into())
        }
        fn parse_number(&mut self) -> Result<Json, String> {
            self.skip();
            let start = self.i;
            if self.peek() == Some(b'-') {
                self.i += 1;
            }
            while self.i < self.s.len() && self.s[self.i].is_ascii_digit() {
                self.i += 1;
            }
            if self.i < self.s.len() && self.s[self.i] == b'.' {
                self.i += 1;
                while self.i < self.s.len() && self.s[self.i].is_ascii_digit() {
                    self.i += 1;
                }
            }
            let text = std::str::from_utf8(&self.s[start..self.i]).map_err(|_| "bad number")?;
            let n: f64 = text.parse().map_err(|_| "bad number")?;
            Ok(Json::Num(n))
        }
    }
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct SnapshotInfo {
        pub file_name: String,
        pub name: String,
        pub date: String,
        pub summary: String,
        pub play_time: i32,
        /// Descending sort key: modified time then name.
        pub sort_key: (u64, String),
    }

    #[derive(Clone, Debug, Default, PartialEq, Eq)]
    pub struct SnapshotList {
        pub infos: Vec<SnapshotInfo>,
        pub errors: Vec<String>,
    }

    pub fn safe_file_name(name: &str) -> Result<String, String> {
        let base = std::path::Path::new(name)
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| "invalid snapshot name".to_string())?;
        if !base.ends_with(".json")
            || base.contains("..")
            || base.contains('/')
            || base.contains('\\')
        {
            return Err("invalid snapshot name".into());
        }
        Ok(base.to_string())
    }

    pub fn unique_file_name(dir: &std::path::Path, stem: &str) -> String {
        let first = format!("{stem}.json");
        if !dir.join(&first).exists() {
            return first;
        }
        for i in 2..1000 {
            let name = format!("{stem}-{i}.json");
            if !dir.join(&name).exists() {
                return name;
            }
        }
        format!("{stem}-overflow.json")
    }

    pub fn play_time_summary(ticks: i32) -> String {
        let secs = ticks.max(0) as f64 / 20.0;
        if ticks <= 0 {
            return "No play time recorded".into();
        }
        if secs < 60.0 {
            format!("Play time {:.0}s", secs)
        } else if secs < 3600.0 {
            let m = (secs / 60.0).floor();
            let s = secs % 60.0;
            if s < 0.5 {
                format!("Play time {m:.0}m")
            } else {
                format!("Play time {m:.0}m {s:.0}s")
            }
        } else {
            let h = (secs / 3600.0).floor();
            let m = ((secs % 3600.0) / 60.0).floor();
            format!("Play time {h:.0}h {m:.0}m")
        }
    }

    fn info_from_snapshot(file_name: &str, path: &std::path::Path, snap: Snapshot) -> SnapshotInfo {
        let play_time = snap.stats.get("minecraft:custom", "minecraft:play_time");
        let summary = play_time_summary(play_time);
        let name = if snap.name.trim().is_empty() {
            file_name.trim_end_matches(".json").to_string()
        } else {
            snap.name
        };
        let modified = path
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let date = if looks_like_stamp(&name) {
            name.clone()
        } else {
            format_unix_minute(modified)
        };
        SnapshotInfo {
            file_name: file_name.to_string(),
            name,
            date,
            summary,
            play_time,
            sort_key: (modified, file_name.to_string()),
        }
    }

    fn looks_like_stamp(name: &str) -> bool {
        let b = name.as_bytes();
        // YYYY-MM-DD HH:MM, optionally with seconds.
        b.len() >= 16
            && b[4] == b'-'
            && b[7] == b'-'
            && (b[10] == b' ' || b[10] == b'T')
            && b[13] == b':'
    }

    fn format_unix_minute(secs: u64) -> String {
        let days = (secs / 86400) as i64;
        let rem = secs % 86400;
        let (y, mo, d) = civil_from_days(days);
        let h = rem / 3600;
        let m = (rem % 3600) / 60;
        format!("{y:04}-{mo:02}-{d:02} {h:02}:{m:02}")
    }

    fn civil_from_days(mut z: i64) -> (i32, u32, u32) {
        z += 719468;
        let era = if z >= 0 { z } else { z - 146096 } / 146097;
        let doe = (z - era * 146097) as u64;
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
        let y = yoe as i64 + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        let y = if m <= 2 { y + 1 } else { y };
        (y as i32, m as u32, d as u32)
    }

    pub fn save_to_dir(
        dir: &std::path::Path,
        name: &str,
        stats: StatMap,
    ) -> Result<SnapshotInfo, String> {
        std::fs::create_dir_all(dir).map_err(|e| format!("could not create snapshots folder: {e}"))?;
        let stem = super::snapshot_file_stem(name);
        let file_name = unique_file_name(dir, &stem);
        let snap = Snapshot::new(name, stats);
        let path = dir.join(&file_name);
        std::fs::write(&path, dump_snapshot(&snap)).map_err(|e| format!("could not write snapshot: {e}"))?;
        Ok(info_from_snapshot(&file_name, &path, snap))
    }

    pub fn list_dir(dir: &std::path::Path) -> SnapshotList {
        let mut out = SnapshotList::default();
        let rd = match std::fs::read_dir(dir) {
            Ok(rd) => rd,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return out,
            Err(e) => {
                out.errors.push(format!("could not read snapshots folder: {e}"));
                return out;
            }
        };
        for ent in rd.flatten() {
            let path = ent.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let Some(file_name) = path.file_name().and_then(|n| n.to_str()).map(|s| s.to_string()) else {
                continue;
            };
            match std::fs::read_to_string(&path) {
                Ok(text) => match load_snapshot(&text) {
                    Ok(snap) => out.infos.push(info_from_snapshot(&file_name, &path, snap)),
                    Err(e) => out.errors.push(format!("Skipped {file_name} ({e})")),
                },
                Err(e) => out.errors.push(format!("Skipped {file_name} ({e})")),
            }
        }
        out.infos.sort_by(|a, b| b.sort_key.cmp(&a.sort_key));
        out
    }

    pub fn load_from_dir(dir: &std::path::Path, name: &str) -> Result<Snapshot, String> {
        let listed = list_dir(dir);
        let file_name = resolve_snapshot(&listed.infos, name)?;
        let path = dir.join(&file_name);
        let text = std::fs::read_to_string(&path).map_err(|e| format!("could not read {file_name}: {e}"))?;
        load_snapshot(&text)
    }

    pub fn delete_from_dir(dir: &std::path::Path, name: &str) -> Result<(), String> {
        let listed = list_dir(dir);
        let file_name = resolve_snapshot(&listed.infos, name)?;
        let path = dir.join(&file_name);
        std::fs::remove_file(&path).map_err(|e| format!("could not delete {file_name}: {e}"))
    }

    pub fn resolve_snapshot(infos: &[SnapshotInfo], arg: &str) -> Result<String, String> {
        let arg = arg.trim();
        if arg.is_empty() {
            return infos
                .first()
                .map(|i| i.file_name.clone())
                .ok_or_else(|| "no snapshots saved yet".to_string());
        }
        let want = if arg.ends_with(".json") {
            arg.to_string()
        } else {
            format!("{arg}.json")
        };
        let lower = arg.to_ascii_lowercase();
        let want_l = want.to_ascii_lowercase();
        if let Some(i) = infos.iter().find(|i| i.file_name.eq_ignore_ascii_case(&want) || i.file_name.eq_ignore_ascii_case(&want_l)) {
            return Ok(i.file_name.clone());
        }
        if let Some(i) = infos.iter().find(|i| i.name.eq_ignore_ascii_case(arg) || i.name.to_ascii_lowercase() == lower) {
            return Ok(i.file_name.clone());
        }
        if let Some(i) = infos.iter().find(|i| {
            let stem = i.file_name.trim_end_matches(".json");
            stem.eq_ignore_ascii_case(arg) || stem.eq_ignore_ascii_case(&super::snapshot_file_stem(arg))
        }) {
            return Ok(i.file_name.clone());
        }
        Err(format!("no snapshot named {arg}"))
    }

    /// Load and delete button rects on a Files row.
    pub fn files_row_buttons(row: [f32; 4]) -> ([f32; 4], [f32; 4]) {
        let btn_w = 50.0;
        let gap = 3.0;
        let h: f32 = 16.0;
        let y = row[1] + 2.0;
        let delete = [row[0] + row[2] - 6.0 - btn_w, y, btn_w, h.min((row[3] - 4.0).max(1.0))];
        let load = [delete[0] - gap - btn_w, y, btn_w, delete[3]];
        (load, delete)
    }

}

pub mod persist {
    use super::catalog::View;
    use super::filters::{GroupBy, MobFilter, SortBy};
    use super::units::{DistUnit, TimeUnit};

    pub const FORMAT: u32 = 1;
    pub const FILE_NAME: &str = "statsplus.cfg";
    #[derive(Clone, Debug, PartialEq)]
    pub struct Prefs {
        pub format: u32,
        pub show_empty: bool,
        pub dist: DistUnit,
        pub time: TimeUnit,
        pub sort: SortBy,
        pub group: GroupBy,
        pub view: View,
        pub mobs: MobFilter,
    }

    impl Default for Prefs {
        fn default() -> Self {
            Self {
                format: FORMAT,
                show_empty: false,
                dist: DistUnit::Meters,
                time: TimeUnit::Hours,
                sort: SortBy::Vanilla,
                group: GroupBy::All,
                view: View::General,
                mobs: MobFilter::All,
            }
        }
    }

    fn flag(v: bool) -> &'static str {
        if v {
            "yes"
        } else {
            "no"
        }
    }

    fn parse_flag(v: &str) -> bool {
        matches!(v.trim().to_ascii_lowercase().as_str(), "yes" | "true" | "on" | "1")
    }

    pub fn dump_prefs(p: &Prefs) -> String {
        let mut s = String::new();
        s.push_str(&format!("format={}\n", p.format.max(FORMAT)));
        s.push_str(&format!("show_empty={}\n", flag(p.show_empty)));
        s.push_str(&format!("dist={}\n", p.dist.label()));
        s.push_str(&format!("time={}\n", p.time.label()));
        s.push_str(&format!("sort={}\n", p.sort.label().to_ascii_lowercase()));
        s.push_str(&format!("group={}\n", p.group.label().to_ascii_lowercase()));
        s.push_str(&format!("view={}\n", p.view.label().to_ascii_lowercase()));
        s.push_str(&format!("mobs={}\n", p.mobs.as_name()));
        s
    }

    pub fn load_prefs(text: &str) -> Prefs {
        let mut p = Prefs::default();
        for raw in text.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((k, v)) = line.split_once('=') else {
                continue;
            };
            let k = k.trim();
            let v = v.trim();
            match k {
                "format" => {
                    if let Ok(n) = v.parse() {
                        p.format = n;
                    }
                }
                "show_empty" => p.show_empty = parse_flag(v),
                "dist" => p.dist = DistUnit::from_name(v),
                "time" => p.time = TimeUnit::from_name(v),
                "sort" => p.sort = SortBy::from_name(v),
                "group" => p.group = GroupBy::from_name(v),
                "view" => {
                    p.view = match v.to_ascii_lowercase().as_str() {
                        "items" => View::Items,
                        "blocks" => View::Blocks,
                        "mobs" => View::Mobs,
                        "food" => View::Food,
                        "hunter" => View::Mobs,
                        "files" => View::Files,
                        _ => View::General,
                    }
                }
                "mobs" => p.mobs = MobFilter::from_name(v),
"panel_key" => {}
                _ => {}
            }
        }
        p
    }
}

/// Collect unique subject ids from a registry list plus any keys already in the map.
pub fn merge_ids(registry: &[String], extras: impl Iterator<Item = String>) -> Vec<String> {
    let mut seen = std::collections::BTreeSet::new();
    let mut out = Vec::new();
    for id in registry.iter().cloned().chain(extras) {
        let id = catalog::normalize_id(&id);
        if seen.insert(id.clone()) {
            out.push(id);
        }
    }
    out
}

pub fn snapshot_file_stem(now: &str) -> String {
    let safe: String = now
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    format!("snapshot-{safe}")
}

/// Close and toggle rules, kept free of the game runtime so they can be tested.
pub mod close {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Destination {
        Pause,
        Playing,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Toggle {
        Open,
        Close,
    }

    /// Per-frame take-over must not clear a pending Done/Escape close.
    /// Returns the claim and want_close flags to keep.
    pub fn apply_take_over(claim: bool, want_close: bool) -> (bool, bool) {
        if want_close {
            (claim, true)
        } else {
            (true, false)
        }
    }

    /// A first auto-claim (the screen opened without our command) returns
    /// to the pause menu, matching how the game itself leaves Statistics.
    pub fn auto_return_to_pause(already_claimed: bool, current: bool) -> bool {
        if already_claimed {
            current
        } else {
            true
        }
    }

    pub fn decide_close(claimed: bool, want_close: bool, return_to_pause: bool) -> Option<Destination> {
        if !want_close || !claimed {
            None
        } else if return_to_pause {
            Some(Destination::Pause)
        } else {
            Some(Destination::Playing)
        }
    }

    pub fn toggle_from_claimed(claimed: bool) -> Toggle {
        if claimed {
            Toggle::Close
        } else {
            Toggle::Open
        }
    }

    /// If the overlay thinks it is open but the game has left Statistics, drop the claim.
    pub fn should_drop_claim(claimed: bool, screen_is_stats: bool) -> bool {
        claimed && !screen_is_stats
    }
}
