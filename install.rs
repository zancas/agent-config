#!/usr/bin/env rust-script
//! Installer for this agent-config repo.
//!
//! Agent-agnostic global instructions live in `shared/AGENTS.md`. Per-agent
//! wiring lives under `agents/<name>/`. This installer symlinks the shared
//! instructions and each agent's config into place.
//!
//! Currently supports the "claude" agent (Claude Code). To support another
//! agent, add an `agents/<name>/` dir and a matching `install_<name>` arm in
//! `main`.
//!
//! Run from the repo root:
//!     ./install.rs
//!
//! Requires: rust-script (to run this), git, and a Rust toolchain (cargo).
//! std-only — no external crates — so it builds fast on a fresh machine.
#![forbid(unsafe_code)]
use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::Command;

fn home() -> PathBuf {
    PathBuf::from(std::env::var("HOME").expect("HOME is not set"))
}

/// Pick a non-clobbering backup path: foo.bak, then foo.bak.1, foo.bak.2, ...
fn backup_path(p: &Path) -> PathBuf {
    let first = PathBuf::from(format!("{}.bak", p.display()));
    if !first.exists() {
        return first;
    }
    let mut n = 1;
    loop {
        let cand = PathBuf::from(format!("{}.bak.{n}", p.display()));
        if !cand.exists() {
            return cand;
        }
        n += 1;
    }
}

/// Create `link_at` -> `target`. If `link_at` is an existing symlink it is
/// replaced; an existing real file/dir is moved aside to a `.bak` path first.
fn link(target: &Path, link_at: &Path) {
    if let Some(parent) = link_at.parent() {
        fs::create_dir_all(parent).expect("create parent dir");
    }
    if let Ok(meta) = fs::symlink_metadata(link_at) {
        if meta.file_type().is_symlink() {
            fs::remove_file(link_at).expect("remove old symlink");
        } else {
            let bak = backup_path(link_at);
            println!("  backing up {} -> {}", link_at.display(), bak.display());
            fs::rename(link_at, &bak).expect("back up existing path");
        }
    }
    symlink(target, link_at).expect("create symlink");
    println!("  linked {} -> {}", link_at.display(), target.display());
}

/// Run a command, returning true on success. Streams output through.
fn run(cmd: &str, args: &[&str]) -> bool {
    Command::new(cmd)
        .args(args)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Is a binary runnable (e.g. `topiary --version` succeeds)?
fn have(cmd: &str) -> bool {
    Command::new(cmd)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Ensure ripgrep (`rg`) is installed — preferred over grep everywhere.
/// Agent-agnostic, so this runs regardless of which agents are configured.
fn ensure_ripgrep() {
    println!("Ensuring ripgrep (rg) is installed ...");
    if have("rg") {
        println!("  ripgrep already installed");
    } else if have("cargo") {
        if !run("cargo", &["install", "ripgrep", "--locked"]) {
            eprintln!("  warning: `cargo install ripgrep` failed");
        }
    } else {
        eprintln!("  warning: cargo not found; install Rust + ripgrep for `rg`");
    }
}

/// Install the Claude Code (claude) agent: settings, hooks, shared
/// instructions (as CLAUDE.md), grill skills, and the Topiary formatter.
fn install_claude(repo: &Path) {
    let agent = repo.join("agents/claude");
    let shared_instructions = repo.join("shared/AGENTS.md");
    let claude = home().join(".claude");
    fs::create_dir_all(&claude).expect("create ~/.claude");

    println!("[claude] linking config into {} ...", claude.display());
    link(&agent.join("settings.json"), &claude.join("settings.json"));
    link(&agent.join("hooks"), &claude.join("hooks"));
    // Claude reads CLAUDE.md; point it at the shared, agent-agnostic instructions.
    link(&shared_instructions, &claude.join("CLAUDE.md"));

    // Grill skills: cloned from upstream, symlinked into ~/.claude/skills.
    println!("[claude] setting up grill skills from mattpocock/skills ...");
    let skills_src = home().join("src/mattpocock-skills");
    if skills_src.exists() {
        run("git", &["-C", skills_src.to_str().unwrap(), "pull", "--ff-only"]);
    } else {
        fs::create_dir_all(skills_src.parent().unwrap()).ok();
        if !run(
            "git",
            &[
                "clone",
                "https://github.com/mattpocock/skills.git",
                skills_src.to_str().unwrap(),
            ],
        ) {
            eprintln!("  warning: failed to clone mattpocock/skills; skills not linked");
        }
    }
    if skills_src.exists() {
        link(
            &skills_src.join("skills/productivity/grill-me"),
            &claude.join("skills/grill-me"),
        );
        link(
            &skills_src.join("skills/engineering/grill-with-docs"),
            &claude.join("skills/grill-with-docs"),
        );
    }

    // Topiary: the Rust formatter the PreToolUse hook shells out to.
    println!("[claude] ensuring Topiary (bash formatter) is installed ...");
    if have("topiary") {
        println!("  topiary already installed");
    } else if have("cargo") {
        if run("cargo", &["install", "topiary-cli", "--locked"]) {
            run("topiary", &["prefetch"]);
        } else {
            eprintln!("  warning: `cargo install topiary-cli` failed; the format hook will no-op");
        }
    } else {
        eprintln!("  warning: cargo not found; install Rust + topiary-cli for the format hook");
    }
}

fn main() {
    let repo = std::env::current_dir().expect("cwd");
    if !repo.join("shared/AGENTS.md").exists() {
        eprintln!(
            "error: run from the repo root (missing {}/shared/AGENTS.md)",
            repo.display()
        );
        std::process::exit(1);
    }

    // Agent-agnostic tooling.
    ensure_ripgrep();

    // One arm per supported agent. Add more here as agents/<name>/ are added.
    install_claude(&repo);

    println!("\nDone. Agent hooks take effect in new sessions.");
}
