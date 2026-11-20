// SPDX-License-Identifier: MIT
// Statistics+ game side: stats cache, snapshots, and commands.
// Rules live in the mod crate (`mod_statsplus::logic`); this support module
// sits in the game (beside native_hooks/client_tick_post.rs) and talks to it.

use mod_statsplus::logic as rules;
use rules::catalog::normalize_id;
use rules::persist::{dump_prefs, load_prefs, FILE_NAME};
use rules::snapshot::{SnapshotInfo};
use rules::{merge_ids, Prefs, StatMap};
use sunrise_protocol::packets::play::{ClientCommand, Serverbound};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

pub const MOD_ID: &str = "statsplus";
pub const TITLE: &str = "Statistics+";
pub const VERSION: &str = "1.0.0";
pub const BLURB: &str = "A clearer statistics screen for Sunrise, with search, filters, and local snapshots.";
pub const AUTHOR: &str = "Zekrenlight";

#[derive(Clone, Debug)]
pub struct Session {
    pub prefs: Prefs,
    pub ready: bool,
    pub claim: bool,
    pub return_to_pause: bool,
    pub cache: StatMap,
    pub session_start: Option<Instant>,
    pub files: Vec<SnapshotInfo>,
    pub file_errors: Vec<String>,
    pub note: String,
    pub note_error: bool,
    pub tab: u8,
    pub scroll: f32,
    pub bar_grab: Option<f32>,
    pub query: String,
    pub query_focus: bool,
    pub want_close: bool,
    pub viewing: Option<String>,
    pub confirm_delete: Option<(String, Instant)>,
    pub pending: Option<FileOp>,
}

#[derive(Clone, Debug)]
pub enum FileOp {
    Save,
    Load(String),
    Delete(String),
    Live,
}

impl Default for Session {
    fn default() -> Self {
        Self {
            prefs: Prefs::default(),
            ready: false,
            claim: false,
            return_to_pause: false,
            cache: StatMap::default(),
            session_start: None,
            files: Vec::new(),
            file_errors: Vec::new(),
            note: String::new(),
            note_error: false,
            tab: 0,
            scroll: 0.0,
            bar_grab: None,
            query: String::new(),
            query_focus: false,
            want_close: false,
            viewing: None,
            confirm_delete: None,
            pending: None,
        }
    }
}

thread_local! {
    static SESSION: RefCell<Session> = RefCell::new(Session::default());
}

pub fn revise<R>(f: impl FnOnce(&mut Session) -> R) -> R {
    SESSION.with(|c| f(&mut c.borrow_mut()))
}

pub fn session_ticks() -> i32 {
    revise(|s| {
        s.session_start
            .map(|t| {
                let secs = Instant::now().saturating_duration_since(t).as_secs_f64();
                (secs * 20.0) as i32
            })
            .unwrap_or(0)
    })
}

pub fn ensure_ready(game: &crate::game::Game<'_>) {
    let need = revise(|s| !s.ready);
    if need {
        load_prefs_from(game);
        revise(|s| {
            s.ready = true;
            s.tab = s.prefs.view.as_u8();
            if s.session_start.is_none() && game.player.is_some() {
                s.session_start = Some(Instant::now());
            }
        });
        refresh_files(game);
    } else {
        revise(|s| {
            if s.session_start.is_none() && game.player.is_some() {
                s.session_start = Some(Instant::now());
            }
        });
    }
}

fn prefs_path(game: &crate::game::Game<'_>) -> PathBuf {
    game.native_data_dir(MOD_ID).join(FILE_NAME)
}

fn snapshots_dir(game: &crate::game::Game<'_>) -> PathBuf {
    game.native_data_dir(MOD_ID).join("snapshots")
}

fn load_prefs_from(game: &crate::game::Game<'_>) {
    let path = prefs_path(game);
    if let Ok(text) = fs::read_to_string(&path) {
        revise(|s| s.prefs = load_prefs(&text));
    }
}

