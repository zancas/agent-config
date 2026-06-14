# Agent instructions

Global, agent-agnostic instructions for any AI coding assistant. This file is the
canonical source; each agent's expected instructions file (e.g. `~/.claude/CLAUDE.md`)
is symlinked to it.

## Human-readable commands

Every shell command you propose to run must be human-readable — the user reads
it before approving, so optimize the command for a human, not just the shell.

- Prefer multi-line form with `\` line continuations over one long line.
- Put each distinct step on its own line (one pipeline stage, one `&&` clause,
  or one logical action per line).
- Add a short `#` comment before any step whose purpose isn't obvious.
- Use long flag names (`--force` not `-f`) when the short form is cryptic.
- Avoid dense, quoted-within-quoted one-liners. If logic needs nesting, `printf`
  juggling, or escaped JSON inside a string, move it into a named script file
  and call that script instead.
- Keep names meaningful: `skill="grill-me"` beats `s=grill-me`.

The goal: a person skimming the command should understand what it does and why
without having to mentally parse it.

## Scripting language preference

When a task calls for a script, choose the language by these rules, in priority
order (a higher rule wins when they conflict):

1. **Checked into a repo → MUST be Rust (`rust-script`).** Anything committed —
   hooks, installers, tooling kept on disk and re-run — is Rust. No Bash or
   Python files get checked in. Use single-file `rust-script` (shebang
   `#!/usr/bin/env rust-script`, inline `//! ```cargo` dependency block), or a
   real Cargo project once it outgrows one file.
2. **Long-lived system/config scripts** (agent hooks, tooling kept on disk and
   re-run over time) → **Rust / `rust-script`**, even if not yet committed.
3. **Single-use throwaway scripts I read as a command proposal** (inline one-offs
   I approve in the moment) → **Python**, because it's the most readable at a
   glance for review — *except* when a simple proposal is more succinctly
   expressed in **Bash**, in which case use Bash.
4. **Anything else / general scripting** → prefer `rust-script` or Python over
   shell; avoid Bash for non-trivial logic.

Net: committed/long-lived ⇒ Rust; quick proposals I review ⇒ Python, or Bash
when that's genuinely shorter and clearer for a simple task.

### Rust conventions

- Every rust-script (and Rust source generally) must forbid unsafe code: put
  `#![forbid(unsafe_code)]` at the crate top (after the shebang and `//!` doc /
  `cargo` block, before the first `use`). `forbid` — not `deny` — so it can't be
  locally overridden.

## Tooling preferences

### Rust code: reach for the Rust toolchain first

When operating against Rust source, prefer the language's own standard tooling
over generic text manipulation, wherever possible:

- **Understanding, navigation, refactors:** use the LSP (rust-analyzer) —
  go-to-definition, find-references, rename, hover, workspace symbols,
  diagnostics — instead of grepping for symbols by hand.
- **Build / check / test / run / format / lint:** use `cargo` (`cargo check`,
  `cargo build`, `cargo test`, `cargo run`, `cargo clippy`, `cargo fmt`) and
  `rustc`, rather than ad-hoc parsing of source or output.

Only fall back to generic tooling (Python, `rg`, `sed`, `awk`, etc.) when there
is genuinely no Rust-tool-specific way to accomplish the task.

### Search: ripgrep, always

Use `rg` (ripgrep) instead of `grep` in every case — it is faster, respects
`.gitignore`, and has saner defaults. If `rg` is missing, install it with
`cargo install ripgrep`.
