//! Wallpaper: first `wallpaper-countryside_dawn-1080p.png`, else any `wallpaper-*-1080p.png`, else gradient.
//! Start menu icons: `resources/manifest.json` assets at 32×32 → `OUT_DIR` BGRA + `generated_icons.rs`.

use std::path::{Path, PathBuf};

use serde_json::Value;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap_or_else(|_| Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
}

fn pick_wallpaper_png() -> Option<PathBuf> {
    let wp_dir = repo_root().join("resources/wallpapers");
    let preferred = wp_dir.join("wallpaper-countryside_dawn-1080p.png");
    if preferred.is_file() {
        return Some(preferred);
    }
    let Ok(rd) = std::fs::read_dir(&wp_dir) else {
        return None;
    };
    let mut pngs: Vec<PathBuf> = rd
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            let n = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            n.starts_with("wallpaper-") && n.ends_with("-1080p.png")
        })
        .collect();
    pngs.sort();
    pngs.into_iter().next()
}

fn gradient_bgra(w: u32, h: u32) -> Vec<u8> {
    let mut v = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let b = (x * 255 / w.max(1)) as u8;
            let g = (y * 255 / h.max(1)) as u8;
            let r = (((x + y) * 128) / (w + h).max(1)) as u8;
            v.push(b);
            v.push(g);
            v.push(r);
            v.push(0xff);
        }
    }
    v
}

fn png_to_bgra_max(path: &Path, max_w: u32, max_h: u32) -> (u32, u32, Vec<u8>) {
    let img = image::open(path)
        .unwrap_or_else(|e| panic!("open {}: {e}", path.display()))
        .to_rgba8();
    let (mut w, mut h) = img.dimensions();
    if w > max_w || h > max_h {
        let scale = (max_w as f64 / w as f64).min(max_h as f64 / h as f64);
        let nw = ((w as f64 * scale).round() as u32).max(1);
        let nh = ((h as f64 * scale).round() as u32).max(1);
        let resized = image::imageops::resize(
            &img,
            nw,
            nh,
            image::imageops::FilterType::Triangle,
        );
        w = nw;
        h = nh;
        let mut v = Vec::with_capacity((w * h * 4) as usize);
        for p in resized.pixels() {
            let c = p.0;
            v.push(c[2]);
            v.push(c[1]);
            v.push(c[0]);
            v.push(c[3]);
        }
        return (w, h, v);
    }
    let mut v = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let c = img.get_pixel(x, y).0;
            v.push(c[2]);
            v.push(c[1]);
            v.push(c[0]);
            v.push(c[3]);
        }
    }
    (w, h, v)
}

fn manifest_asset_path(root: &Path, id: &str) -> PathBuf {
    let mpath = root.join("resources/manifest.json");
    let data = std::fs::read_to_string(&mpath)
        .unwrap_or_else(|e| panic!("read {}: {e}", mpath.display()));
    let v: Value = serde_json::from_str(&data).unwrap_or_else(|e| panic!("manifest json: {e}"));
    let Some(arr) = v.get("assets").and_then(|a| a.as_array()) else {
        panic!("manifest.assets missing");
    };
    for a in arr {
        if a.get("id").and_then(|x| x.as_str()) == Some(id) {
            let rel = a
                .get("path")
                .and_then(|x| x.as_str())
                .unwrap_or_else(|| panic!("manifest path for {id}"));
            return root.join("resources").join(rel);
        }
    }
    panic!("manifest.json has no asset id {id:?}");
}

/// Start menu rows — see `taskbar` / `session::activate_menu_item` (order must match).
const START_MENU_ICON_IDS: [&str; 11] = [
    "icon.terminal.32",
    "icon.file_manager.32",
    "icon.computer.32",
    "icon.settings.32",
    "icon.calendar.32",
    "icon.browser.32",
    "icon.documents.32",
    "icon.calculator.32",
    "icon.mail.32",
    "icon.store.32",
    "icon.power.32",
];

const START_MENU_LABEL_TEXTS: [&str; 11] = [
    "Terminal",
    "Files",
    "Task Manager",
    "Settings",
    "Control Panel",
    "Run",
    "Notepad",
    "Calculator",
    "Documents",
    "About",
    "Power",
];