pub fn write_prefs(game: &crate::game::Game<'_>) {
    let dir = game.native_data_dir(MOD_ID);
    let _ = fs::create_dir_all(&dir);
    let text = revise(|s| dump_prefs(&s.prefs));
    let _ = fs::write(prefs_path(game), text);
}

fn live_stats(game: &crate::game::Game<'_>) -> StatMap {
    let mut cache = StatMap::default();
    for (ty, keys) in &game.stats.values {
        for (key, value) in keys {
            if *value != 0 {
                cache
                    .values
                    .entry(normalize_id(ty))
                    .or_default()
                    .insert(normalize_id(key), *value);
            }
        }
    }
    cache
}

pub fn refresh_stats(game: &crate::game::Game<'_>) {
    if revise(|s| s.viewing.is_some()) {
        return;
    }
    let cache = live_stats(game);
    revise(|s| s.cache = cache);
}

pub fn is_viewing() -> bool {
    revise(|s| s.viewing.is_some())
}

pub fn refresh_files(game: &crate::game::Game<'_>) {
    let dir = snapshots_dir(game);
    let listed = rules::snapshot::list_dir(&dir);
    revise(|s| {
        s.files = listed.infos;
        s.file_errors = listed.errors;
        if !s.file_errors.is_empty() && s.note.is_empty() {
            s.note = s.file_errors[0].clone();
            s.note_error = true;
        }
    });
}

pub fn request_and_open(game: &mut crate::game::Game<'_>, from_pause: bool) {
    ensure_ready(game);
    let _ = game.load_stats();
    if game.remote.is_some() {
        game.send(Serverbound::ClientCommand(ClientCommand { action: 1 }));
    }
    refresh_stats(game);
    refresh_files(game);
    revise(|s| {
        s.claim = true;
        s.return_to_pause = from_pause;
        s.want_close = false;
        s.scroll = 0.0;
        s.query_focus = false;
        if s.viewing.is_none() {
            s.note.clear();
            s.note_error = false;
        }
        s.tab = s.prefs.view.as_u8();
        s.confirm_delete = None;
    });
    if game.screen != crate::game::Screen::Stats {
        game.open_stats();
    }
}

pub fn claimed() -> bool {
    revise(|s| s.claim)
}

/// Take over an open Stats screen: claim it and keep the cache fresh.
/// A pending Done/Escape close is left untouched so the next tick can
/// actually leave the screen.
pub fn take_over_stats(game: &crate::game::Game<'_>) {
    ensure_ready(game);
    let pending = revise(|s| {
        let (claim, want_close) = rules::close::apply_take_over(s.claim, s.want_close);
        if want_close {
            return true;
        }
        s.return_to_pause = rules::close::auto_return_to_pause(s.claim, s.return_to_pause);
        s.claim = claim;
        false
    });
    if pending {
        return;
    }
    refresh_stats(game);
    refresh_files(game);
}

pub fn intercept_pause(game: &crate::game::Game<'_>, screen: &mod_api::ScreenView<'_>) -> bool {
    if screen.id != "paused" {
        return false;
    }
    ensure_ready(game);
    let extra = pause_extra_rows(game);
    let rect = rules::listing::pause_stats_rect(screen.rect[2], screen.rect[3], extra);
    let p = (screen.mouse[0], screen.mouse[1]);
    if hit(p, rect) {
        revise(|s| {
            s.claim = true;
            s.return_to_pause = true;
            s.want_close = false;
            s.scroll = 0.0;
        });
        // Let the pause menu open Screen::Stats; draw_stats is cancelled and we paint over the blur.
        return false;
    }
    false
}

fn has_pause_additions(game: &crate::game::Game<'_>) -> bool {
    game.remote.as_ref().is_some_and(|remote| {
        remote
            .registries
            .tag("minecraft:dialog", "minecraft:pause_screen_additions")
            .is_some_and(|tag| !tag.is_empty())
    })
}

fn pause_extra_rows(game: &crate::game::Game<'_>) -> u8 {
    let row3 = !game.server_links().is_empty() || has_pause_additions(game);
    // Full-width rows other code registers above Options (Sunrise 9425651 dropped the
    // split-screen row; registered pause pages are the only extra rows left).
    let added = sunrise_hooks::pages::engine::all()
        .iter()
        .filter(|page| page.place() == sunrise_hooks::pages::Place::Pause)
        .count();
    u8::from(row3).saturating_add(u8::try_from(added).unwrap_or(u8::MAX))
}

