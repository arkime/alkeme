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
- `ArkimeField` — Deserialized field definition with `dbField`, `type`, `exp` (expression name), `friendlyName`, `regex` (Option), `noFacet` (Option). `is_visible()` returns false for fields with regex or noFacet="true".
- `AuthMode` — Enum: `None` | `Basic` | `Digest` | `Form`.
- `GraphData` — Deserialized histogram data from `facets=1` API response.
- `TableState` — ratatui widget state for session/stats list scrolling.
- `DetailActionMenu` — Popup for adding a field/value to expression from session detail. Options: AND/AND NOT/OR/OR NOT. Stores `field` (exp name for expressions), `display` (friendlyName for UI), `value`, `selected` index, `values` (for array value picker), and `value_selected`.
- `ActionScope` — Enum: `Visible` | `Matching`. For ALL PCAP/CSV actions, selects between visible session IDs or all matching sessions.
- `SummaryMetric` — Enum: `Sessions` | `Packets` | `Bytes`. Selects which metric to display in Arkime summary bar chart and sort column.
- `SummaryItem` — Deserialized summary API item with `item` (Value), `sessions`, `packets`, `bytes` (u64).
- `SessionDetail` — Holds detail data, scroll position, selected row, total_rows, and `filter` string for live field filtering.
- `Packet` — Parsed packet hex dump: `src` (bool), `bytes` (u32), `timestamp` (Option<u64>), `flags` (String), `lines` (Vec<String>).
- `PacketsData` — Holds parsed packets, src/dst column labels, and total packet count. Displayed as a separate overlay via `p` key.
- `LineMode` — Enum: `Off` | `Hex` | `Decimal`. Cycles line number display in packets view.
- `FetchClient` — Lightweight Send-able clone of `ArkimeClient` auth state for background fetches via `tokio::spawn`. Has `fetch_url` (GET) and `fetch_post` (POST with form data) methods.
- `ColumnDef` — Dynamic column definition: `field` (dbField String), `exp` (expression name String), `label` (String), `width` (u16). `default_columns()` returns the default set.
- `ColumnEditorItem` — Column editor entry: `db_field`, `exp`, `friendly_name`, `enabled` (bool). Built from `all_fields`.
- `SavedLayout` — Server-stored layout: `name`, `columns` (Vec<String>), `sort_field`, `sort_dir`.
- `ColumnEditorMode` — Enum: `Browse` | `Reorder`. Controls column editor key behavior.
- `LayoutPopupMode` — Enum: `List` | `SaveInput` | `ConfirmDelete`. Controls layout popup state.
- `ArkimeView` — Server view: `id`, `name`, `expression`, `user`, `shared` (bool). Fetched from `/api/views`.
- `ViewPopupMode` — Enum: `List` | `SaveInput` | `ConfirmDelete`. Controls view popup state.
- `HttpLogEntry` — Records HTTP request: timestamp, method, url, post_data, status, first_byte_ms, last_byte_ms. Stored in `HttpLog` (`Arc<Mutex<Vec<HttpLogEntry>>>`), shared between `ArkimeClient` and `FetchClient`.
- Session data is `serde_json::Value` (not typed structs) since Arkime fields are dynamic.
- Stats data is also `serde_json::Value` — column definitions are in `StatsTab::columns()`.

## Current keybindings

| Key | Action |
|---|---|
| Tab / Shift+Tab | Switch tabs |
| j / k / ↑ / ↓ | Navigate sessions/stats |
| Shift+↑ / Shift+↓ | Page up/down in list, detail, or packets |
| ← / → | Previous/next page (sessions); jump to top/bottom (detail/stats detail/arkime/packets); in expression input, move cursor |
| Shift+← / Shift+→ | First/last page |
| Home / End | First page; in expression input, cursor to start/end |
| PgUp / PgDn | Page up/down in detail, stats detail, or packets view |
| Enter | Open session/stats detail; in detail or summary, open expression menu |
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
| p | View packet hex dump (session list or detail) |
| c | Columns & layouts menu |
| v | Views (select/create/delete views) |
| D | HTTP debug log overlay |
| h / ? | Show context-sensitive help overlay |
| q | Quit |

## Session columns (default)

ipProtocol (4-char mapped: TCP/UDP/ICMP/ICM6/etc), firstPacket, lastPacket, source.ip,
source.port, destination.ip, destination.port, protocol (array, comma-joined),
source.packets, destination.packets, source.bytes, destination.bytes

Columns are now dynamic via `ColumnDef` struct and `App.columns: Vec<ColumnDef>`. `session_fields` stays in sync via `sync_session_fields()`. `c` opens the Columns & Layouts popup: "Edit Columns" opens the column editor (toggle/reorder fields, `/` filter, exp names); "Save Current Layout" saves to Arkime API; saved layouts can be loaded or deleted (`x`). Layout API: `/api/user/layouts/sessionstable`; `/` filters layouts.

## Column Layout API

- `GET /api/user/layouts/sessionstable` — returns array of layout objects (requires `x-arkime-cookie` header)
- `POST /api/user/layouts/sessionstable` — body: `{name, columns: [field_names], order: [[sortField, dir]]}`
- `PUT /api/user/layouts/sessionstable` — same body, updates existing
- `DELETE /api/user/layouts/sessionstable/:name` — deletes layout
- All mutating calls require `x-arkime-cookie` header (CSRF token from `ARKIME-COOKIE` response cookie)
- `ArkimeClient::fetch_cookie()` captures the cookie at startup for all auth modes
- `authenticated_get_with_cookie()` sends `x-arkime-cookie` header for GET endpoints with `checkCookieToken`
- `extract_cookie()` helper shared between `login()` and `fetch_cookie()`

