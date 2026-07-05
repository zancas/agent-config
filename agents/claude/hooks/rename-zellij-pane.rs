#!/usr/bin/env rust-script
//! SessionStart hook: label the zellij pane claude was launched in with
//! the launch directory's absolute path (with a leading `~` when it is
//! under $HOME).
//!
//! `zellij action rename-pane` targets the *focused* pane; at session
//! start that is the pane claude was just launched in, so no pane-id
//! plumbing is needed. A manually set pane name sticks until
//! `undo-rename-pane`, so later terminal-title updates from claude
//! don't overwrite it.
//!
//! Outside zellij this hook does nothing: claude's own OSC terminal
//! title (icon + path) reaches the window manager unaided.
#![forbid(unsafe_code)]
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The absolute launch path, with $HOME abbreviated to `~` when it is a
/// prefix (`/home/me/src/x` -> `~/src/x`, `/home/me` -> `~`).
fn display_name(cwd: &Path) -> String {
    if let Some(home) = std::env::var_os("HOME") {
        if let Ok(rest) = cwd.strip_prefix(PathBuf::from(home)) {
            return if rest.as_os_str().is_empty() {
                "~".to_string()
            } else {
                format!("~/{}", rest.display())
            };
        }
    }
    cwd.display().to_string()
}

fn main() {
    // Drain the hook payload so the harness never blocks on a full pipe;
    // the payload itself carries nothing this hook needs.
    let mut sink = String::new();
    let _ = std::io::stdin().read_to_string(&mut sink);

    let Ok(cwd) = std::env::current_dir() else {
        return;
    };
    let name = display_name(&cwd);

    if std::env::var_os("ZELLIJ").is_some() {
        // A failed rename is purely cosmetic and a hook has no one to
        // report to, so the exit status is deliberately discarded.
        let _ = Command::new("zellij")
            .args(["action", "rename-pane", &name])
            .status();
    }
}