const EXPLORER_ROW_ICON_IDS: [&str; 2] = ["icon.computer.32", "icon.file_manager.32"];

/// English labels for the Files window default listing (`explorer_view`); rasterized at build time.
const EXPLORER_ROW_LABEL_TEXTS: [&str; 6] = [
    "This PC",
    "System volume (not mounted)",
    "EFI\\Boot",
    "Documents",
    "Pictures",
    "Network (stub)",
];

/// Window title bar rasters (Libertinus, light-on-dark style); order matches `AppId` titles in `app_host`.
const WINDOW_TITLE_TEXTS: [&str; 9] = [
    "Zircon Files",
    "Task Manager",
    "Settings",
    "Control Panel",
    "Run",
    "Notepad",
    "Calculator",
    "About NT10",
    "Properties",
];

/// Left column on wallpaper (`resources/icon-catalog.json` shell_desktop).
const DESKTOP_SHORTCUT_ICON_IDS: [&str; 4] = [
    "icon.computer.32",
    "icon.documents.32",
    "icon.recycle_bin.32",
    "icon.network.32",
];

/// Desktop shortcut captions — rasterized with **Noto Sans** (UI Latin).
const DESKTOP_CAPTION_TEXTS: [&str; 4] = [
    "This PC",
    "Documents",
    "Recycle Bin",
    "Network",
];

/// Context menu rows — **Noto Sans** (Latin UI); CJK menus can switch to **LXGW WenKai** in `kai/`.
/// Order matches `activate_ctx_item` in `session.rs`.
const CONTEXT_MENU_LABEL_TEXTS: [&str; 3] = ["Open", "Properties", "Close"];

/// Prefer Noto Sans (OFL); allow other common open-licensed UI fonts if present.
const FONT_CANDIDATE_REL_PATHS: [&str; 3] = [
    "third_party/fonts/latin/NotoSans-Regular.ttf",
    "third_party/fonts/latin/LiberationSans-Regular.ttf",
    "third_party/fonts/latin/DejaVuSans.ttf",
];

fn find_ui_sans_bytes(root: &Path) -> Option<Vec<u8>> {
    for rel in FONT_CANDIDATE_REL_PATHS {
        let p = root.join(rel);
        println!("cargo:rerun-if-changed={}", p.display());
        if let Ok(b) = std::fs::read(&p) {
            return Some(b);
        }
    }
    None
}

/// Build-time UI font: real OFL TTF via fontdue, or **placeholder** bars (no Microsoft fonts).
enum BuildUiFont {
    Real(fontdue::Font),
    Placeholder,
}

impl BuildUiFont {
    fn caption(&self, text: &str, px: f32) -> (u32, u32, Vec<u8>) {
        match self {
            BuildUiFont::Real(f) => rasterize_caption_line_bgra(f, text, px),
            BuildUiFont::Placeholder => rasterize_caption_placeholder(text),
        }
    }

    fn menu(&self, text: &str, px: f32) -> (u32, u32, Vec<u8>) {
        match self {
            BuildUiFont::Real(f) => rasterize_menu_label_line_bgra(f, text, px),
            BuildUiFont::Placeholder => rasterize_menu_placeholder(text),
        }
    }

    fn glyph8(&self, ch: char) -> [u8; 8] {
        match self {
            BuildUiFont::Real(f) => glyph_mask_8x8(f, ch),
            BuildUiFont::Placeholder => glyph_mask_8x8_placeholder(ch),
        }
    }
}

/// Readable-ish fallback when no `.ttf` is vendored (clone-friendly builds).
fn rasterize_caption_placeholder(text: &str) -> (u32, u32, Vec<u8>) {
    let h = 22u32;
    let w = ((text.chars().count() as u32).saturating_mul(9)).max(24);
    let mut buf = vec![0u8; (w * h * 4) as usize];
    for px in 0..w {
        for py in 0..h {
            let o = ((py * w + px) * 4) as usize;
            buf[o] = 0x1c;
            buf[o + 1] = 0x18;
            buf[o + 2] = 0x18;
            buf[o + 3] = 0xff;
        }
    }
    for (i, _) in text.chars().enumerate() {
        let base_x = 4u32.saturating_add((i as u32).saturating_mul(9));
        if base_x.saturating_add(6) >= w {
            break;
        }
        for cx in base_x..base_x + 6 {
            for cy in 6..14 {
                let o = ((cy * w + cx) * 4) as usize;
                buf[o] = 0xf0;
                buf[o + 1] = 0xf0;
                buf[o + 2] = 0xf8;
                buf[o + 3] = 0xff;
            }
        }
    }
    (w, h, buf)
}