## Views

- `v` opens the Views popup from Sessions or Arkime tabs
- Views are server-side saved expressions that filter sessions
- `GET /api/views` — returns `{data: [{id, name, expression, user, ...}], recordsTotal, recordsFiltered}`
- `POST /api/view` — body: `{name, expression}` — creates a new view (requires cookie)
- `DELETE /api/view/:id` — deletes a view (requires cookie)
- View query param: `&view=viewname` added to session/summary/pcap/csv/tag API calls
- Server resolves view by name or id, applies the view's expression as an additional filter
- Shared views (created by other users) shown with 🔗 indicator, cannot be deleted
- Active view shown in title bar: `Sessions [view: MyView] [1-100 of 742]`
- State: `active_view: Option<String>`, `saved_views: Vec<ArkimeView>`, `show_view_popup`, `view_popup_mode`, `view_popup_selected`, `view_filter`
- Popup: "Save Current Expression as View" (top), "Clear Active View", then saved views with filter

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
- Detail overlay supports full navigation: ↑/↓ (line), Shift+↑/↓ and PgUp/PgDn (page), ←/Home (top), →/End (bottom)
- `/` filters fields in detail overlay; `h`/`?` shows help

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
- Field selector popup (`f`): type-to-filter, shows `exp (friendlyName)`, Enter to select
- `/` opens expression editor (same as Sessions tab)
- Bar chart: ratatui `BarChart` widget showing top values for selected metric (cyan bars)
- Table view: columns are Value, Sessions, Packets, Bytes (bytes human-readable via `format_human_bytes`)
- `G` cycles bar chart metric: Sessions → Packets → Bytes
- `s` cycles sort column: Value → Sessions → Packets → Bytes; `S` toggles sort direction
- Enter on a table row opens AND/AND NOT/OR/OR NOT expression menu (reuses `DetailActionMenu`)
- Expression changes auto-refresh summary data
- Sort indicators (▲/▼) shown in table header on active sort column
- Sort is client-side on already-fetched data
- `t`/`T` changes time range and re-fetches; `r` refreshes
- Navigation: ↑/↓ (row), Shift+↑/↓ and PgUp/PgDn (page), ←/Home (top), →/End (bottom)
- Graph is hidden on this tab (only shown on Sessions tab)
- State: `all_fields` (Vec<ArkimeField>), `summary_field`, `summary_data` (Vec<SummaryItem>), `summary_metric`, `summary_sort`, `summary_sort_desc`, `field_filter`, `field_filter_selected`
- Fetch is async via `tokio::spawn` + `FetchClient::fetch_post` — shows walking owl loading popup during fetch
- `pending_summary_fetch` flag triggers spawn in main loop; result polled via `JoinHandle::is_finished()`
- Fields with `regex` or `noFacet="true"` are hidden from field selector (`is_visible()` filter)

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
- `p` opens packet hex dump overlay (fetched from `/api/session/:node/:id/packets?base=hex`)
- Left/Right arrows jump to top/bottom of field list

## Packet hex dump overlay

- `p` key opens a full-screen overlay from session list or session detail
- Fetches from `/api/session/:node/:id/packets?base=hex&ts=true&packets=10000` (returns HTML, not JSON)
- `r` toggles `showFrames=true` param — shows individual frames with TCP flags (syn, ack, psh, etc.)
- `l` cycles client-side line offset display: hex (`0000:`) → decimal (`    0:`) → off
- Timestamps displayed as `HH:MM:SS.mmm` in DarkGray before each packet header
- TCP flags shown in header line (e.g. `── syn ack psh 60 bytes ──`)
- Two-column layout: source (cyan, left) and destination (green, right)
- Column headers parsed from HTML `srccol`/`dstcol` spans
- Hex offset (`0000:`, `0010:`, etc.) shown in DarkGray before each line
- HTML entities decoded: `&#39;` → `'`, `&#47;` → `/`, `&amp;` → `&`, `&lt;` → `<`, `&gt;` → `>`, `&nbsp;` → ` `, `&quot;` → `"`
- Total packet count from `source.packets + destination.packets` in session metadata (not div count, since Arkime combines packets)
- Title shows scroll percentage
- Navigation: ↑/↓ (line), Shift+↑/↓ and PgUp/PgDn (page), ← (top), → (bottom)
- State: `packets_view: Option<PacketsData>`, `packets_scroll: u16`, `packets_raw: bool`, `packets_line: LineMode`
- Fetch is async via `tokio::spawn` + `FetchClient` — main loop keeps drawing during fetch
- Loading popup with walking owl shown for sessions with >500 packets (`show_loading` flag)
- `pending_packets_fetch` flag triggers spawn in main loop; result polled via `JoinHandle::is_finished()`
- `FetchClient` is a lightweight Send-able clone of auth state from `ArkimeClient::clone_for_fetch()`

## Context-sensitive help

- `h` or `?` shows help overlay tailored to the current view
- 8 contexts: Sessions list, Session detail, Packets view, Stats list, Stats detail, Arkime summary, Column editor, Layouts
- Help renders last in draw order (on top of all overlays including packets)
- Uses `macro_rules! hdr` for section headers to avoid closure lifetime issues

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
| `/api/session/:node/:id/packets` | GET | Packet hex dump (HTML) | `base`, `ts`, `packets`, `showFrames` |
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
ratatui 0.29, crossterm 0.28, tokio 1 (full), reqwest 0.12 (rustls-tls), serde/serde_json 1, anyhow 1, urlencoding 2, digest_auth 0.3, rpassword 7, clap 4, chrono, regex 1
