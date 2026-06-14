#!/usr/bin/env rust-script
//! PreToolUse(Bash) hook: auto-format the proposed command with Topiary so the
//! user reviews a clean, readable version before approving it. Topiary is a
//! Rust, tree-sitter-based formatter (the shfmt analog used here).
//!
//! Fails safe: if Topiary is missing, errors, or makes no change, the hook
//! prints nothing and the original command runs untouched.
//!
//! ```cargo
//! [dependencies]
//! serde_json = "1"
//! ```
#![forbid(unsafe_code)]
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};

/// Locate the topiary binary, preferring ~/.cargo/bin then falling back to PATH.
fn topiary_bin() -> String {
    if let Ok(home) = std::env::var("HOME") {
        let cargo = format!("{home}/.cargo/bin/topiary");
        if Path::new(&cargo).exists() {
            return cargo;
        }
    }
    "topiary".to_string()
}

fn main() {
    // Read the hook payload from stdin.
    let mut input = String::new();
    if std::io::stdin().read_to_string(&mut input).is_err() {
        return;
    }
    let payload: Value = match serde_json::from_str(&input) {
        Ok(v) => v,
        Err(_) => return,
    };

    let tool_input = match payload.get("tool_input") {
        Some(v) => v.clone(),
        None => return,
    };
    let command = tool_input
        .get("command")
        .and_then(Value::as_str)
        .unwrap_or("");
    if command.is_empty() {
        return;
    }

    // Format the command via topiary (bash). Any failure -> leave it untouched.
    let mut child = match Command::new(topiary_bin())
        .args(["format", "--language", "bash"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(_) => return,
    };
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(command.as_bytes());
        // stdin dropped here, closing the pipe so topiary can finish.
    }
    let output = match child.wait_with_output() {
        Ok(o) => o,
        Err(_) => return,
    };
    if !output.status.success() {
        return;
    }

    let formatted = String::from_utf8_lossy(&output.stdout);
    let formatted = formatted.trim_end_matches('\n');

    // No meaningful change -> no-op (don't rewrite the tool input).
    if formatted.is_empty() || formatted == command {
        return;
    }

    // Replace only the command, preserving any other tool_input fields.
    let mut new_input = tool_input;
    new_input["command"] = json!(formatted);

    let out = json!({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "updatedInput": new_input,
        }
    });
    println!("{out}");
}
