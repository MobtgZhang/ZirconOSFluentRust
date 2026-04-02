//! Logical ids for the workspace `resources/manifest.json` entries.
//! Wallpaper and Start menu icons are embedded as BGRA at build time (this crate’s `build.rs`).
//!
//! **Workspace layout** (repo root): `resources/manifest.json`, `resources/icon-catalog.json`,
//! `resources/wallpapers/`, `resources/icons/`. SVG sources follow ZirconOS Fluent; run
//! `cargo run -p xtask -- rasterize-resources` after updating SVGs.
//! Desktop captions and most shell labels: **Noto Sans** (OFL). Context menu labels use the same raster path.
//! Optional CJK later: Source Han fonts in `third_party/fonts/cjk/`.

/// Repo-relative path to the machine-readable asset list (for host tools / docs; not loaded by kernel).
pub const WORKSPACE_RESOURCES_MANIFEST_REL: &str = "resources/manifest.json";

include!(concat!(env!("OUT_DIR"), "/generated_wallpaper.rs"));
include!(concat!(env!("OUT_DIR"), "/generated_icons.rs"));
include!(concat!(env!("OUT_DIR"), "/generated_cursor.rs"));
include!(concat!(env!("OUT_DIR"), "/generated_ascii_font.rs"));

/// Default desktop wallpaper (`role`: wallpaper).
pub const DEFAULT_WALLPAPER_ID: &str = "wallpaper.countryside_dawn.1080p";

pub const ICON_COMPUTER_32: &str = "icon.computer.32";
/// File Explorer / folder slot — Zircon `file_manager.svg`.
pub const ICON_FOLDER_32: &str = "icon.file_manager.32";
pub const ICON_SETTINGS_32: &str = "icon.settings.32";
pub const ICON_TERMINAL_32: &str = "icon.terminal.32";
/// Recycle bin — Zircon `recycle_bin.svg`.
pub const ICON_TRASH_32: &str = "icon.recycle_bin.32";
pub const ICON_DOCUMENTS_32: &str = "icon.documents.32";
pub const ICON_POWER_32: &str = "icon.power.32";

/// Start menu row `i` BGRA (32×32); order matches `START_MENU_ROW_COUNT` in `generated_icons.rs`.
#[inline]
pub fn start_menu_icon_bgra(i: usize) -> Option<&'static [u8]> {
    match i {
        0 => Some(START_MENU_ICON_0_BGRA),
        1 => Some(START_MENU_ICON_1_BGRA),
        2 => Some(START_MENU_ICON_2_BGRA),
        3 => Some(START_MENU_ICON_3_BGRA),
        4 => Some(START_MENU_ICON_4_BGRA),
        5 => Some(START_MENU_ICON_5_BGRA),
        6 => Some(START_MENU_ICON_6_BGRA),
        7 => Some(START_MENU_ICON_7_BGRA),
        8 => Some(START_MENU_ICON_8_BGRA),
        9 => Some(START_MENU_ICON_9_BGRA),
        10 => Some(START_MENU_ICON_10_BGRA),
        _ => None,
    }
}

/// Raster label beside Start menu row `i`.
#[inline]
pub fn start_menu_label_bgra(i: usize) -> Option<(&'static [u8], u32, u32)> {
    match i {
        0 => Some((START_MENU_LABEL_0_BGRA, START_MENU_LABEL_0_W, START_MENU_LABEL_0_H)),
        1 => Some((START_MENU_LABEL_1_BGRA, START_MENU_LABEL_1_W, START_MENU_LABEL_1_H)),
        2 => Some((START_MENU_LABEL_2_BGRA, START_MENU_LABEL_2_W, START_MENU_LABEL_2_H)),
        3 => Some((START_MENU_LABEL_3_BGRA, START_MENU_LABEL_3_W, START_MENU_LABEL_3_H)),
        4 => Some((START_MENU_LABEL_4_BGRA, START_MENU_LABEL_4_W, START_MENU_LABEL_4_H)),
        5 => Some((START_MENU_LABEL_5_BGRA, START_MENU_LABEL_5_W, START_MENU_LABEL_5_H)),
        6 => Some((START_MENU_LABEL_6_BGRA, START_MENU_LABEL_6_W, START_MENU_LABEL_6_H)),
        7 => Some((START_MENU_LABEL_7_BGRA, START_MENU_LABEL_7_W, START_MENU_LABEL_7_H)),
        8 => Some((START_MENU_LABEL_8_BGRA, START_MENU_LABEL_8_W, START_MENU_LABEL_8_H)),
        9 => Some((START_MENU_LABEL_9_BGRA, START_MENU_LABEL_9_W, START_MENU_LABEL_9_H)),
        10 => Some((START_MENU_LABEL_10_BGRA, START_MENU_LABEL_10_W, START_MENU_LABEL_10_H)),
        _ => None,
    }
}

/// Explorer list: alternate computer / folder icons from manifest.
#[inline]
pub fn explorer_row_icon_bgra(row: usize) -> &'static [u8] {
    if row % 2 == 0 {
        EXPLORER_ROW_ICON_0_BGRA
    } else {
        EXPLORER_ROW_ICON_1_BGRA
    }
}

