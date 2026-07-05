#!/usr/bin/env rust-script
//! SessionEnd hook: release the zellij pane name pinned by the
//! SessionStart rename-zellij-pane hook, so the pane returns to
//! terminal-title-driven naming once claude exits.
//!
//! `zellij action undo-rename-pane` targets the *focused* pane; the
//! session ends from input in the pane claude runs in, so that is the
//! focused pane — the same assumption the rename hook makes at start.
//!
//! Outside zellij there is nothing to release: the start hook pins
//! nothing there.
#![forbid(unsafe_code)]
use std::io::Read;
use std::process::Command;

fn main() {
    // Drain the hook payload so the harness never blocks on a full pipe;
    // the payload itself carries nothing this hook needs.
    let mut sink = String::new();
    let _ = std::io::stdin().read_to_string(&mut sink);

    if std::env::var_os("ZELLIJ").is_some() {
        // A failed undo is purely cosmetic and a hook has no one to
        // report to, so the exit status is deliberately discarded.
        let _ = Command::new("zellij")
            .args(["action", "undo-rename-pane"])
            .status();
    }
}