fn rasterize_menu_placeholder(text: &str) -> (u32, u32, Vec<u8>) {
    let h = 20u32;
    let w = ((text.chars().count() as u32).saturating_mul(8)).max(20);
    let mut buf = vec![0u8; (w * h * 4) as usize];
    for px in 0..w {
        for py in 0..h {
            let o = ((py * w + px) * 4) as usize;
            buf[o] = 0x28;
            buf[o + 1] = 0x28;
            buf[o + 2] = 0x30;
            buf[o + 3] = 0xff;
        }
    }
    for (i, _) in text.chars().enumerate() {
        let base_x = 3u32.saturating_add((i as u32).saturating_mul(8));
        if base_x.saturating_add(5) >= w {
            break;
        }
        for cx in base_x..base_x + 5 {
            for cy in 5..13 {
                let o = ((cy * w + cx) * 4) as usize;
                buf[o] = 0xfc;
                buf[o + 1] = 0xfc;
                buf[o + 2] = 0xfc;
                buf[o + 3] = 0xff;
            }
        }
    }
    (w, h, buf)
}

fn glyph_mask_8x8_placeholder(_ch: char) -> [u8; 8] {
    [0x10, 0x10, 0x10, 0xFE, 0x10, 0x10, 0x10, 0x10]
}

fn rasterize_caption_line_bgra(font: &fontdue::Font, text: &str, px: f32) -> (u32, u32, Vec<u8>) {
    let baseline_y: i32 = 24;
    struct GlyphPlaced {
        gx: i32,
        gy: i32,
        w: usize,
        h: usize,
        bmp: Vec<u8>,
    }
    let mut glyphs: Vec<GlyphPlaced> = Vec::new();
    let mut pen = 0.0f32;
    for c in text.chars() {
        let (metrics, bitmap) = font.rasterize(c, px);
        let gx = (pen + metrics.xmin as f32).floor() as i32;
        let gy = baseline_y + metrics.ymin;
        pen += metrics.advance_width;
        glyphs.push(GlyphPlaced {
            gx,
            gy,
            w: metrics.width,
            h: metrics.height,
            bmp: bitmap,
        });
    }
    if glyphs.is_empty() {
        return (1, 1, vec![0, 0, 0, 0]);
    }
    let mut min_x = i32::MAX;
    let mut min_y = i32::MAX;
    let mut max_x = i32::MIN;
    let mut max_y = i32::MIN;
    for g in &glyphs {
        min_x = min_x.min(g.gx);
        min_y = min_y.min(g.gy);
        max_x = max_x.max(g.gx + g.w as i32);
        max_y = max_y.max(g.gy + g.h as i32);
    }
    let pad = 1i32;
    min_x -= pad;
    min_y -= pad;
    max_x += pad;
    max_y += pad;
    let w = (max_x - min_x).max(1) as u32;
    let h = (max_y - min_y).max(1) as u32;
    let mut buf = vec![0u8; (w * h * 4) as usize];
    let (br, bg, bb) = (0x18u8, 0x18u8, 0x1cu8);
    for g in glyphs {
        for row in 0..g.h {
            for col in 0..g.w {
                let a = g.bmp[row * g.w + col];
                if a == 0 {
                    continue;
                }
                let px = g.gx - min_x + col as i32;
                let py = g.gy - min_y + row as i32;
                if px < 0 || py < 0 {
                    continue;
                }
                let px = px as u32;
                let py = py as u32;
                if px >= w || py >= h {
                    continue;
                }
                let o = ((py * w + px) * 4) as usize;
                buf[o] = bb;
                buf[o + 1] = bg;
                buf[o + 2] = br;
                buf[o + 3] = a;
            }
        }
    }
    (w, h, buf)
}

