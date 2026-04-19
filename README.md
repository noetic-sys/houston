# Houston

A terminal UI for managing GitHub Actions workflows and deployments — built for engineers who live in the terminal.

Houston gives you a multi-panel, keyboard-driven interface to browse repos, trigger workflows, monitor runs, tail logs, and track deployment environments without leaving your shell.

---

## Installation

```sh
brew install noetic-sys/tap/houston
```

### Prerequisites

Houston reads your GitHub token via the `gh` CLI. Make sure you're authenticated:

```sh
gh auth login
```

---

## Usage

```sh
houston
```

That's it. Houston picks up your GitHub credentials automatically and loads your repos on launch.

---

## Keybindings

### Navigation

| Key | Action |
|-----|--------|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `Tab` | Focus next panel |
| `Shift+Tab` | Focus previous panel |
| `h` | Focus previous panel (vim-style) |

### Layout Presets

Switch between preset layouts with number keys:

| Key | Layout |
|-----|--------|
| `1` | Repos · Workflows · Tags |
| `2` | Workflows (fullscreen) |
| `3` | Runs · Logs |
| `4` | Deployments · Runs |
| `5` | Tags · Workflows |

### Panel Controls

| Key | Action |
|-----|--------|
| `z` | Zoom focused panel fullscreen / unzoom |
| `l` | Toggle Logs panel |
| `d` | Toggle Deployments panel |
| `t` | Toggle Tags panel |

### Actions

| Key | Action |
|-----|--------|
| `Space` | Trigger selected workflow |
| `Enter` | Open run logs (when Runs panel focused) |
| `r` | Refresh runs |
| `/` | Search / filter repos |
| `Esc` | Exit search or close dialog |
| `Ctrl+C` / `Ctrl+Q` | Quit |

### Workflow Input Dialogs

When a workflow has `workflow_dispatch` inputs, Houston shows a form automatically:

| Key | Action |
|-----|--------|
| `Tab` / `Shift+Tab` | Next / previous field |
| `j` / `k` | Navigate dropdown options |
| `Space` | Open dropdown / toggle boolean |
| `Enter` | Submit |
| `Esc` | Cancel |

### Log Viewer (zoomed)

| Key | Action |
|-----|--------|
| `j` / `↓` | Scroll down |
| `k` / `↑` | Scroll up |
| `g` | Jump to top |
| `G` | Jump to bottom |
| `z` / `Esc` | Exit zoom |

---

## Building from Source

Requires Rust stable (1.75+).

```sh
git clone https://github.com/kasandell/houston
cd houston
cargo build --release
# binary at target/release/houston
```

---

## License

MIT — see [LICENSE](LICENSE).
