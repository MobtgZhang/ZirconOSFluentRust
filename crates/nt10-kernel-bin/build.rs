use std::path::Path;

fn main() {
    let target = std::env::var("TARGET").unwrap_or_default();
    if target != "x86_64-unknown-none" {
        return;
    }

    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let ld = manifest_dir
        .join("../../link/x86_64-uefi-load.ld")
        .canonicalize()
        .expect("link/x86_64-uefi-load.ld missing");
    println!("cargo:rerun-if-changed={}", ld.display());
    println!("cargo:rustc-link-arg=-T{}", ld.display());
}
