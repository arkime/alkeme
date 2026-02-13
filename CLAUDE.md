# CLAUDE.md - Alkeme Development Guide

## What is this?
Rust/ratatui TUI for Arkime. Talks to the Arkime viewer REST API. Run: `cargo run -- http://localhost:8005`

## Build
```
cargo build                                                    # debug
cargo run -- URL                                               # no auth
cargo run -- URL --auth digest --user admin:admin              # digest auth
cargo run -- URL --auth basic --user admin:admin               # basic auth
cargo run -- URL --auth form --user admin:admin                # form auth (cookie-based)
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
- `InputMode` — Enum: `Normal` | `Expression` | `ActionPrompt` | `DetailFilter` | `FieldSelector`. Controls where key input is routed.
- `SessionView` — Enum: `List` | `Detail`. Controls which session sub-view renders.
- `StatsTab` — Enum: `Capture` | `DBStats` | `DBIndices`. Sub-tabs within Stats tab.
- `StatsView` — Enum: `List` | `Detail`. Controls which stats sub-view renders.
- `StatsDetail` — Holds detail data + scroll position for stats detail overlay.
- `GraphType` — Enum: `Sessions` | `Packets` | `Bytes`. Selects which histogram to display.
- `GraphSize` — Enum: `Off` | `Small` (10 rows) | `Large` (20 rows). Three-state graph toggle.
- `ArkimeClient` — Wraps `reqwest::Client` + `base_url` + auth. All API calls return `Result<T>`.
- `ArkimeField` — Deserialized field definition with `dbField`, `type`, `exp` (expression name), `friendlyName`.
- `AuthMode` — Enum: `None` | `Basic` | `Digest` | `Form`.
- `GraphData` — Deserialized histogram data from `facets=1` API response.
- `TableState` — ratatui widget state for session/stats list scrolling.
- `DetailActionMenu` — Popup for adding a field/value to expression from session detail. Options: AND/AND NOT/OR/OR NOT. Stores `field` (exp name for expressions), `display` (friendlyName for UI), `value`, `selected` index, `values` (for array value picker), and `value_selected`.
- `ActionScope` — Enum: `Visible` | `Matching`. For ALL PCAP/CSV actions, selects between visible session IDs or all matching sessions.
- `SummaryMetric` — Enum: `Sessions` | `Packets` | `Bytes`. Selects which metric to display in Arkime summary bar chart and sort column.
- `SummaryItem` — Deserialized summary API item with `item` (Value), `sessions`, `packets`, `bytes` (u64).
- `SessionDetail` — Holds detail data, scroll position, selected row, total_rows, and `filter` string for live field filtering.
- Session data is `serde_json::Value` (not typed structs) since Arkime fields are dynamic.
- Stats data is also `serde_json::Value` — column definitions are in `StatsTab::columns()`.

## Current keybindings

| Key | Action |
|---|---|
| Tab / Shift+Tab | Switch tabs |
| j / k / ↑ / ↓ | Navigate sessions/stats |
| Shift+↑ / Shift+↓ | Page up/down in list or detail |
| ← / → | Previous/next page (sessions); in expression input, move cursor |
| Shift+← / Shift+→ | First/last page |
| Home / End | First page; in expression input, cursor to start/end |
| PgUp / PgDn | Page up/down in detail view |
| Enter | Open session/stats detail; in detail, open expression menu |
| Esc | Close overlay |
| r | Refresh data |
| / | Search expression or filter (Enter to apply, Esc to cancel); in session detail, live-filter fields |
| t / T | Cycle time range forward/backward (sessions) |
| s | Next sort column |
| S | Toggle sort direction (asc/desc) |
| g | Cycle graph: Off → Small → Large → Off (sessions tab only) |
| G | Cycle graph type: Sessions → Packets → Bytes (sessions); cycle bar chart metric (arkime) |
| a | Session action menu (download pcap, add/remove tags) |
| A | All sessions action menu (download pcap, export csv, add/remove tags) — pcap/csv show Visible/Matching scope selector |
| 1 / 2 / 3 | Switch stats sub-tab (Capture/DB Stats/DB Indices) |
| f | Open field selector (arkime tab) |
| h / ? | Show help overlay |
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

## Stats tab

- Has 3 sub-tabs switchable with `1/2/3` keys: Capture Stats, DB Stats, DB Indices
- Each sub-tab defines its own columns via `StatsTab::columns()` returning `(field, label, width)` tuples
- Stats tab has its own layout: no time range picker, no graph — just sub-tab bar + filter + table
- Filter (`/`) is passed as `filter` query param to the API (server-side filtering)
- Auto-refreshes every 30 seconds when on the Stats tab
- Numeric/size columns are right-justified
- Enter opens a detail overlay showing all fields for the selected row

### Stats columns per sub-tab
- **Capture Stats**: nodeName, currentTime (formatted date), monitoring (Sessions), freeSpaceM (human-readable + percent), deltaPackets, deltaBytesPerSec (human-readable), deltaSessions, deltaDropped
- **DB Stats**: name (Node), storeSize (Disk Used, human-readable), docs, searches, searchesTime, version
- **DB Indices**: index, status, health, docs.count (nested field), store.size (human-readable), pri (Shards)

### Human-readable byte formatting
- `format_human_bytes()` uses Ki/Mi/Gi/Ti units (1024-based)
- `format_human_megabytes()` converts MB input to bytes then formats
- `parse_size_string()` parses strings like "10.2gb" from ES API into bytes
- `format_epoch_secs()` converts epoch seconds to `YYYY/MM/DD HH:MM:SS`

### Nested field access
- `get_nested_value()` tries flat key first (e.g., `"store.size"`), then dot-separated path (e.g., `"docs"` → `"count"`)

## Arkime (Summary) tab

- Calls `POST /api/sessions/summary` with `fields` in form body; response is a streamed JSON array
- Field selector popup (`f` or `/`): type-to-filter, shows `exp (friendlyName)`, Enter to select
- Bar chart: ratatui `BarChart` widget showing top values for selected metric (cyan bars)
- Table view: columns are Value, Sessions, Packets, Bytes (bytes human-readable via `format_human_bytes`)
- `G` cycles bar chart metric: Sessions → Packets → Bytes
- `s` cycles sort column: Sessions → Packets → Bytes; `S` toggles sort direction
- Sort indicators (▲/▼) shown in table header on active sort column
- Sort is client-side on already-fetched data
- `t`/`T` changes time range and re-fetches; `r` refreshes
- Graph is hidden on this tab (only shown on Sessions tab)
- State: `all_fields` (Vec<ArkimeField>), `summary_field`, `summary_data` (Vec<SummaryItem>), `summary_metric`, `summary_sort`, `summary_sort_desc`, `field_filter`, `field_filter_selected`

## Owl animation
- Settings tab shows a 90s "Under Construction" page with animated owl
- Owl bounces around the content area, flipping direction at edges
- Two walking frames alternate every 75ms
- Construction banner cycles through rainbow colors, barricade bars scroll
- Animation state: `owl_x/y/dx/dy/frame/tick` + `anim_start` (never-reset Instant for color cycling)

## Session detail expression builder

- In session detail view, rows are selectable with ↑/↓ arrows (highlighted in yellow)
- Auto-scrolls to keep selected row visible
- Enter opens an action menu with 4 options: AND value, AND NOT value, OR value, OR NOT value
- For array fields with multiple values, a "Select Value" picker appears first before the AND/OR menu
- Single-element arrays skip the picker and go straight to AND/OR options
- The AND/OR menu title shows `fieldName = value` for the selected value
- If expression is empty, adds `field == value` directly
- If expression is non-empty, prepends `&&` or `||` connector
- String values are quoted, numeric values are not
- Esc closes the action menu and returns to session detail
- `DetailActionMenu` holds `field` (exp name), `display` (friendlyName), value string, selected menu index, `values` (Option<Vec<String>> for array picker), and `value_selected`
- `field_exp_map` maps dbField → exp name, `field_friendly_map` maps dbField → friendlyName
- Session detail labels show friendlyName; expressions use exp name
- Fields ending in `Cnt` and `packetPos`/`packetRange`/`packetLen` are hidden from session detail
- `/` activates a live field filter (case-insensitive substring match on dbField and friendlyName)
- Filter text shown in title bar; Esc clears filter, Enter keeps filter active

## User API

- At startup, `/api/user` is called and the response stored as `serde_json::Value` in `App.user`
- `removeEnabled` controls whether "Remove Tags" appears in action menus
- `App::remove_enabled()` helper checks `user["removeEnabled"]`

## Expression input

- Expression and stats filter inputs support full cursor movement (Left/Right/Home/End/Delete)
- `expression_cursor` tracks cursor position within the edit string
- Characters insert at cursor position; Backspace deletes before cursor; Delete deletes at cursor

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
2. Use `self.authenticated_get(&url).await?` for GET requests, `self.authenticated_post(&url, &form).await?` for POST
3. Use `authenticated_get_bytes`/`authenticated_post_bytes` for binary responses (pcap, csv)
4. Parse with `serde_json::from_str` — use `Value` for dynamic data, typed structs for fixed schemas
5. Add response struct with `#[derive(Deserialize)]` if needed, use `#[serde(rename = "camelCase")]` for JS field names
6. Call from `App` method in `app.rs`, store result in App fields

