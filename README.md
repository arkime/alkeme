# Alkeme

A terminal user interface (TUI) for [Arkime](https://arkime.com) (full packet capture & analysis), built with Rust and [ratatui](https://github.com/ratatui/ratatui).

Alkeme connects to an Arkime viewer instance and lets you browse, search, and inspect network sessions directly from the command line.

This project was entirely created by Claude — code, architecture, and documentation.

![License](https://img.shields.io/badge/license-Apache--2.0-blue)

## Features

- **Session browsing** — paginated session list with configurable columns and sort order
- **Session detail** — drill into any session to view all captured fields
- **Expression search** — filter sessions using Arkime's expression syntax (e.g. `ip.src == 10.0.0.1 && protocols == tls`)
- **Time range selection** — quickly switch between preset time ranges (15 min to all time)
- **Histograms** — toggle session/packet/byte graphs rendered with block characters
- **Stats & settings tabs** — view Elasticsearch cluster stats and connection settings
- **Authentication** — supports no-auth, HTTP Basic, and HTTP Digest authentication
- **Keyboard-driven** — fully navigable with keyboard shortcuts

## Requirements

- [Rust](https://www.rust-lang.org/tools/install) (edition 2024)
- A running [Arkime](https://arkime.com) viewer instance

## Installation

```bash
git clone https://github.com/ArkimeAdmin/alkeme.git
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

## Keybindings

| Key | Action |
|---|---|
| `Tab` / `Shift+Tab` | Switch tabs |
| `j` / `k` / `↑` / `↓` | Navigate sessions |
| `←` / `→` | Previous / next page |
| `Shift+←` / `Shift+→` | First / last page |
| `Home` | First page |
| `PgUp` / `PgDn` | Scroll detail view |
| `Enter` | Open session detail |
| `Esc` | Close overlay / cancel search |
| `r` | Refresh sessions |
| `/` | Search expression (`Enter` to apply, `Esc` to cancel) |
| `t` / `T` | Cycle time range forward / backward |
| `s` | Next sort column |
| `S` | Toggle sort direction (asc / desc) |
| `g` | Cycle graph size: Off → Small → Large → Off |
| `G` | Cycle graph type: Sessions → Packets → Bytes |
| `h` | Show help overlay |
| `q` | Quit |

## License

Apache License 2.0 — see [LICENSE](LICENSE) for details.
