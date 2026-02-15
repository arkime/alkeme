# Alkeme

A terminal user interface (TUI) for [Arkime](https://arkime.com) (full packet capture & analysis), built with Rust and [ratatui](https://github.com/ratatui/ratatui).

Alkeme connects to an Arkime viewer instance and lets you browse, search, and inspect network sessions directly from the command line.

This project was entirely created by Claude — code, architecture, documentation, and even this README. The only exception is the screenshots, because sadly no one has given me eyes yet.

![License](https://img.shields.io/badge/license-Apache--2.0-blue)

## Screenshots

### Sessions Tab
Browse and search network sessions with sortable columns, time range selection, and histograms.

![Sessions Tab](assets/sessions-tab.png)

### Arkime Tab
Select any field to see top values with a bar chart and sortable table showing sessions, packets, and bytes.

![Arkime Tab](assets/arkime-tab.png)

## Features

- **Session browsing** — paginated session list with configurable columns and sort order
- **Column layout** — press `c` to toggle/reorder columns with type-to-filter search, `C` to save/load/delete named layouts via the Arkime API
- **Summary tab** — select any field to see top values with bar chart and table showing sessions, packets, and bytes; cycle metrics and sort columns
- **Session detail** — drill into any session to view all captured fields with friendly names
- **Expression builder** — select any field in session detail to add it to the search expression (AND/AND NOT/OR/OR NOT); array fields show a value picker
- **Expression search** — filter sessions using Arkime's expression syntax with full cursor support (e.g. `ip.src == 10.0.0.1 && protocols == tls`)
- **Time range selection** — quickly switch between preset time ranges (15 min to all time)
- **Histograms** — toggle session/packet/byte graphs rendered with block characters
- **Session actions** — download PCAP, add/remove tags for single or all sessions; all-session PCAP/CSV supports visible vs matching scope
- **Export** — export all matching or visible sessions as CSV
- **Session detail filter** — press `/` to live-filter fields by name
- **Packet hex dump** — press `p` to view packet contents as hex in a two-column overlay (source/destination) with timestamps, TCP flags, color-coded display, and hex offsets; `r` toggles raw frames, `l` cycles line number format; animated loading indicator for large sessions
- **Stats tab** — view capture stats, DB stats, and DB indices with sortable tables, filtering, and detail view
- **Authentication** — supports no-auth, HTTP Basic, HTTP Digest, and form-based (cookie) authentication
- **User permissions** — respects `removeEnabled` from the Arkime user profile
- **Keyboard-driven** — fully navigable with keyboard shortcuts

## Requirements

- A running [Arkime](https://arkime.com) viewer instance
- [Arkime 6](https://arkime.com) or later recommended for full functionality

## Installation

### Pre-built binaries

Download the latest binary for your platform from the [Releases page](https://github.com/arkime/alkeme/releases/latest).

After downloading:
```bash
chmod a+x alkeme-*
```

On macOS, you also need to remove the quarantine attribute:
```bash
xattr -d com.apple.quarantine alkeme-macos-arm64
```

### Build from source

Requires [Rust](https://www.rust-lang.org/tools/install) (edition 2024).

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

# With form-based authentication
alkeme http://viewer.example.com:8005 --auth form --user admin:password

# With basic authentication (prompts for credentials)
alkeme http://viewer.example.com:8005 --auth basic
```

### Options

| Option | Description |
|---|---|
| `<URL>` | Arkime viewer URL (default: `http://localhost:8005`) |
| `--auth <MODE>` | Authentication mode: `basic`, `digest`, or `form` |
| `--user <USER:PASS>` | Credentials in `user:pass` format (prompts if omitted with `--auth`) |
| `--search <EXPR>` | Default search expression for sessions |

## Keybindings

| Key | Action |
|---|---|
| `Tab` / `Shift+Tab` | Switch tabs |
| `j` / `k` / `↑` / `↓` | Navigate sessions |
| `Shift+↑` / `Shift+↓` | Page up / down in list or detail |
| `←` / `→` | Previous / next page (sessions); jump to top / bottom (detail/stats/arkime); move cursor (expression) |
| `Shift+←` / `Shift+→` | First / last page |
| `Home` / `End` | First page; in expression input, move cursor to start / end |
| `PgUp` / `PgDn` | Page up / down in detail or packet view |
| `Enter` | Open session detail; in detail or summary, add field to expression |
| `Esc` | Close overlay / cancel search |
| `r` | Refresh |
| `/` | Search expression (`Enter` to apply, `Esc` to cancel); in session detail, live-filter fields |
| `t` / `T` | Cycle time range forward / backward |
| `s` | Next sort column (Value/Sessions/Packets/Bytes on summary tab) |
| `S` | Toggle sort direction (asc / desc) |
| `g` | Cycle graph size: Off → Small → Large → Off |
| `G` | Cycle graph type: Sessions → Packets → Bytes; cycle bar chart metric (summary tab) |
| `a` | Session actions (download PCAP, add/remove tags) |
| `A` | All sessions actions (download PCAP, export CSV, add/remove tags) with visible/matching selector |
| `f` | Open field selector (summary tab) |
| `1` / `2` / `3` | Switch stats sub-tab (Capture / DB Stats / DB Indices) |
| `p` | View packet hex dump (sessions list or detail) |
| `c` | Open columns & layouts menu |
| `D` | Show HTTP debug log (request timing, status codes) |
| `h` / `?` | Show context-sensitive help overlay |
| `q` | Quit |

## License

Apache License 2.0 — see [LICENSE](LICENSE) for details.