fn hit(p: (f32, f32), r: [f32; 4]) -> bool {
    p.0 >= r[0] && p.0 < r[0] + r[2] && p.1 >= r[1] && p.1 < r[1] + r[3]
}

/// Leave Statistics+ the same way the game leaves Advancements: set the
/// screen to Paused or Playing and fix mouse capture. Does not depend on
/// the stock Statistics widgets, which we no longer draw.
pub fn close(game: &mut crate::game::Game<'_>) {
    let back = revise(|s| {
        s.claim = false;
        s.want_close = false;
        s.query_focus = false;
        s.bar_grab = None;
        s.return_to_pause
    });
    if game.screen == crate::game::Screen::Stats {
        if back {
            game.screen = crate::game::Screen::Paused;
            game.capture(false);
        } else {
            game.screen = crate::game::Screen::Playing;
            game.capture(true);
        }
    }
}

pub fn close_if_asked(game: &mut crate::game::Game<'_>) {
    let dest = revise(|s| {
        let dest = rules::close::decide_close(s.claim, s.want_close, s.return_to_pause);
        if dest.is_some() {
            s.want_close = false;
        }
        dest
    });
    if dest.is_some() {
        close(game);
    }
}

/// `/statsplus panel` toggles: close if we already own the screen, otherwise open it.
pub fn toggle_panel(game: &mut crate::game::Game<'_>) {
    match rules::close::toggle_from_claimed(claimed()) {
        rules::close::Toggle::Close => close(game),
        rules::close::Toggle::Open => {
            request_and_open(game, game.screen == crate::game::Screen::Paused)
        }
    }
}

fn release_stale_claim(game: &crate::game::Game<'_>) {
    if rules::close::should_drop_claim(claimed(), game.screen == crate::game::Screen::Stats) {
        revise(|s| {
            s.claim = false;
            s.want_close = false;
            s.query_focus = false;
            s.bar_grab = None;
        });
    }
}

/// Runs every frame, including while singleplayer is paused (the tick hook does not).
pub fn on_frame(game: &mut crate::game::Game<'_>) {
    close_if_asked(game);
    release_stale_claim(game);
    apply_pending(game);
    expire_confirm();
}

pub fn queue(op: FileOp) {
    revise(|s| s.pending = Some(op));
}

fn apply_pending(game: &crate::game::Game<'_>) {
    let op = revise(|s| s.pending.take());
    match op {
        Some(FileOp::Save) => {
            let _ = save_snapshot(game);
        }
        Some(FileOp::Load(name)) => {
            let _ = load_named_snapshot(game, &name);
        }
        Some(FileOp::Delete(name)) => {
            let _ = delete_named_snapshot(game, &name);
        }
        Some(FileOp::Live) => return_to_live(game),
        None => {}
    }
}

fn expire_confirm() {
    revise(|s| {
        if let Some((_, at)) = &s.confirm_delete {
            if at.elapsed().as_secs_f32() >= 3.0 {
                s.confirm_delete = None;
            }
        }
    });
}

pub fn request_delete(file_name: &str) -> bool {
    let now = Instant::now();
    revise(|s| {
        if let Some((name, at)) = &s.confirm_delete {
            if name == file_name && at.elapsed().as_secs_f32() < 3.0 {
                s.pending = Some(FileOp::Delete(file_name.to_string()));
                s.confirm_delete = None;
                return true;
            }
        }
        s.confirm_delete = Some((file_name.to_string(), now));
        s.note = "Click Delete again to confirm".into();
        s.note_error = false;
        false
    })
}

pub fn confirming_delete(file_name: &str) -> bool {
    revise(|s| {
        s.confirm_delete.as_ref().is_some_and(|(n, at)| n == file_name && at.elapsed().as_secs_f32() < 3.0)
    })
}

