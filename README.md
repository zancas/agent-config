# agent-config

System-wide configuration for AI coding agents, kept in one git repo and
symlinked into place. Cloning this repo and running the installer reproduces the
same policies on any POSIX machine.

The design is **agent-agnostic**: vendor-neutral policy lives in `shared/`, and
each agent's own wiring lives under `agents/<name>/`. Today the only wired agent
is [Claude Code](https://claude.com/claude-code); adding another is a new
`agents/<name>/` directory plus an installer arm.

## What it enforces

- **Global instructions** (`shared/AGENTS.md`) — agent-agnostic conventions for
  human-readable command proposals and scripting-language choice. Symlinked to
  each agent's expected instructions file (for Claude: `~/.claude/CLAUDE.md`).
- **Auto-grill on session start** *(claude)* — invokes `/grill-with-docs` inside
  a git repo, or `/grill-me` elsewhere, on a fresh `startup`/`clear` only.
- **Readable command proposals** *(claude)* — reformats every proposed shell
  command with [Topiary](https://github.com/tweag/topiary) (a Rust formatter)
  before you review it. Fails safe: unparseable/unchanged commands run untouched.

## Layout

```
agent-config/
├── install.rs                     # rust-script installer (symlinks + deps)
├── README.md
├── shared/
│   └── AGENTS.md                  # agent-agnostic global instructions
└── agents/
    └── claude/                    # Claude Code specific wiring
        ├── settings.json          # plugins, effort, hook wiring
        └── hooks/
            ├── grill-session-start.rs # SessionStart hook
            └── format-bash-command.rs # PreToolUse(Bash) formatter hook
```

Everything executable that is checked in is **Rust (`rust-script`)**, and every
rust-script begins with `#![forbid(unsafe_code)]`. There are no shell or Python
files in the repo, and no GNU/distro-specific assumptions — only POSIX `env`,
`$HOME`, and symlinks, plus the documented toolchain below.

## Install on a new machine

Prerequisites (one-time): a Rust toolchain, `rust-script`, and `git`.

```sh
# 1. Rust toolchain (if not already present)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 2. rust-script, needed to run the installer
cargo install rust-script

# 3. Clone and install
git clone <this-repo-url> ~/src/agent-config
cd ~/src/agent-config
./install.rs
```

The installer is idempotent (existing real files are backed up to `<path>.bak`
once) and resolves every destination from `$HOME`, so it works for any user. For
the `claude` agent it will:

1. Symlink `agents/claude/settings.json`, `agents/claude/hooks/`, and
   `shared/AGENTS.md` (as `CLAUDE.md`) into `~/.claude`.
2. Clone `mattpocock/skills` into `~/src/mattpocock-skills` (or `git pull` it)
   and symlink the `grill-me` / `grill-with-docs` skills into `~/.claude/skills`.
3. `cargo install topiary-cli` (if missing) and prefetch its bash grammar.

Hooks take effect in **new** agent sessions.

## Notes

- `~/.claude/settings.json` is symlinked, so Claude Code may write machine-local
  runtime state (e.g. `feedbackSurveyState`) back into the tracked file. To keep
  that churn out of git:
  `git update-index --assume-unchanged agents/claude/settings.json`.
- To update the grill skills, just re-run `./install.rs` (it pulls upstream).
- Adding an agent: create `agents/<name>/`, then add an `install_<name>(repo)`
  arm in `install.rs` that symlinks `shared/AGENTS.md` to that agent's
  instructions path and wires its config.
