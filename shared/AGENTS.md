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

1. **Checked into a repo → MUST be Rust, in the workbench crate.** Anything
   committed — hooks, installers, tooling kept on disk and re-run — is Rust. No
   Bash or Python files get checked in, and `rust-script` single-file scripts
   are retired: put the logic in the repo's **workbench crate** (the Cargo crate
   for developer/CI tooling). Small cargo-make task glue may stay bash; real
   logic goes in the workbench crate.
2. **Long-lived system/config scripts** (agent hooks, tooling kept on disk and
   re-run over time) → **Rust** (workbench crate), even if not yet committed.
3. **Single-use throwaway scripts I read as a command proposal** (inline one-offs
   I approve in the moment) → **Python**, because it's the most readable at a
   glance for review — *except* when a simple proposal is more succinctly
   expressed in **Bash**, in which case use Bash.
4. **Anything else / general scripting** → prefer Rust (workbench crate) or
   Python over shell; avoid Bash for non-trivial logic.

Net: committed/long-lived ⇒ Rust; quick proposals I review ⇒ Python, or Bash
when that's genuinely shorter and clearer for a simple task.

### Rust conventions

- Every Rust source file must forbid unsafe code: put `#![forbid(unsafe_code)]`
  at the crate top (before the first `use`). `forbid` — not `deny` — so it can't
  be locally overridden.

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

- **Dependency and manifest changes:** use `cargo add`, `cargo remove`, and
  `cargo update` (with `--features`, `--optional`, `--dev` as needed), not
  hand-edits or scripted rewrites of `Cargo.toml`.

**Always prefer Rust-native tools in any domain where they are designed to
operate.** Only fall back to generic tooling (Python, `rg`, `sed`, `awk`, etc.)
when there is genuinely no Rust-tool-specific way to accomplish the task. In
particular, never batch-edit Rust source or manifests with Python/sed sweeps
when a cargo or rust-analyzer operation covers the job.

**rust-analyzer cold-start gotcha.** On a large workspace, `workspaceSymbol`
(and other index-wide queries) return *empty* — e.g. "No symbols found in
workspace" — for the first seconds-to-minutes after the server starts, because
the workspace symbol index is still building. This is **not** a "server down" or
"symbol missing" result. Diagnose by running a *per-file* op first
(`documentSymbol`/`hover`/`goToDefinition` at a known location): if that
succeeds, the server is up and the empty `workspaceSymbol` just means the index
isn't warm yet. Remedy: retry `workspaceSymbol` after a short delay, or use the
per-file op meanwhile. Don't conclude a symbol doesn't exist from an empty
`workspaceSymbol` on a cold index.

Note the agent's LSP tool runs its *own* rust-analyzer process, independent of
any editor's (Helix, VS Code, etc.) server on the same repo. So a warm editor
LSP does **not** mean the agent's server is warm — it has its own cold-start.
And two rust-analyzers indexing a large workspace at once contend for CPU/RAM,
which slows indexing for both, lengthening the empty-`workspaceSymbol` window.

**Verify your own edits with the toolchain.** Use the LSP and `cargo`
(`cargo check`, `cargo clippy`, `cargo fmt`), `rustc` actively and wherever
possible to confirm your changes compile and are clean — iterate on the errors
before handing work back. Do not hand off a non-trivial change unverified on the
theory that "the user runs their own pipeline."

**But tests and commits are the user's to run.** The user prefers to run tests
and create commits manually:
- Running *fast* tests yourself is fine. Do **not** launch long-running test
  jobs (full `cargo nextest run`, `cargo test`, integration/container suites)
  without asking first. When you do run tests, use `cargo nextest run`, never
  bare `cargo test`.
- Propose commits; do not auto-commit or push unless explicitly asked.

### Search: ripgrep, always

Use `rg` (ripgrep) instead of `grep` in every case — it is faster, respects
`.gitignore`, and has saner defaults. If `rg` is missing, install it with
`cargo install ripgrep`.