/// Light foreground for menu rows (readable on gray `#363640` and blue `#0078d7`).
fn rasterize_menu_label_line_bgra(font: &fontdue::Font, text: &str, px: f32) -> (u32, u32, Vec<u8>) {
    let baseline_y: i32 = 24;
    struct GlyphPlaced {
        gx: i32,
        gy: i32,
        w: usize,
        h: usize,
        bmp: Vec<u8>,
    }
    let mut glyphs: Vec<GlyphPlaced> = Vec::new();
    let mut pen = 0.0f32;
    for c in text.chars() {
        let (metrics, bitmap) = font.rasterize(c, px);
        let gx = (pen + metrics.xmin as f32).floor() as i32;
        let gy = baseline_y + metrics.ymin;
        pen += metrics.advance_width;
        glyphs.push(GlyphPlaced {
            gx,
            gy,
            w: metrics.width,
            h: metrics.height,
            bmp: bitmap,
        });
    }
    if glyphs.is_empty() {
        return (1, 1, vec![0, 0, 0, 0]);
    }
    let mut min_x = i32::MAX;
    let mut min_y = i32::MAX;
    let mut max_x = i32::MIN;
    let mut max_y = i32::MIN;
    for g in &glyphs {
        min_x = min_x.min(g.gx);
        min_y = min_y.min(g.gy);
        max_x = max_x.max(g.gx + g.w as i32);
        max_y = max_y.max(g.gy + g.h as i32);
    }
    let pad = 1i32;
    min_x -= pad;
    min_y -= pad;
    max_x += pad;
    max_y += pad;
    let w = (max_x - min_x).max(1) as u32;
    let h = (max_y - min_y).max(1) as u32;
    let mut buf = vec![0u8; (w * h * 4) as usize];
    let (br, bg, bb) = (0xf8u8, 0xf8u8, 0xfcu8);
    for g in glyphs {
        for row in 0..g.h {
            for col in 0..g.w {
                let a = g.bmp[row * g.w + col];
                if a == 0 {
                    continue;
                }
                let px = g.gx - min_x + col as i32;
                let py = g.gy - min_y + row as i32;
                if px < 0 || py < 0 {
                    continue;
                }
                let px = px as u32;
                let py = py as u32;
                if px >= w || py >= h {
                    continue;
                }
                let o = ((py * w + px) * 4) as usize;
                buf[o] = bb;
                buf[o + 1] = bg;
                buf[o + 2] = br;
                buf[o + 3] = a;
            }
        }
    }
    (w, h, buf)
}

/// 8×8 monospace-style bitmask for runtime ASCII (`generated_ascii_font.rs`); `lib` = Libertinus.
fn glyph_mask_8x8(lib: &fontdue::Font, ch: char) -> [u8; 8] {
    let (metrics, bitmap) = lib.rasterize(ch, 7.5_f32);
    let w = metrics.width;
    let h = metrics.height;
    let mut rows = [0u8; 8];
    if w == 0 || h == 0 {
        return rows;
    }
    let xmin = metrics.xmin as i32;
    let ymin = metrics.ymin as i32;
    for by in 0..h {
        for bx in 0..w {
            let a = bitmap[by * w + bx];
            if a < 96 {
                continue;
            }
            let sx = bx as i32 + xmin;
            let sy = by as i32 + ymin;
            // Map into 8×8 cell (top-left origin, y down).
            let tx = sx + 1;
            let ty = sy + 2;
            if (0..8).contains(&tx) && (0..8).contains(&ty) {
                rows[ty as usize] |= 1u8 << (7 - tx as usize);
            }
        }
    }
    rows
}

