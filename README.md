# gittui

A VS Code Git Sidebar experience in your terminal, built with Rust.

```
┌─ gittui ── main ── ↑2 ↓3 ─────────────────────────────────────────┐
│                                                                     │
│  ┌─ BRANCHES ──────────┐  ┌─ DIFF VIEWER ────────────────────────┐ │
│  │ ● main              │  │                                      │ │
│  │   feature/auth      │  │  src/main.rs                         │ │
│  │   fix/typo          │  │  ─────────────────────────────────── │ │
│  │                     │  │  @@ -12,6 +12,8 @@                  │ │
│  ├─ STAGED (2) ────────┤  │    fn main() {                       │ │
│  │  ✓ src/main.rs      │  │  -    println!("old");              │ │
│  │  ✓ Cargo.toml       │  │  +    println!("new");              │ │
│  │                     │  │  +    init_logger();                │ │
│  ├─ CHANGES (3) ───────┤  │    }                                 │ │
│  │  M src/lib.rs       │  │                                      │ │
│  │  A src/utils.rs     │  ├─ LOG / GRAPH ────────────────────────┤ │
│  │  D old_file.rs      │  │                                      │ │
│  │                     │  │  ● a1b2c3d fix: typo (HEAD)         │ │
│  ├─ STASH (1) ─────────┤  │  ● e4f5g6h feat: add auth           │ │
│  │  stash@{0}: WIP     │  │  │\                                  │ │
│  │                     │  │  │ ● i7j8k9l fix: css              │ │
│  ├─ COMMIT ────────────┤  │  │/                                  │ │
│  │  fix: typo in README│  │  ● m0n1o2p init commit              │ │
│  │  [Commit] [Amend]   │  │                                      │ │
│  ├─ REMOTE ────────────┤  │                                      │ │
│  │  origin (synced ✓)  │  │                                      │ │
│  │  [Fetch][Pull][Push]│  │                                      │ │
│  └─────────────────────┘  └──────────────────────────────────────┘ │
│  [?] Help  [q] Quit  [/] Search  [r] Refresh       Status: Clean   │
└─────────────────────────────────────────────────────────────────────┘
```

## Features

- **8-panel layout** — Branches, Staged, Changes, Stash, Commit, Remote, Diff Viewer, Log Graph
- **Mouse + keyboard hybrid** — Click to select, scroll to navigate, or use Vim-style `j`/`k` keys
- **Word-level diff highlighting** — See exactly which words changed within each line
- **Commit graph** — Unicode box-drawing ASCII graph with branch/merge visualization
- **Command palette** — `Ctrl+P` fuzzy finder for any action, just like VS Code
- **Stash management** — Save, apply, pop, drop with full diff preview
- **Branch operations** — Create, checkout, delete, rename branches
- **Remote operations** — Fetch, pull, push with status indicators (↑N ↓M)
- **Conflict detection** — Identify and list conflicted files
- **Configurable** — TOML config file with theme, keybinding, and diff settings

## Install

```bash
# From source
git clone https://github.com/maplefukku/gittui.git
cd gittui
cargo install --path .

# Or build directly
cargo build --release
# Binary at: target/release/gittui
```

### Requirements

- Rust 1.70+
- libgit2 (bundled via `git2` crate)
- A terminal with 256-color and mouse support

## Usage

```bash
# Run in the current git repository
gittui

# Run in a specific directory
gittui /path/to/repo
```

## Keybindings

### Navigation

| Key | Action |
|-----|--------|
| `Tab` / `Shift+Tab` | Next / previous panel |
| `j` / `k` or `↑` / `↓` | Move up / down in list |
| `h` / `l` | Switch left / right pane |
| `1`–`6` | Jump to panel by number |
| `g` / `G` | Jump to first / last item |

### Git Operations