pub fn return_to_live(game: &crate::game::Game<'_>) {
    revise(|s| {
        s.viewing = None;
        s.note = "Live statistics".into();
        s.note_error = false;
    });
    refresh_stats(game);
}

pub fn save_snapshot(game: &crate::game::Game<'_>) -> Result<String, String> {
    let dir = snapshots_dir(game);
    let stamp = local_stamp();
    // Always capture the live game stats, even while a snapshot is being viewed.
    let stats = live_stats(game);
    match rules::snapshot::save_to_dir(&dir, &stamp, stats) {
        Ok(info) => {
            refresh_files(game);
            revise(|s| {
                s.note = format!("Saved {}", info.name);
                s.note_error = false;
            });
            Ok(info.name)
        }
        Err(e) => {
            revise(|s| {
                s.note = format!("Save failed: {e}");
                s.note_error = true;
            });
            Err(e)
        }
    }
}

pub fn load_named_snapshot(game: &crate::game::Game<'_>, name: &str) -> Result<(), String> {
    let dir = snapshots_dir(game);
    match rules::snapshot::load_from_dir(&dir, name) {
        Ok(snap) => {
            let label = if snap.name.trim().is_empty() {
                name.trim_end_matches(".json").to_string()
            } else {
                snap.name.clone()
            };
            revise(|s| {
                s.cache = snap.stats;
                s.viewing = Some(label.clone());
                s.note = format!("Viewing snapshot {label}");
                s.note_error = false;
                s.scroll = 0.0;
                s.confirm_delete = None;
            });
            Ok(())
        }
        Err(e) => {
            revise(|s| {
                s.note = format!("Load failed: {e}");
                s.note_error = true;
            });
            Err(e)
        }
    }
}

pub fn delete_named_snapshot(game: &crate::game::Game<'_>, name: &str) -> Result<(), String> {
    let dir = snapshots_dir(game);
    match rules::snapshot::delete_from_dir(&dir, name) {
        Ok(()) => {
            let viewing = revise(|s| s.viewing.clone());
            refresh_files(game);
            // If the open snapshot was deleted, go back to live.
            let dropped = viewing.as_ref().is_some_and(|v| {
                v == name || v.eq_ignore_ascii_case(name.trim_end_matches(".json"))
            });
            if dropped {
                return_to_live(game);
            }
            revise(|s| {
                s.note = format!("Deleted {name}");
                s.note_error = false;
                s.confirm_delete = None;
            });
            Ok(())
        }
        Err(e) => {
            revise(|s| {
                s.note = format!("Delete failed: {e}");
                s.note_error = true;
            });
            Err(e)
        }
    }
}

fn local_stamp() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M").to_string()
}

pub fn handle_command(game: &mut crate::game::Game<'_>, command: &str, ran: &mut bool) {
    let raw = command.trim();
    let (head, rest) = raw
        .split_once(' ')
        .map(|(a, b)| (a, b.trim()))
        .unwrap_or((raw, ""));
    if !head.eq_ignore_ascii_case("statsplus") && !head.eq_ignore_ascii_case("/statsplus") {
        return;
    }
    *ran = true;
    ensure_ready(game);
    let sub = if rest.is_empty() { "help" } else { rest };
    let (cmd, arg) = sub
        .split_once(' ')
        .map(|(a, b)| (a, b.trim()))
        .unwrap_or((sub, ""));
    match cmd.to_ascii_lowercase().as_str() {
        "panel" | "open" => toggle_panel(game),
        "save" => match save_snapshot(game) {
            Ok(name) => game.native_chat(&format!("Statistics+: saved {name}")),
            Err(e) => game.native_chat(&format!("Statistics+: save failed: {e}")),
        },
        "load" => {
            refresh_files(game);
            let name = if arg.is_empty() {
                revise(|s| s.files.first().map(|i| i.file_name.clone())).unwrap_or_default()
            } else {
                arg.to_string()
            };
            if name.is_empty() {
                game.native_chat("Statistics+: no snapshots saved yet.");
            } else {
                match load_named_snapshot(game, &name) {
                    Ok(()) => {
                        let label = revise(|s| s.viewing.clone()).unwrap_or(name);
                        game.native_chat(&format!("Statistics+: loaded {label}"));
                        request_and_open(game, game.screen == crate::game::Screen::Paused);
                    }
                    Err(e) => game.native_chat(&format!("Statistics+: load failed: {e}")),
                }
            }
        }
        "empty" => {
            let on = revise(|s| {
                s.prefs.show_empty = !s.prefs.show_empty;
                s.prefs.show_empty
            });
            write_prefs(game);
            game.native_chat(&format!(
                "Statistics+: show empty is {}",
                if on { "on" } else { "off" }
            ));
        }
        _ => game.native_chat(
            "Statistics+: /statsplus panel | save | load [name] | empty | help",
        ),
    }
}