fn build_cursor_bgra_32() -> Vec<u8> {
    /// 16×16 arrow mask (MSB = left). High-contrast fill so the pointer is visible on any wallpaper
    /// (Win32-style client-area pointer concept: `references/win32/.../mouse-movement.md`).
    const MASK: [u16; 16] = [
        0x8000, 0xC000, 0xA000, 0x9000, 0x8800, 0x8400, 0x8200, 0x8100, 0x8080, 0x8040, 0x8020,
        0x8010, 0x9FF8, 0xB770, 0xC1E0, 0x81C0,
    ];
    #[inline]
    fn mask_on(sr: usize, sc: usize) -> bool {
        sr < 16 && sc < 16 && (MASK[sr] >> (15 - sc)) & 1 != 0
    }
    #[inline]
    fn mask_on_i(nr: i32, nc: i32) -> bool {
        nr >= 0 && nc >= 0 && mask_on(nr as usize, nc as usize)
    }
    let mut v = vec![0u8; 32 * 32 * 4];
    for row in 0..32u32 {
        for col in 0..32u32 {
            let sr = (row / 2) as usize;
            let sc = (col / 2) as usize;
            if !mask_on(sr, sc) {
                let mut border = false;
                for dr in -1i32..=1 {
                    for dc in -1i32..=1 {
                        if dr == 0 && dc == 0 {
                            continue;
                        }
                        let nr = sr as i32 + dr;
                        let nc = sc as i32 + dc;
                        if mask_on_i(nr, nc) {
                            border = true;
                            break;
                        }
                    }
                    if border {
                        break;
                    }
                }
                if !border {
                    continue;
                }
                // Outer outline: dark gray (pure black reads as a solid block on some panels).
                let i = ((row * 32 + col) * 4) as usize;
                v[i] = 0x55;
                v[i + 1] = 0x55;
                v[i + 2] = 0x55;
                v[i + 3] = 0xff;
                continue;
            }
            let mut edge = false;
            for dr in -1i32..=1 {
                for dc in -1i32..=1 {
                    if dr == 0 && dc == 0 {
                        continue;
                    }
                    let nr = sr as i32 + dr;
                    let nc = sc as i32 + dc;
                    if nr < 0
                        || nc < 0
                        || nr >= 16
                        || nc >= 16
                        || (MASK[nr as usize] >> (15 - nc as usize)) & 1 == 0
                    {
                        edge = true;
                        break;
                    }
                }
                if edge {
                    break;
                }
            }
            // White rim + saturated red fill (stronger mid-tone for visibility on busy wallpapers).
            let (b, g, r) = if edge {
                (0xffu8, 0xffu8, 0xffu8)
            } else {
                (0x00u8, 0x28u8, 0xffu8)
            };
            let i = ((row * 32 + col) * 4) as usize;
            v[i] = b;
            v[i + 1] = g;
            v[i + 2] = r;
            v[i + 3] = 0xff;
        }
    }
    // Second-pass: thicken outer black ring (2px feel) for software cursor on photo wallpapers.
    let mut out = v;
    for row in 1..31u32 {
        for col in 1..31u32 {
            let sr = (row / 2) as usize;
            let sc = (col / 2) as usize;
            if mask_on(sr, sc) {
                continue;
            }
            let mut near = false;
            for dr in -1i32..=1 {
                for dc in -1i32..=1 {
                    let nr = row as i32 + dr;
                    let nc = col as i32 + dc;
                    if nr < 0 || nc < 0 || nr >= 32 || nc >= 32 {
                        continue;
                    }
                    let i = ((nr as u32 * 32 + nc as u32) * 4) as usize;
                    if out[i + 3] >= 0xe0
                        && out[i] < 0x40
                        && out[i + 1] < 0x40
                        && out[i + 2] < 0x40
                    {
                        near = true;
                        break;
                    }
                }
                if near {
                    break;
                }
            }
            if near {
                let i = ((row * 32 + col) * 4) as usize;
                if out[i + 3] < 0x10 {
                    out[i] = 0x4a;
                    out[i + 1] = 0x4a;
                    out[i + 2] = 0x4a;
                    out[i + 3] = 0xff;
                }
            }
        }
    }
    out
}

