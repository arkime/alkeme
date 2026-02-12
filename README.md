# Alkeme

A terminal user interface (TUI) for [Arkime](https://arkime.com) (full packet capture & analysis), built with Rust and [ratatui](https://github.com/ratatui/ratatui).

Alkeme connects to an Arkime viewer instance and lets you browse, search, and inspect network sessions directly from the command line.

This project was entirely created by Claude — code, architecture, and documentation.

![License](https://img.shields.io/badge/license-Apache--2.0-blue)

## Features

- **Session browsing** — paginated session list with configurable columns and sort order
- **Session detail** — drill into any session to view all captured fields with friendly names
- **Expression builder** — select any field in session detail to add it to the search expression (AND/AND NOT/OR/OR NOT); array fields show a value picker
- **Expression search** — filter sessions using Arkime's expression syntax with full cursor support (e.g. `ip.src == 10.0.0.1 && protocols == tls`)
- **Time range selection** — quickly switch between preset time ranges (15 min to all time)
- **Histograms** — toggle session/packet/byte graphs rendered with block characters
- **Session actions** — download PCAP, add/remove tags for single or all sessions; all-session PCAP/CSV supports visible vs matching scope
- **Export** — export all matching or visible sessions as CSV
- **Session detail filter** — press `/` to live-filter fields by name
- **Stats tab** — view capture stats, DB stats, and DB indices with sortable tables, filtering, and detail view
- **Authentication** — supports no-auth, HTTP Basic, and HTTP Digest authentication
- **User permissions** — respects `removeEnabled` from the Arkime user profile
- **Keyboard-driven** — fully navigable with keyboard shortcuts

## Requirements

- [Rust](https://www.rust-lang.org/tools/install) (edition 2024)
- A running [Arkime](https://arkime.com) viewer instance
- [Arkime 6](https://arkime.com) or later recommended for full functionality

## Installation

```bash
git clone https://github.com/arkime/alkeme.git
cd alkeme
cargo build --release
```

The binary will be at `target/release/alkeme`.

## Usage

```bash
# Connect to a local Arkime viewer (default: http://localhost:8005)
alkeme

# Connect to a specific URL
alkeme http://viewer.example.com:8005

# With digest authentication (inline credentials)
alkeme http://viewer.example.com:8005 --auth digest --user admin:password

# With basic authentication (prompts for credentials)
alkeme http://viewer.example.com:8005 --auth basic
```

### Options

| Option | Description |
|---|---|
| `<URL>` | Arkime viewer URL (default: `http://localhost:8005`) |
| `--auth <MODE>` | Authentication mode: `basic` or `digest` |
| `--user <USER:PASS>` | Credentials in `user:pass` format (prompts if omitted with `--auth`) |
| `--search <EXPR>` | Default search expression for sessions |

## Keybindings

| Key | Action |
|---|---|
| `Tab` / `Shift+Tab` | Switch tabs |
| `j` / `k` / `↑` / `↓` | Navigate sessions |
| `Shift+↑` / `Shift+↓` | Page up / down in list |
| `←` / `→` | Previous / next page; in expression input, move cursor |
| `Shift+←` / `Shift+→` | First / last page |
| `Home` / `End` | First page; in expression input, move cursor to start / end |
| `PgUp` / `PgDn` | Page up / down in detail view |
| `Enter` | Open session detail; in detail, add field to expression |
| `Esc` | Close overlay / cancel search |
| `r` | Refresh data |
| `/` | Search expression or filter (`Enter` to apply, `Esc` to cancel); in session detail, live-filter fields |
| `t` / `T` | Cycle time range forward / backward |
| `s` | Next sort column |
| `S` | Toggle sort direction (asc / desc) |
| `g` | Cycle graph size: Off → Small → Large → Off |
| `G` | Cycle graph type: Sessions → Packets → Bytes |
| `a` | Session actions (download PCAP, add/remove tags) |
| `A` | All sessions actions (download PCAP, export CSV, add/remove tags) with visible/matching selector |
| `1` / `2` / `3` | Switch stats sub-tab (Capture / DB Stats / DB Indices) |
| `h` | Show help overlay |
| `q` | Quit |

## License

Apache License 2.0 — see [LICENSE](LICENSE) for details.
