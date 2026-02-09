# CLAUDE.md - Alkeme Development Guide

## What is this?
Rust/ratatui TUI for Arkime. Talks to the Arkime viewer REST API. Run: `cargo run -- http://localhost:8005`

## Build
```
cargo build                                                    # debug
cargo run -- URL                                               # no auth
cargo run -- URL --auth digest --user admin:admin              # digest auth
cargo run -- URL --auth basic --user admin:admin               # basic auth
cargo run -- URL --auth digest                                 # prompts for credentials
```

## Architecture

```
src/
  main.rs   - Entry point, clap CLI parsing, terminal setup, event loop (crossterm polling)
  app.rs    - All state + input handling (App struct, enums, key handlers)
  api.rs    - ArkimeClient: HTTP calls to viewer (reqwest + digest_auth + serde_json::Value)
  ui.rs     - All rendering (ratatui Frame draws, one fn per view)
```

## Key types

- `App` — All mutable state. Passed as `&mut` to handlers and renderers.
- `Tab` — Enum with `ALL` const array. Tabs: Arkime, Sessions, Stats, Settings.
- `TimeRange` — Enum: Minutes15..All. Has `label()`, `date_value()`, `next()`, `prev()`.
- `InputMode` — Enum: `Normal` | `Expression`. Controls whether keys go to expression input.
- `SessionView` — Enum: `List` | `Detail`. Controls which session sub-view renders.
- `GraphType` — Enum: `Sessions` | `Packets` | `Bytes`. Selects which histogram to display.
- `GraphSize` — Enum: `Off` | `Small` (10 rows) | `Large` (20 rows). Three-state graph toggle.
- `ArkimeClient` — Wraps `reqwest::Client` + `base_url` + auth. All API calls return `Result<T>`.
- `AuthMode` — Enum: `None` | `Basic` | `Digest`.
- `GraphData` — Deserialized histogram data from `facets=1` API response.
- `TableState` — ratatui widget state for session list scrolling.
- Session data is `serde_json::Value` (not typed structs) since Arkime fields are dynamic.

## Current keybindings

| Key | Action |
|---|---|
| Tab / Shift+Tab | Switch tabs |
| j / k / ↑ / ↓ | Navigate sessions |
| ← / → | Previous/next page |
| Shift+← / Shift+→ | First/last page |
| Home | First page |
| PgUp / PgDn | Scroll detail view |
| Enter | Open session detail |
| Esc | Close overlay |
| r | Refresh sessions |
| / | Search expression (Enter to apply, Esc to cancel) |
| t / T | Cycle time range forward/backward |
| s | Next sort column |
| S | Toggle sort direction (asc/desc) |
| g | Cycle graph: Off → Small → Large → Off |
| G | Cycle graph type: Sessions → Packets → Bytes |
| h | Show help overlay |
| q | Quit |

## Session columns (in order)

ipProtocol (4-char mapped: TCP/UDP/ICMP/ICM6/etc), firstPacket, lastPacket, source.ip,
source.port, destination.ip, destination.port, protocol (array, comma-joined),
source.packets, destination.packets, source.bytes, destination.bytes

## Pagination

- `App` tracks `page_start` (offset) and `page_size` (default 100)
- API uses `start=N&length=100` params
- Title bar shows `Sessions [1-100 of 742] ◄ ►`
- Selection resets to top row on page change
- `page_start` resets to 0 when changing time range, sort, or expression
- Shift+arrow uses match guards on `key.modifiers.contains(KeyModifiers::SHIFT)` — must be matched before plain arrow keys

## Graph feature

- `facets=1` is only added to the sessions API query when the graph is visible (performance)
- Graph renders directly to the frame buffer using `█` block characters
- For Packets/Bytes graphs, src (cyan) and dst (green) are shown with different colors
- Timestamps are spread proportionally across terminal width using first/last timestamps
- Bottom border shows start/stop times; title shows max value and per-bar duration
- `GraphData` holds histogram arrays: `sessions_histo`, `src_packets_histo`, `dst_packets_histo`, `src_bytes_histo`, `dst_bytes_histo`

## Date field handling

- At startup, `/api/fields?array=true` is called to identify date fields
- Both `type: "seconds"` and `type: "date"` fields store values as **epoch milliseconds** (divide by 1000)
- `format_epoch()` converts to local time as `YYYY/MM/DD HH:MM:SS`
- `ip_protocol_str()` maps IANA protocol numbers to 4-char strings

## Patterns

