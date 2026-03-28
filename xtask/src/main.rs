//! Workspace tasks: build targets, pack FAT ESP, launch QEMU (see `scripts/`).

mod rasterize;

use std::env;
use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask at repo root /xtask")
        .to_path_buf()
}

fn print_help() {
    eprintln!(
        "\
ZirconOS xtask — dev helpers

Usage:
  cargo run -p xtask -- <command> [args...]

Commands:
  build [--release]     Build nt10-boot-uefi (UEFI) + nt10-kernel-bin (kernel)
  rasterize-resources   Rasterize resources/icons/*.svg and default wallpaper SVG to PNG (run after syncing Fluent assets)
  pack-esp [--release] <dir>   Populate ESP tree (calls scripts/pack-esp.sh)
  qemu [--release] [-- <qemu-args>]   scripts/run-qemu-x86_64.sh (temp ESP uses PROFILE)
  qemu-kernel [-- <qemu-args>]   scripts/run-qemu-kernel.sh (direct -kernel ELF)

Environment (qemu):
  OVMF_CODE, OVMF_VARS, ZBM10_ESP — see scripts/run-qemu-x86_64.sh

Environment (pack-esp / qemu temp ESP):
  PROFILE=release — same as passing --release to pack-esp or qemu
"
    );
}

fn has_flag(args: &[String], flag: &str) -> (bool, Vec<String>) {
    let mut out = Vec::new();
    let mut found = false;
    for a in args {
        if a == flag {
            found = true;
        } else {
            out.push(a.clone());
        }
    }
    (found, out)
}

fn main() {
    let mut args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() || args[0] == "-h" || args[0] == "--help" {
        print_help();
        return;
    }

    let cmd = args.remove(0);
    let root = repo_root();

    match cmd.as_str() {
        "rasterize-resources" => {
            if !args.is_empty() {
                eprintln!("rasterize-resources takes no arguments");
                std::process::exit(2);
            }
            if let Err(e) = rasterize::run(&root) {
                eprintln!("rasterize-resources: {e}");
                std::process::exit(1);
            }
        }
        "build" => {
            let (release, rest) = has_flag(&args, "--release");
            if !rest.is_empty() {
                eprintln!("unexpected args: {rest:?}");
                std::process::exit(2);
            }
            let mut c = Command::new("cargo");
            c.arg("build")
                .arg("-p")
                .arg("nt10-boot-uefi")
                .arg("--target")
                .arg("x86_64-unknown-uefi")
                .current_dir(&root);
            if release {
                c.arg("--release");
            }
            let s = c.status().expect("cargo");
            if !s.success() {
                std::process::exit(s.code().unwrap_or(1));
            }
            let mut c2 = Command::new("cargo");
            c2.arg("build")
                .arg("-p")
                .arg("nt10-kernel-bin")
                .arg("--target")
                .arg("x86_64-unknown-none")
                .current_dir(&root);
            if release {
                c2.arg("--release");
            }
            let s2 = c2.status().expect("cargo");
            std::process::exit(s2.code().unwrap_or(0));
        }
        "pack-esp" => {
            let (release, mut rest) = has_flag(&args, "--release");
            if rest.len() != 1 {
                eprintln!("usage: pack-esp [--release] <esp-directory>");
                std::process::exit(2);
            }
            let dest = rest.pop().unwrap();
            let mut cmd = Command::new("bash");
            cmd.arg(root.join("scripts/pack-esp.sh"))
                .arg(&dest)
                .current_dir(&root);
            if release {
                cmd.env("PROFILE", "release");
            }
            let s = cmd.status().expect("pack-esp");
            std::process::exit(s.code().unwrap_or(0));
        }
        "qemu" => {
            let (release, rest) = has_flag(&args, "--release");
            let (qemu_args, trailing) = split_at_double_dash(rest);
            if !trailing.is_empty() {
                eprintln!("unexpected before --: {trailing:?}");
                std::process::exit(2);
            }
            let mut c = Command::new("bash");
            c.arg(root.join("scripts/run-qemu-x86_64.sh"))
                .args(&qemu_args)
                .current_dir(&root);
            if release {
                c.env("PROFILE", "release");
            }
            let s = c.status().expect("qemu");
            std::process::exit(s.code().unwrap_or(0));
        }
        "qemu-kernel" => {
            let (qemu_args, trailing) = split_at_double_dash(args);
            if !trailing.is_empty() {
                eprintln!("unexpected before --: {trailing:?}");
                std::process::exit(2);
            }
            let mut c = Command::new("bash");
            c.arg(root.join("scripts/run-qemu-kernel.sh"))
                .args(&qemu_args)
                .current_dir(&root);
            let s = c.status().expect("qemu-kernel");
            std::process::exit(s.code().unwrap_or(0));
        }
        _ => {
            eprintln!("unknown command: {cmd}");
            print_help();
            std::process::exit(2);
        }
    }
}

fn split_at_double_dash(args: Vec<String>) -> (Vec<String>, Vec<String>) {
    let pos = args.iter().position(|a| a == "--");
    match pos {
        Some(i) => {
            let after = args[i + 1..].to_vec();
            let before = args[..i].to_vec();
            (after, before)
        }
        None => (args, vec![]),
    }
}
