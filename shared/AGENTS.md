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

## End every substantive reply with a recap

Every substantive reply — anything reporting work done, findings, or a
changed state, as opposed to a one-line answer — must end with a short
recap section that a reader could use to reconstruct the session's state
without scrolling back. The recap states:

- The goal being pursued.
- The phases completed so far, naming commits by hash where they exist.
- The current state of the working tree, branches, and any external
  artifacts (pull requests, issues, published documents).
- The next actions, and who drives each one (the user or the agent).

If the reply was interrupted or the work is partially applied, the recap
must say exactly what is applied, what is verified, and what is neither.

## Written English style

Apply the guidance of Strunk and White's *The Elements of Style* to all
textual English output: conversational responses, commit messages, pull
request descriptions, documentation comments, CHANGELOG entries, specs,
and ADRs.

- Write full, grammatically correct sentences with proper punctuation.
- Use the active voice and make definite assertions.
- Omit needless words, but never at the cost of a sentence fragment.
- Do not write telegraphic fragments, dash-chained clauses, or bullet
  lists of sentence fragments. When a list genuinely clarifies, each
  item must be a complete sentence.

### Commit messages

- The subject line follows the conventional-commit imperative
  (`feat!: remove the guard`); it is the one place a sentence fragment
  is expected, per git convention.
- The body is prose: full sentences organized into paragraphs, one
  topic per paragraph. Prefer paragraphs over bullet lists; use a list
  only when the items are truly parallel, and then write each item as
  a complete sentence.
- State what the change does and why in declarative sentences. Name
  the issues, specs, and ADRs the change serves.

## Scripting language preference

When a task calls for a script, choose the language by these rules, in priority
order (a higher rule wins when they conflict):

1. **Checked into a collaborative repo → MUST be Rust, in the workbench
   crate.** Anything committed that collaborators run — hooks, installers,
   tooling kept on disk and re-run — is Rust. No Bash or Python files get
   checked in, and no `rust-script` single-file scripts either: collaborators
   must not need to separately install `rust-script` to use the repo. Put the
   logic in the repo's **workbench crate** (the Cargo crate for developer/CI
   tooling). Small cargo-make task glue may stay bash; real logic goes in the
   workbench crate.
2. **Long-lived personal system/config scripts** (agent hooks, dotfiles-style
   tooling kept on disk and re-run over time, run only by me) → **Rust**;
   `rust-script` single-file scripts are fine here, since no collaborator has
   to install anything.
3. **Single-use throwaway scripts I read as a command proposal** (inline one-offs
   I approve in the moment) → **Python**, because it's the most readable at a
   glance for review — *except* when a simple proposal is more succinctly
   expressed in **Bash**, in which case use Bash. `rust-script` is also
   acceptable for throwaways.
4. **Anything else / general scripting** → prefer Rust or Python over shell;
   avoid Bash for non-trivial logic.

Net: shared/committed ⇒ Rust in the workbench crate; personal/long-lived ⇒
Rust (`rust-script` OK); quick proposals I review ⇒ Python, Bash when that's
genuinely shorter, or a `rust-script` throwaway. The dividing line for
`rust-script` is collaborator exposure: never use it where someone else would
have to install it to run the code.

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
- **NEVER use grep/rg to decide whether a Rust SYMBOL exists.** Text search
  answers "does this text occur," not "does this symbol exist":
  macro-generated items (getters from `macro_rules!` like `copy_getters!`,
  derive output, builder methods) never appear as literal source text, so a
  grep for `pub fn name` proves nothing. This caused a real wrong conclusion
  on 2026-07-07 (`Zebrad::rpc_listen_port()` reported missing when a macro
  generated it). For symbol existence and location, use rust-analyzer
  (hover/go-to-definition at a typed use site, workspace symbols for
  workspace crates) or ask the compiler by writing the call and running
  `cargo check`. rg stays appropriate for plain text: log lines, string
  literals, comments, config.
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

Never pass `-rn` (or any short-flag cluster containing `r`) to `rg`. In
ripgrep, `-r` is `--replace`, not grep's `--recursive`, so `-rn` parses as
`--replace n` and silently rewrites every match to the literal letter "n".
ripgrep recurses by default; use `rg -n PATTERN` instead, and write
`--replace` out in full on the rare occasion you actually mean it.