### Adding a new tab
1. Add variant to `Tab` enum in `app.rs`, update `ALL` array and `name()`
2. Add `Tab::Foo => draw_foo(f, app, area)` in `ui::draw()` match
3. Add `draw_foo()` fn in `ui.rs`
4. Add any tab-specific state fields to `App` struct
5. If tab has sub-views, create an enum like `SessionView`

### Adding a new API call
1. Add method to `ArkimeClient` in `api.rs`
2. Use `self.authenticated_get(&url).await?` for GET requests (handles auth automatically)
3. Parse with `serde_json::from_str` — use `Value` for dynamic data, typed structs for fixed schemas
4. Add response struct with `#[derive(Deserialize)]` if needed, use `#[serde(rename = "camelCase")]` for JS field names
5. Call from `App` method in `app.rs`, store result in App fields

### Adding keybindings
1. Key handling is in `app.rs`: `handle_key()` dispatches to view-specific handlers
2. List-level keys go in `handle_list_key()`, detail keys in `handle_detail_key()`
3. Global keys (Ctrl+C, q) are in `main.rs::run_app()`
4. For new tabs with their own keys, add a `handle_<tab>_key()` method
5. Update help text in `draw_help()` in `ui.rs`

### Adding a new view/sub-view
1. Add draw fn in `ui.rs`: `fn draw_foo(f: &mut Frame, app: &mut App, area: Rect)`
2. Use `Layout::default().direction().constraints().split(area)` to subdivide
3. Use ratatui widgets: `Table`, `Paragraph`, `List`, `Block`, `Tabs`
4. For scrollable lists, use `TableState` with `render_stateful_widget`
5. Selection state lives in `App`, rendering reads it

### Adding a session column
1. Add field name to `session_fields` vec in `App::new()` in `app.rs`
2. Add label to `labels` array in `draw_session_list()` in `ui.rs`
3. Add width to `widths` array in `draw_session_list()` in `ui.rs`
4. If field needs special formatting (like ipProtocol), add handling in the cell render loop

## Arkime API reference (viewer REST)

All endpoints are relative to base_url. Use `flatten=1` to get dot-notation field names.

| Endpoint | Method | Purpose | Key params |
|---|---|---|---|
| `/api/sessions` | GET/POST | List/search sessions | `fields`, `expression`, `length`, `start`, `flatten`, `date`, `order`, `facets` |
| `/api/session/:id` | GET | Single session JSON (all fields) | `flatten`, `date` |
| `/api/session/:nodeName/:id/detail` | GET | Session detail (HTML) | |
| `/api/stats` | GET | ES stats | |
| `/api/dstats` | GET | Detailed stats over time | `nodeName`, `name`, `start`, `stop`, `step`, `interval` |
| `/api/files` | GET | PCAP files | `sortField`, `desc`, `filter`, `length`, `start` |
| `/api/eshealth` | GET | ES cluster health | |
| `/api/fields` | GET | Available session fields | `array=true` for array format |
| `/api/valueactions` | GET | Right-click actions | |
| `/api/reversedns` | GET | Reverse DNS | `ip` |
| `/api/users` | GET/POST | User management | |

Session fields use dot notation with `flatten=1`: `source.ip`, `destination.port`, `http.uri`, `dns.host`, etc.
Expression syntax: `ip.src == 10.0.0.1 && protocols == tls`
Sort: `order=field:asc` or `order=field:desc`
Pagination: `start=0&length=100` (offset-based)
Facets: `facets=1` adds `graph` object with histogram arrays to response (slower)

## Ratatui patterns used
- `Layout` with `Constraint::Length` (fixed) / `Constraint::Min` (fill) for vertical splits
- `Table` with `Row`/`Cell` for session list, `widths` array sets column sizes
- `TableState` + `render_stateful_widget` for auto-scrolling table selection
- `Paragraph` with `Vec<Line<Span>>` for detail view, `.scroll((y, 0))` for scrolling
- `Tabs` widget for top tab bar with `.select()` and `.highlight_style()`
- `Block::default().borders(Borders::ALL).title()` / `.title_bottom()` wraps most widgets
- `Style::default().fg(Color::X).add_modifier(Modifier::BOLD)` for styling
- Direct buffer writes (`f.buffer_mut()`) for custom graph rendering with block chars

## Crate versions
ratatui 0.29, crossterm 0.28, tokio 1 (full), reqwest 0.12 (rustls-tls), serde/serde_json 1, anyhow 1, urlencoding 2, digest_auth 0.3, rpassword 7, clap 4, chrono