### Adding keybindings
1. Key handling is in `app.rs`: `handle_key()` dispatches based on active tab and view
2. Sessions: `handle_list_key()` for list, `handle_detail_key()` for detail
3. Stats: `handle_stats_key()` for list, `handle_stats_detail_key()` for detail
4. Expression/filter input: `handle_expression_key()` is context-aware (sessions vs stats)
5. Global keys (Ctrl+C, q) are in `main.rs::run_app()`
6. For new tabs with their own keys, add a `handle_<tab>_key()` method
7. Update help text in `draw_help()` in `ui.rs`

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
| `/api/sessions.pcap` | GET | Download PCAP | `expression`, `date`, `ids` |
| `/api/sessions/csv` | GET | Export CSV | `expression`, `date`, `fields`, `ids` |
| `/api/session/:id` | GET | Single session JSON (all fields) | `flatten`, `date` |
| `/api/session/:nodeName/:id/detail` | GET | Session detail (HTML) | |
| `/api/sessions/summary` | POST | Summary/aggregation by field | `fields` (form body), `expression`, `date` |
| `/api/stats` | GET | Capture node stats | `sortField`, `desc`, `filter` |
| `/api/esstats` | GET | DB/ES node stats | `sortField`, `desc`, `filter` |
| `/api/esindices` | GET | DB/ES indices | `sortField`, `desc`, `filter` |
| `/api/dstats` | GET | Detailed stats over time | `nodeName`, `name`, `start`, `stop`, `step`, `interval` |
| `/api/files` | GET | PCAP files | `sortField`, `desc`, `filter`, `length`, `start` |
| `/api/eshealth` | GET | ES cluster health | |
| `/api/fields` | GET | Available session fields | `array=true` for array format |
| `/api/user` | GET | Current user profile | returns `removeEnabled`, etc. |
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
