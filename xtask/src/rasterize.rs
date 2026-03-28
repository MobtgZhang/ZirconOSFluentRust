//! Rasterize ZirconOS Fluent SVG assets to PNG for `resources/manifest.json` and kernel `build.rs`.

use std::fs;
use std::path::Path;

const ICON_SIZES: [u32; 5] = [16, 24, 32, 48, 256];

pub fn run(repo_root: &Path) -> Result<(), String> {
    let res = repo_root.join("resources");
    let icons_src = res.join("icons");
    let wp_src = res.join("wallpapers");

    if !icons_src.is_dir() {
        return Err(format!("missing {}", icons_src.display()));
    }

    for entry in fs::read_dir(&icons_src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("svg") {
            continue;
        }
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| format!("bad file name {}", path.display()))?;
        let out_dir = icons_src.join(stem);
        fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;
        for &sz in &ICON_SIZES {
            let out = out_dir.join(format!("icon-{sz}.png"));
            rasterize_svg_to_png(&path, sz, sz, &out)?;
        }
    }

    // Start menu "Power" row uses store-style glyph; mirror `store.svg` into `icons/power/`.
    let store_svg = icons_src.join("store.svg");
    if store_svg.is_file() {
        let power_dir = icons_src.join("power");
        fs::create_dir_all(&power_dir).map_err(|e| e.to_string())?;
        for &sz in &ICON_SIZES {
            let out = power_dir.join(format!("icon-{sz}.png"));
            rasterize_svg_to_png(&store_svg, sz, sz, &out)?;
        }
    }

    if wp_src.is_dir() {
        for entry in fs::read_dir(&wp_src).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("svg") {
                continue;
            }
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .ok_or_else(|| format!("bad wallpaper name {}", path.display()))?;
            let wp_out = wp_src.join(format!("wallpaper-{stem}-1080p.png"));
            rasterize_svg_to_png(&path, 1920, 1080, &wp_out)?;
        }
    }

    eprintln!("rasterize: wrote PNGs under resources/icons/*/ and resources/wallpapers/wallpaper-*-1080p.png");
    Ok(())
}

fn rasterize_svg_to_png(svg_path: &Path, out_w: u32, out_h: u32, png_path: &Path) -> Result<(), String> {
    let svg_data = fs::read(svg_path).map_err(|e| format!("read {}: {e}", svg_path.display()))?;
    let mut opt = resvg::usvg::Options::default();
    opt.resources_dir = svg_path.parent().map(Path::to_path_buf);
    let tree =
        resvg::usvg::Tree::from_data(&svg_data, &opt).map_err(|e| format!("usvg {}: {e}", svg_path.display()))?;

    let sz = tree.size();
    let (vw, vh) = (sz.width(), sz.height());
    if vw <= 0.0 || vh <= 0.0 {
        return Err(format!("invalid svg size: {}", svg_path.display()));
    }
    let scale = (out_w as f32 / vw).min(out_h as f32 / vh);
    let transform = tiny_skia::Transform::from_scale(scale, scale);

    let mut pixmap = tiny_skia::Pixmap::new(out_w, out_h).ok_or_else(|| "pixmap alloc".to_string())?;
    pixmap.fill(tiny_skia::Color::TRANSPARENT);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    let rgba = pixmap.data();
    let mut img = image::RgbaImage::new(out_w, out_h);
    for y in 0..out_h {
        for x in 0..out_w {
            let i = ((y * out_w + x) * 4) as usize;
            let r = rgba[i];
            let g = rgba[i + 1];
            let b = rgba[i + 2];
            let a = rgba[i + 3];
            img.put_pixel(x, y, image::Rgba([r, g, b, a]));
        }
    }
    img.save(png_path)
        .map_err(|e| format!("write {}: {e}", png_path.display()))?;
    Ok(())
}