| Key | Action |
|-----|--------|
| `s` | Stage file (in Changes panel) |
| `u` | Unstage file (in Staged panel) |
| `c` | Open commit message editor |
| `Ctrl+Enter` | Execute commit |
| `a` | Toggle amend mode |
| `d` | Discard changes |
| `f` | Fetch |
| `p` | Push |
| `P` | Pull |
| `b` | Focus branches |
| `B` | Create new branch |
| `z` | Stash save |
| `Z` | Stash pop |

### General

| Key | Action |
|-----|--------|
| `?` | Toggle help overlay |
| `q` | Quit |
| `r` | Refresh all state |
| `/` | Search |
| `Ctrl+P` | Command palette |
| `Ctrl+D` | Toggle diff mode (unified / side-by-side) |

### Mouse

| Action | Effect |
|--------|--------|
| Left click | Select item, focus panel |
| Scroll | Scroll lists and diff viewer |

## Configuration

Configuration file: `~/.config/gittui/config.toml`

```toml
[general]
refresh_interval_ms = 250
mouse_enabled = true
editor = "vim"

[theme]
name = "default"

[diff]
context_lines = 3
mode = "unified"     # "unified" or "side-by-side"
word_diff = true

[log]
graph_style = "rounded"  # "rounded" | "ascii" | "unicode"
show_remote_branches = true
max_count = 500
```

## Architecture

```
src/
├── main.rs              # Entry point, terminal setup, event loop
├── app.rs               # Central state management (1,200+ lines)
├── config.rs            # TOML config file handling
├── event.rs             # Event polling (key, mouse, tick, resize)
├── git/                 # Git operations via git2-rs
│   ├── repo.rs          # Repository open, HEAD info
│   ├── status.rs        # File status (staged/unstaged/untracked)
│   ├── diff.rs          # Diff generation with hunk parsing
│   ├── branch.rs        # Branch CRUD + checkout
│   ├── commit.rs        # Commit + amend
│   ├── remote.rs        # Fetch/push/pull + ahead/behind
│   ├── stash.rs         # Stash operations
│   ├── log.rs           # Commit log with graph lane computation
│   └── conflict.rs      # Conflict detection + resolution
├── ui/                  # Rendering via ratatui
│   ├── layout.rs        # 2-pane layout calculation
│   ├── branches.rs      # Branch list panel
│   ├── staged.rs        # Staged files panel
│   ├── changes.rs       # Unstaged changes panel
│   ├── stash.rs         # Stash list panel
│   ├── commit.rs        # Commit message input
│   ├── remote.rs        # Remote status + buttons
│   ├── diff_viewer.rs   # Diff with word-level highlighting
│   ├── log_graph.rs     # Commit graph with Unicode art
│   ├── statusbar.rs     # Bottom bar with key hints
│   ├── dialog.rs        # Modal dialogs (confirm/input/select)
│   ├── help.rs          # Full-screen help overlay
│   ├── command_palette.rs # Fuzzy command finder
│   └── context_menu.rs  # Right-click popup menu
└── input/               # Input handling
    ├── key.rs           # Keyboard shortcuts dispatcher
    ├── mouse.rs         # Mouse click/scroll handler
    └── text_editor.rs   # Mini editor for commit messages
```

### Key Dependencies

| Crate | Purpose |
|-------|---------|
| `ratatui` | TUI rendering engine |
| `crossterm` | Terminal control, mouse events, key input |
| `git2` | libgit2 bindings for fast Git operations |
| `tokio` | Async runtime for background operations |
| `syntect` | Syntax highlighting support |
| `unicode-width` | CJK full-width character handling |
| `dirs` | Config file path resolution |
| `toml` / `serde` | Config file parsing |

## Design Philosophy

**"GUI usability in a TUI"** — Inspired by VS Code's Git sidebar, gittui brings the same intuitive workflow to the terminal. Mouse-first but keyboard-complete: every action is reachable both ways.

Pairs well with terminal multiplexers (tmux, zellij) and other TUI tools like `novim` for a fully terminal-native development workflow.

## License

MIT
