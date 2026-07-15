#!/usr/bin/env rust-script
//! PreToolUse(Bash) hook: deny commands that invoke ripgrep with a short-flag
//! cluster containing `r` (e.g. `rg -rn`). In ripgrep `-r` is `--replace`, not
//! grep's `--recursive`, so `-rn` parses as `--replace n` and silently rewrites
//! every match to the literal letter "n". The deny reason teaches the model the
//! correct form so it retries with `rg -n`.
//!
//! The tokenizer is quote-aware: a quoted argument such as `rg -l 'rg -rn '`
//! (searching *for* the footgun text) is one quoted token and is never treated
//! as a flag, so legitimate searches are not blocked.
//!
//! Fails safe: on unreadable or unexpected input the hook prints nothing and
//! the command runs untouched.
//!
//! ```cargo
//! [dependencies]
//! serde_json = "1"
//! ```
#![forbid(unsafe_code)]
use serde_json::{json, Value};
use std::io::Read;

/// One shell word, with a note of whether any part of it was quoted.
struct Token {
    text: String,
    quoted: bool,
}

/// Split a command line into pipeline/list segments of whitespace-separated
/// tokens, tracking single quotes, double quotes, and backslash escapes so
/// that quoted text neither splits tokens nor ends segments.
fn segments(command: &str) -> Vec<Vec<Token>> {
    let mut all = Vec::new();
    let mut segment: Vec<Token> = Vec::new();
    let mut token = String::new();
    let mut token_quoted = false;
    let mut in_single = false;
    let mut in_double = false;
    let mut chars = command.chars().peekable();

    let flush_token = |token: &mut String, token_quoted: &mut bool, segment: &mut Vec<Token>| {
        if !token.is_empty() || *token_quoted {
            segment.push(Token {
                text: std::mem::take(token),
                quoted: std::mem::take(token_quoted),
            });
        }
    };

    while let Some(c) = chars.next() {
        if in_single {
            if c == '\'' {
                in_single = false;
            } else {
                token.push(c);
            }
            continue;
        }
        if in_double {
            match c {
                '"' => in_double = false,
                '\\' => {
                    if let Some(&next) = chars.peek() {
                        token.push(next);
                        chars.next();
                    }
                }
                _ => token.push(c),
            }
            continue;
        }
        match c {
            '\'' => {
                in_single = true;
                token_quoted = true;
            }
            '"' => {
                in_double = true;
                token_quoted = true;
            }
            '\\' => {
                if let Some(&next) = chars.peek() {
                    token.push(next);
                    chars.next();
                }
            }
            '|' | ';' | '&' | '\n' | '(' | ')' | '`' => {
                flush_token(&mut token, &mut token_quoted, &mut segment);
                if !segment.is_empty() {
                    all.push(std::mem::take(&mut segment));
                }
            }
            c if c.is_whitespace() => {
                flush_token(&mut token, &mut token_quoted, &mut segment);
            }
            _ => token.push(c),
        }
    }
    flush_token(&mut token, &mut token_quoted, &mut segment);
    if !segment.is_empty() {
        all.push(segment);
    }
    all
}

/// True when this unquoted token names the ripgrep binary.
fn is_rg(token: &Token) -> bool {
    !token.quoted && (token.text == "rg" || token.text.ends_with("/rg"))
}

/// True when an unquoted flag token after `rg` is a short-flag cluster that
/// contains `r` alongside other letters — the `--replace` footgun. A bare
/// `-r` is deliberate replace usage and is allowed.
fn is_footgun_flag(token: &Token) -> bool {
    if token.quoted || !token.text.starts_with('-') || token.text.starts_with("--") {
        return false;
    }
    let cluster = &token.text[1..];
    cluster.len() >= 2
        && cluster.contains('r')
        && cluster.chars().all(|c| c.is_ascii_alphanumeric())
}

/// Scan every command segment for an rg invocation followed by a footgun
/// cluster, stopping at an unquoted `--` (end of options).
fn command_has_footgun(command: &str) -> bool {
    for segment in segments(command) {
        for (i, token) in segment.iter().enumerate() {
            if !is_rg(token) {
                continue;
            }
            for later in &segment[i + 1..] {
                if !later.quoted && later.text == "--" {
                    break;
                }
                if is_footgun_flag(later) {
                    return true;
                }
            }
        }
    }
    false
}

fn main() {
    let mut input = String::new();
    if std::io::stdin().read_to_string(&mut input).is_err() {
        return;
    }
    let payload: Value = match serde_json::from_str(&input) {
        Ok(v) => v,
        Err(_) => return,
    };
    let command = payload
        .get("tool_input")
        .and_then(|t| t.get("command"))
        .and_then(Value::as_str)
        .unwrap_or("");
    if command.is_empty() || !command_has_footgun(command) {
        return;
    }

    let out = json!({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": "deny",
            "permissionDecisionReason": "In ripgrep, -r is --replace (not grep's --recursive), so a cluster like -rn parses as `--replace n` and silently rewrites every match to the literal letter n. ripgrep recurses by default: re-run with `rg -n PATTERN` (or plain `rg`), and spell out --replace if you genuinely meant it.",
        }
    });
    println!("{out}");
}
