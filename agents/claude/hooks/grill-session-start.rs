#!/usr/bin/env rust-script
//! SessionStart hook: auto-start a "grilling" session.
//!
//! - Interactive sessions only: fire on a fresh `startup` or `/clear`, but stay
//!   quiet on `resume`/`compact` (continuations that re-trigger SessionStart).
//! - Repo-aware: inside a git repo use /grill-with-docs, otherwise /grill-me.
//!
//! Emits a SessionStart hookSpecificOutput JSON object on stdout whose
//! additionalContext tells the model to invoke the chosen skill immediately.
//!
//! ```cargo
//! [dependencies]
//! serde_json = "1"
//! ```
#![forbid(unsafe_code)]
use serde_json::{json, Value};
use std::io::Read;
use std::process::{Command, Stdio};

fn main() {
    // Read the hook payload from stdin.
    let mut input = String::new();
    if std::io::stdin().read_to_string(&mut input).is_err() {
        return;
    }
    let payload: Value = serde_json::from_str(&input).unwrap_or(Value::Null);

    // Only run for genuinely new interactive sessions.
    let source = payload.get("source").and_then(Value::as_str).unwrap_or("");
    if source != "startup" && source != "clear" {
        return;
    }

    // Pick the skill based on whether we launched inside a git repo.
    let in_repo = Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    let skill = if in_repo { "grill-with-docs" } else { "grill-me" };

    // Inject the standing instruction back into the model's context.
    let instruction = format!(
        "Standing user instruction: immediately invoke the /{skill} skill now, \
         before doing anything else, to begin a grilling session. \
         Do this even if the user has not yet asked a question."
    );

    let out = json!({
        "hookSpecificOutput": {
            "hookEventName": "SessionStart",
            "additionalContext": instruction,
        }
    });
    println!("{out}");
}