fn main() {
    let out_dir_s = std::env::var("OUT_DIR").expect("OUT_DIR");
    let out_dir = Path::new(&out_dir_s);
    let root = repo_root();

    let (w, h, bgra) = if let Some(png) = pick_wallpaper_png() {
        println!("cargo:rerun-if-changed={}", png.display());
        png_to_bgra_max(&png, 1920, 1080)
    } else {
        let w = 320u32;
        let h = 200u32;
        (w, h, gradient_bgra(w, h))
    };

    let bin_path = out_dir.join("wallpaper.bgra");
    std::fs::write(&bin_path, &bgra).expect("write wallpaper.bgra");

    let gen_wp = out_dir.join("generated_wallpaper.rs");
    let rs_wp = format!(
        r#"// @generated by nt10-kernel/build.rs
pub const DEFAULT_WALLPAPER_WIDTH: u32 = {w};
pub const DEFAULT_WALLPAPER_HEIGHT: u32 = {h};
pub static DEFAULT_WALLPAPER_BGRA: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/wallpaper.bgra"));
"#
    );
    std::fs::write(&gen_wp, rs_wp).expect("write generated_wallpaper.rs");

    println!("cargo:rerun-if-changed={}", root.join("resources/manifest.json").display());
    let mut icon_rs = String::from(
        "// @generated by nt10-kernel/build.rs — Start menu + Explorer + desktop shortcuts (BGRA8)\n",
    );
    icon_rs.push_str("pub const START_MENU_ICON_W: u32 = 32;\n");
    icon_rs.push_str("pub const START_MENU_ICON_H: u32 = 32;\n");

    for (i, id) in START_MENU_ICON_IDS.iter().enumerate() {
        let path = manifest_asset_path(&root, id);
        println!("cargo:rerun-if-changed={}", path.display());
        let (iw, ih, bytes) = png_to_bgra_max(&path, 32, 32);
        if iw != 32 || ih != 32 {
            panic!(
                "start menu icon {} must rasterize to 32×32, got {}×{}",
                id, iw, ih
            );
        }
        let fname = format!("start_menu_icon_{i}.bgra");
        std::fs::write(out_dir.join(&fname), &bytes).expect("write icon bgra");
        icon_rs.push_str(&format!(
            "pub static START_MENU_ICON_{i}_BGRA: &[u8] = include_bytes!(concat!(env!(\"OUT_DIR\"), \"/{fname}\"));\n"
        ));
    }

    for (i, id) in EXPLORER_ROW_ICON_IDS.iter().enumerate() {
        let path = manifest_asset_path(&root, id);
        println!("cargo:rerun-if-changed={}", path.display());
        let (iw, ih, bytes) = png_to_bgra_max(&path, 32, 32);
        if iw != 32 || ih != 32 {
            panic!("explorer icon {} must be 32×32, got {}×{}", id, iw, ih);
        }
        let fname = format!("explorer_row_icon_{i}.bgra");
        std::fs::write(out_dir.join(&fname), &bytes).expect("write explorer icon");
        icon_rs.push_str(&format!(
            "pub static EXPLORER_ROW_ICON_{i}_BGRA: &[u8] = include_bytes!(concat!(env!(\"OUT_DIR\"), \"/{fname}\"));\n"
        ));
    }

    let ndesk = DESKTOP_SHORTCUT_ICON_IDS.len();
    icon_rs.push_str(&format!("pub const DESKTOP_ICON_COUNT: usize = {ndesk};\n"));
    for (i, id) in DESKTOP_SHORTCUT_ICON_IDS.iter().enumerate() {
        let path = manifest_asset_path(&root, id);
        println!("cargo:rerun-if-changed={}", path.display());
        let (iw, ih, bytes) = png_to_bgra_max(&path, 32, 32);
        if iw != 32 || ih != 32 {
            panic!("desktop icon {} must be 32×32, got {}×{}", id, iw, ih);
        }
        let fname = format!("desktop_icon_{i}.bgra");
        std::fs::write(out_dir.join(&fname), &bytes).expect("write desktop icon");
        icon_rs.push_str(&format!(
            "pub static DESKTOP_ICON_{i}_BGRA: &[u8] = include_bytes!(concat!(env!(\"OUT_DIR\"), \"/{fname}\"));\n"
        ));
    }

    println!(
        "cargo:rerun-if-changed={}",
        root.join("third_party/fonts/licenses/OFL-NotoSans.txt").display()
    );
    let require_font = std::env::var("NT10_KERNEL_REQUIRE_OFL_FONT")
        .map(|v| v == "1")
        .unwrap_or(false);
    let ui_font: BuildUiFont = match find_ui_sans_bytes(&root) {
        Some(bytes) => {
            match fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default()) {
                Ok(f) => BuildUiFont::Real(f),
                Err(e) if require_font => panic!("fontdue parse UI font: {e}"),
                Err(_) => {
                    println!("cargo:warning=nt10-kernel: fontdue rejected UI font bytes — using placeholders");
                    BuildUiFont::Placeholder
                }
            }
        }
        None => {
            if require_font {
                panic!(
                    "NT10_KERNEL_REQUIRE_OFL_FONT=1 but no TTF under third_party/fonts/latin/ — run ./scripts/fetch-ofl-fonts.sh"
                );
            }
            println!("cargo:warning=nt10-kernel: no OFL UI font found — placeholder text rasters (run ./scripts/fetch-ofl-fonts.sh)");
            BuildUiFont::Placeholder
        }
    };

    assert_eq!(
        START_MENU_LABEL_TEXTS.len(),
        START_MENU_ICON_IDS.len(),
        "start menu label vs icon count"
    );
    let nsm = START_MENU_ICON_IDS.len();
    icon_rs.push_str(&format!(
        "pub const START_MENU_ROW_COUNT: usize = {nsm};\n\
         pub const START_MENU_ICON_COUNT: usize = {nsm};\n"
    ));

    let cap_px = 17.0f32;
    assert_eq!(
        DESKTOP_CAPTION_TEXTS.len(),
        DESKTOP_SHORTCUT_ICON_IDS.len(),
        "caption count must match desktop icon count"
    );
    icon_rs.push_str(&format!(
        "pub const DESKTOP_CAPTION_COUNT: usize = {};\n",
        DESKTOP_CAPTION_TEXTS.len()
    ));
    let mut max_cap_h = 0u32;
    for (i, text) in DESKTOP_CAPTION_TEXTS.iter().enumerate() {
        let (cw, ch, bytes) = ui_font.caption(text, cap_px);
        max_cap_h = max_cap_h.max(ch);
        let fname = format!("desktop_caption_{i}.bgra");
        std::fs::write(out_dir.join(&fname), &bytes).expect("write caption bgra");
        icon_rs.push_str(&format!(
            "pub const DESKTOP_CAPTION_{i}_W: u32 = {cw};\n\
             pub const DESKTOP_CAPTION_{i}_H: u32 = {ch};\n\
             pub static DESKTOP_CAPTION_{i}_BGRA: &[u8] = include_bytes!(concat!(env!(\"OUT_DIR\"), \"/{fname}\"));\n"
        ));
    }
    icon_rs.push_str(&format!("pub const DESKTOP_CAPTION_MAX_H: u32 = {max_cap_h};\n"));

    let menu_row_px = 15.0f32;
    for (i, text) in START_MENU_LABEL_TEXTS.iter().enumerate() {
        let (cw, ch, bytes) = ui_font.menu(text, menu_row_px);
        let fname = format!("start_menu_label_{i}.bgra");
        std::fs::write(out_dir.join(&fname), &bytes).expect("write start menu label bgra");
        icon_rs.push_str(&format!(
            "pub const START_MENU_LABEL_{i}_W: u32 = {cw};\n\
             pub const START_MENU_LABEL_{i}_H: u32 = {ch};\n\
             pub static START_MENU_LABEL_{i}_BGRA: &[u8] = include_bytes!(concat!(env!(\"OUT_DIR\"), \"/{fname}\"));\n"
        ));
    }

    assert_eq!(
        EXPLORER_ROW_LABEL_TEXTS.len(),
        6,
        "explorer row labels vs explorer_view::STATIC_ENTRY_COUNT"
    );
    icon_rs.push_str(&format!(
        "pub const EXPLORER_ROW_LABEL_COUNT: usize = {};\n",
        EXPLORER_ROW_LABEL_TEXTS.len()
    ));
    let ex_px = 13.0f32;
    for (i, text) in EXPLORER_ROW_LABEL_TEXTS.iter().enumerate() {
        let (cw, ch, bytes) = ui_font.menu(text, ex_px);
        let fname = format!("explorer_row_label_{i}.bgra");
        std::fs::write(out_dir.join(&fname), &bytes).expect("write explorer row label bgra");
        icon_rs.push_str(&format!(
            "pub const EXPLORER_ROW_LABEL_{i}_W: u32 = {cw};\n\
             pub const EXPLORER_ROW_LABEL_{i}_H: u32 = {ch};\n\
             pub static EXPLORER_ROW_LABEL_{i}_BGRA: &[u8] = include_bytes!(concat!(env!(\"OUT_DIR\"), \"/{fname}\"));\n"
        ));
    }

    icon_rs.push_str(&format!(
        "pub const WINDOW_TITLE_COUNT: usize = {};\n",
        WINDOW_TITLE_TEXTS.len()
    ));
    let title_px = 14.0f32;
    for (i, text) in WINDOW_TITLE_TEXTS.iter().enumerate() {
        let (cw, ch, bytes) = ui_font.menu(text, title_px);
        let fname = format!("window_title_{i}.bgra");
        std::fs::write(out_dir.join(&fname), &bytes).expect("write window title bgra");
        icon_rs.push_str(&format!(
            "pub const WINDOW_TITLE_{i}_W: u32 = {cw};\n\
             pub const WINDOW_TITLE_{i}_H: u32 = {ch};\n\
             pub static WINDOW_TITLE_{i}_BGRA: &[u8] = include_bytes!(concat!(env!(\"OUT_DIR\"), \"/{fname}\"));\n"
        ));
    }

    let menu_px = 17.0f32;
    icon_rs.push_str("pub const CONTEXT_MENU_LABEL_COUNT: usize = 3;\n");
    for (i, text) in CONTEXT_MENU_LABEL_TEXTS.iter().enumerate() {
        let (cw, ch, bytes) = ui_font.menu(text, menu_px);
        let fname = format!("context_menu_label_{i}.bgra");
        std::fs::write(out_dir.join(&fname), &bytes).expect("write context menu label bgra");
        icon_rs.push_str(&format!(
            "pub const CONTEXT_MENU_LABEL_{i}_W: u32 = {cw};\n\
             pub const CONTEXT_MENU_LABEL_{i}_H: u32 = {ch};\n\
             pub static CONTEXT_MENU_LABEL_{i}_BGRA: &[u8] = include_bytes!(concat!(env!(\"OUT_DIR\"), \"/{fname}\"));\n"
        ));
    }

    let mut ascii_rs = String::from("// @generated by nt10-kernel/build.rs — 8×8 bitmask rows for ASCII 32..=126 (OFL font or placeholder)\n");
    ascii_rs.push_str("pub const FONT_GLYPH_COUNT: usize = 95;\n");
    ascii_rs.push_str("pub static FONT_GLYPH_MASKS: [[u8; 8]; 95] = [\n");
    for cp in 32u8..=126u8 {
        let rows = ui_font.glyph8(cp as char);
        ascii_rs.push_str(&format!(
            "    [{},{},{},{},{},{},{},{}],\n",
            rows[0], rows[1], rows[2], rows[3], rows[4], rows[5], rows[6], rows[7]
        ));
    }
    ascii_rs.push_str("];\n");
    let gen_ascii = out_dir.join("generated_ascii_font.rs");
    std::fs::write(&gen_ascii, ascii_rs).expect("write generated_ascii_font.rs");

    let cursor_bgra = build_cursor_bgra_32();
    std::fs::write(out_dir.join("pointer_cursor.bgra"), &cursor_bgra).expect("cursor bgra");
    let gen_cursor = out_dir.join("generated_cursor.rs");
    std::fs::write(
        &gen_cursor,
        r#"// @generated by nt10-kernel/build.rs
/// Hotspot in client / framebuffer space; sprite top-left = (hotspot_x - POINTER_HOTSPOT_X, ...).
pub const POINTER_HOTSPOT_X: u32 = 0;
pub const POINTER_HOTSPOT_Y: u32 = 0;
pub const POINTER_CURSOR_W: u32 = 32;
pub const POINTER_CURSOR_H: u32 = 32;
pub static POINTER_CURSOR_BGRA: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/pointer_cursor.bgra"));
"#,
    )
    .expect("write generated_cursor.rs");

    let gen_icons = out_dir.join("generated_icons.rs");
    std::fs::write(&gen_icons, icon_rs).expect("write generated_icons.rs");

    println!("cargo:rerun-if-changed={}", gen_cursor.display());
    println!("cargo:rerun-if-changed={}", gen_ascii.display());
    println!("cargo:rerun-if-changed=build.rs");
    let wp_pref = root.join("resources/wallpapers/wallpaper-countryside_dawn-1080p.png");
    println!("cargo:rerun-if-changed={}", wp_pref.display());
}