/// Pre-rasterized list row label `i` (Files window default rows).
#[inline]
pub fn explorer_row_label_bgra(i: usize) -> Option<(&'static [u8], u32, u32)> {
    match i {
        0 => Some((EXPLORER_ROW_LABEL_0_BGRA, EXPLORER_ROW_LABEL_0_W, EXPLORER_ROW_LABEL_0_H)),
        1 => Some((EXPLORER_ROW_LABEL_1_BGRA, EXPLORER_ROW_LABEL_1_W, EXPLORER_ROW_LABEL_1_H)),
        2 => Some((EXPLORER_ROW_LABEL_2_BGRA, EXPLORER_ROW_LABEL_2_W, EXPLORER_ROW_LABEL_2_H)),
        3 => Some((EXPLORER_ROW_LABEL_3_BGRA, EXPLORER_ROW_LABEL_3_W, EXPLORER_ROW_LABEL_3_H)),
        4 => Some((EXPLORER_ROW_LABEL_4_BGRA, EXPLORER_ROW_LABEL_4_W, EXPLORER_ROW_LABEL_4_H)),
        5 => Some((EXPLORER_ROW_LABEL_5_BGRA, EXPLORER_ROW_LABEL_5_W, EXPLORER_ROW_LABEL_5_H)),
        _ => None,
    }
}

/// Title bar text for hosted app title index (`hosted_apps` / `app_host::TITLE_*`).
#[inline]
pub fn window_title_bgra(title_idx: usize) -> Option<(&'static [u8], u32, u32)> {
    match title_idx {
        0 => Some((WINDOW_TITLE_0_BGRA, WINDOW_TITLE_0_W, WINDOW_TITLE_0_H)),
        1 => Some((WINDOW_TITLE_1_BGRA, WINDOW_TITLE_1_W, WINDOW_TITLE_1_H)),
        2 => Some((WINDOW_TITLE_2_BGRA, WINDOW_TITLE_2_W, WINDOW_TITLE_2_H)),
        3 => Some((WINDOW_TITLE_3_BGRA, WINDOW_TITLE_3_W, WINDOW_TITLE_3_H)),
        4 => Some((WINDOW_TITLE_4_BGRA, WINDOW_TITLE_4_W, WINDOW_TITLE_4_H)),
        5 => Some((WINDOW_TITLE_5_BGRA, WINDOW_TITLE_5_W, WINDOW_TITLE_5_H)),
        6 => Some((WINDOW_TITLE_6_BGRA, WINDOW_TITLE_6_W, WINDOW_TITLE_6_H)),
        7 => Some((WINDOW_TITLE_7_BGRA, WINDOW_TITLE_7_W, WINDOW_TITLE_7_H)),
        8 => Some((WINDOW_TITLE_8_BGRA, WINDOW_TITLE_8_W, WINDOW_TITLE_8_H)),
        _ => None,
    }
}

/// Desktop left column: icons + English raster captions (`build.rs` + Libertinus).
#[inline]
pub fn desktop_icon_bgra(i: usize) -> Option<&'static [u8]> {
    match i {
        0 => Some(DESKTOP_ICON_0_BGRA),
        1 => Some(DESKTOP_ICON_1_BGRA),
        2 => Some(DESKTOP_ICON_2_BGRA),
        3 => Some(DESKTOP_ICON_3_BGRA),
        _ => None,
    }
}

/// Desktop shortcut caption `i`: BGRA8 straight-alpha and pixel size (see `DESKTOP_CAPTION_TEXTS` in `build.rs`).
#[inline]
pub fn desktop_caption_bgra(i: usize) -> Option<(&'static [u8], u32, u32)> {
    match i {
        0 => Some((DESKTOP_CAPTION_0_BGRA, DESKTOP_CAPTION_0_W, DESKTOP_CAPTION_0_H)),
        1 => Some((DESKTOP_CAPTION_1_BGRA, DESKTOP_CAPTION_1_W, DESKTOP_CAPTION_1_H)),
        2 => Some((DESKTOP_CAPTION_2_BGRA, DESKTOP_CAPTION_2_W, DESKTOP_CAPTION_2_H)),
        3 => Some((DESKTOP_CAPTION_3_BGRA, DESKTOP_CAPTION_3_W, DESKTOP_CAPTION_3_H)),
        _ => None,
    }
}

/// Context menu row label `i`: BGRA8 straight-alpha (`build.rs` `CONTEXT_MENU_LABEL_TEXTS`).
#[inline]
pub fn context_menu_label_bgra(i: usize) -> Option<(&'static [u8], u32, u32)> {
    match i {
        0 => Some((
            CONTEXT_MENU_LABEL_0_BGRA,
            CONTEXT_MENU_LABEL_0_W,
            CONTEXT_MENU_LABEL_0_H,
        )),
        1 => Some((
            CONTEXT_MENU_LABEL_1_BGRA,
            CONTEXT_MENU_LABEL_1_W,
            CONTEXT_MENU_LABEL_1_H,
        )),
        2 => Some((
            CONTEXT_MENU_LABEL_2_BGRA,
            CONTEXT_MENU_LABEL_2_W,
            CONTEXT_MENU_LABEL_2_H,
        )),
        _ => None,
    }
}