pub fn item_ids(game: &crate::game::Game<'_>) -> Vec<String> {
    let mut reg: Vec<String> = game
        .res
        .items()
        .registry_order
        .iter()
        .map(|s| s.to_string())
        .collect();
    if reg.is_empty() {
        reg = game.res.items().items.keys().map(|s| s.to_string()).collect();
        reg.sort();
    }
    let extras = revise(|s| {
        let mut v = Vec::new();
        for ty in [
            "minecraft:used",
            "minecraft:crafted",
            "minecraft:broken",
            "minecraft:picked_up",
            "minecraft:dropped",
        ] {
            v.extend(s.cache.keys_of(ty));
        }
        v
    });
    merge_ids(&reg, extras.into_iter())
}

pub fn block_ids(game: &crate::game::Game<'_>) -> Vec<String> {
    let mut reg: Vec<String> = game
        .res
        .items()
        .registry_order
        .iter()
        .filter(|id| game.res.items().get(id).is_some_and(|i| i.block.is_some()))
        .map(|s| s.to_string())
        .collect();
    if reg.is_empty() {
        reg = game
            .res
            .items()
            .items
            .iter()
            .filter(|(_, i)| i.block.is_some())
            .map(|(k, _)| k.to_string())
            .collect();
        reg.sort();
    }
    let extras = revise(|s| s.cache.keys_of("minecraft:mined").into_iter().collect::<Vec<_>>());
    merge_ids(&reg, extras.into_iter())
}

pub fn food_ids(game: &crate::game::Game<'_>) -> Vec<String> {
    item_ids(game)
        .into_iter()
        .filter(|id| game.res.items().get(id).is_some_and(|i| i.food.is_some()))
        .collect()
}

pub fn mob_ids(game: &crate::game::Game<'_>) -> (Vec<String>, HashMap<String, String>) {
    let mut cats = HashMap::new();
    let mut reg: Vec<String> = game
        .res
        .mob_types()
        .iter()
        .map(|(id, kind)| {
            cats.insert(normalize_id(id), kind.category.clone());
            id.to_string()
        })
        .collect();
    reg.sort();
    let extras = revise(|s| {
        let mut v = s.cache.keys_of("minecraft:killed");
        v.extend(s.cache.keys_of("minecraft:killed_by"));
        v.into_iter().collect::<Vec<_>>()
    });
    let ids = merge_ids(&reg, extras.into_iter());
    (ids, cats)
}

pub fn label(game: &crate::game::Game<'_>, id: &str, kind: rules::filters::SubjectKind) -> String {
    use rules::filters::SubjectKind;
    let key = match kind {
        SubjectKind::Custom => {
            if id == "statsplus:session" {
                return "Session".into();
            }
            format!("stat.{}", id.replace(':', "."))
        }
        SubjectKind::Item | SubjectKind::Block => {
            let item = format!("item.{}", id.replace(':', "."));
            let named = game.translate(&item).unwrap_or_default();
            if !named.is_empty() && named != item {
                return named;
            }
            format!("block.{}", id.replace(':', "."))
        }
        SubjectKind::Mob => format!("entity.{}", id.replace(':', ".")),
    };
    let named = game.translate(&key).unwrap_or_default();
    if !named.is_empty() && named != key {
        named
    } else {
        rules::label_of(id)
    }
}
