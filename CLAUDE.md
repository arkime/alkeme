# CLAUDE.md - Alkeme Development Guide

## What is this?
Rust/ratatui TUI for Arkime ecosystem. Auto-detects app mode (Viewer, Cont3xt, WISE, Parliament) via `/api/appversion`. Run: `cargo run -- http://localhost:8005`

## Build
```
cargo build                                                    # debug
cargo run -- URL                                               # no auth
cargo run -- URL --auth digest --user admin:admin              # digest auth
cargo run -- URL --auth basic --user admin:admin               # basic auth
cargo run -- URL --auth form --user admin:admin                # form auth (cookie-based)
cargo run -- URL --auth web --user admin:admin                 # web auth (HTML form parsing)
cargo run -- URL --auth okta --user admin:admin                # okta auth (Identity Engine + classic)
cargo run -- URL --auth okta                                   # okta auth (prompts with Okta's labels)
cargo run -- URL --auth digest                                 # prompts for both user+pass
cargo run -- URL --auth digest --user admin                    # prompts for password only
cargo run -- URL --auth digest --user admin --password secret  # password from flag
cargo run -- URL --auth digest --user admin --password '|pass show key'  # password from command
cargo run -- URL --app cont3xt --auth form --user admin:admin  # force cont3xt mode
cargo run -- URL --search 1.2.3.4 --cont3xt-tags mytag       # cont3xt with tags
cargo run -- URL --auth okta --jar cookies.json               # encrypted cookie jar (saves username + cookies)
cargo run -- URL --auth okta --jar cookies.json --jar-password secret  # jar with literal password
cargo run -- URL --auth okta --jar cookies.json --jar-password '|pass show key'  # jar password from command
cargo run -- URL --cont3xt-read-json results.json --app cont3xt  # load saved cont3xt results from file
```

## Architecture

```
src/
  main.rs              - Entry point, clap CLI parsing, terminal setup, event loop (crossterm polling), drain_c3_results()
  app/mod.rs           - App struct, state, fetch methods, enter_expression_mode(), handle_text_input_key()
  app/types.rs         - Enums (AppMode, Tab, TimeRange, InputMode, etc.)
  app/keys.rs          - Key dispatch + expression handler
  app/keys_viewer.rs   - Viewer key handlers
  app/keys_cont3xt.rs  - Cont3xt key handler
  app/keys_parliament.rs - Parliament key handler
  app/keys_wise.rs     - WISE key handler
  api/mod.rs           - ArkimeClient + FetchClient: HTTP calls (reqwest + digest_auth), finish_response(), digest_auth_header()
  api/types.rs         - Shared types (structs, enums, parsing functions), str_val()
  api/auth.rs          - Web + Okta authentication flows
  api/viewer.rs        - Viewer API methods (vr_*), vr_get_sorted_filtered(), vr_bulk_tag_op()
  api/cont3xt.rs       - Cont3xt API methods (c3_*)
  api/parliament.rs    - Parliament API methods (pl_*)
  api/wise.rs          - WISE API methods (ws_*)
  ui/mod.rs            - Draw dispatch, common layout (tabs, toolbar, status bar, graph), center_popup(), sort_header_style(), sort_header_label(), render_text_input(), format_number()
  ui/sessions.rs       - Session list/detail rendering
  ui/stats.rs          - Stats tab rendering
  ui/arkime.rs         - Summary tab + owl animation rendering
  ui/cont3xt.rs        - Cont3xt search/results/card rendering
  ui/parliament.rs     - Parliament dashboard + issues rendering
  ui/wise.rs           - WISE stats + query rendering
  ui/popups.rs         - Help overlays, debug log, action menus
```

## Shared helpers

- `center_popup(width, height, area) -> Rect` — Center a popup within an area (ui/mod.rs)
- `sort_header_style(is_sorted) -> Style` — Cyan+bold if sorted, yellow+bold otherwise (ui/mod.rs)
- `sort_header_label(label, is_sorted, is_desc) -> String` — Append ▲/▼ arrow if sorted (ui/mod.rs)
- `render_text_input(f, text, cursor, is_editing, title, area)` — Render scrollable text input with cursor (ui/mod.rs)
- `format_number(n) -> String` — Format u64 with comma separators (ui/mod.rs)
- `str_val(v, key) -> String` — Extract string from serde_json::Value or return empty string (api/types.rs)
- `enter_expression_mode(&mut self)` — Enter expression editing mode with cursor at end (app/mod.rs)
- `handle_text_input_key(key, text, cursor) -> bool` — Handle cursor-based text input keys (app/mod.rs)
- `drain_c3_results(app, results_arc, sent_arc, total_arc)` — Drain streaming cont3xt results into app state (main.rs)
- `FetchClient::finish_response(...)` — Consolidate timing, status check, logging for HTTP responses (api/mod.rs)
- `FetchClient::digest_auth_header(...)` — Build digest auth Authorization header (api/mod.rs)
- `ArkimeClient::vr_get_sorted_filtered(...)` — Shared stats endpoint with sort/filter params (api/viewer.rs)
- `ArkimeClient::vr_bulk_tag_op(...)` — Shared add/remove tag operation (api/viewer.rs)

## App Mode / Multi-App

- At startup, `/api/appversion` is the first API call (after login/cookie)
- `result.app` determines `AppMode`: "viewer" (default if empty), "cont3xt", "wise"/"wiseService", "parliament"
- If `/api/appversion` fails, exits with "please upgrade to Arkime 6" message
- `--app <mode>` CLI flag skips appversion call and forces a mode
- `/api/user` provides user info (from `result.user` in appversion response)
- Each mode has its own tab set via `AppMode::tabs()`:
  - **Viewer**: Arkime, Sessions, Stats, Settings (defaults to Sessions)
  - **Cont3xt**: Search, Stats, History, Settings (defaults to Search)
  - **Parliament**: Dashboard, Issues, Settings (defaults to Dashboard)
  - **Wise**: Stats, Query, Settings (defaults to Stats)
- `AppMode::default_tab()` returns the starting tab for each mode
- UI rendering routes through `draw_viewer()`, `draw_cont3xt()`, `draw_parliament()`, or `draw_wise()` based on mode
- Key handling routes through mode-specific handlers in `handle_key()`
- Viewer-specific background tasks (packets/summary fetch, stats auto-refresh) only run in Viewer mode
- `--user username` (no colon) prompts only for password; `--auth` with no `--user` prompts for both

## Key types

- `App` — All mutable state. Passed as `&mut` to handlers and renderers. Viewer fields prefixed `vr_`, cont3xt fields prefixed `c3_`, parliament fields prefixed `pl_`, WISE fields prefixed `ws_`. Public methods follow same convention.
- `AppMode` — Enum: `Viewer` | `Cont3xt` | `Wise` | `Parliament`. Determined at startup from `/api/appversion` `result.app` or `--app` flag. Has `tabs()`, `default_tab()`, `label()`.
- `Tab` — Enum: `Arkime` | `Sessions` | `Stats` | `Search` | `C3Stats` | `History` | `Dashboard` | `Issues` | `WsStats` | `WsQuery` | `Settings`. Which tabs are available depends on `AppMode::tabs()`.
- `TimeRange` — Enum: Minutes15..All. Has `label()`, `date_value()`, `next()`, `prev()`.
- `InputMode` — Enum: `Normal` | `Expression` | `ActionPrompt` | `DetailFilter` | `FieldSelector`. Controls where key input is routed.
- `SessionView` — Enum: `List` | `Detail`. Controls which session sub-view renders.
- `StatsTab` — Enum: `Capture` | `DBStats` | `DBIndices`. Sub-tabs within Stats tab. DBStats labeled "DB Nodes".
- `StatsView` — Enum: `List` | `Detail`. Controls which stats sub-view renders.
- `StatsDetail` — Holds detail data + scroll position for stats detail overlay.
- `GraphType` — Enum: `Sessions` | `Packets` | `Bytes`. Selects which histogram to display.
- `GraphSize` — Enum: `Off` | `Small` (10 rows) | `Large` (20 rows). Three-state graph toggle.
- `ArkimeClient` — Wraps `reqwest::Client` + `base_url` + auth. All API calls return `Result<T>`.
- `ArkimeField` — Deserialized field definition with `dbField`, `type`, `exp` (expression name), `friendlyName`, `regex` (Option), `noFacet` (Option). `is_visible()` returns false for fields with regex or noFacet="true".
- `AuthMode` — Enum: `None` | `Basic` | `Digest` | `Form` | `Web` | `Okta`.
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
- `HttpLogEntry` — Records HTTP request: timestamp, method, url, post_data, status, first_byte_ms, last_byte_ms, response_body (Option, first 4096 chars for non-200). Stored in `HttpLog` (`Arc<Mutex<Vec<HttpLogEntry>>>`), shared between `ArkimeClient` and `FetchClient`.
- `Cont3xtFocus` — Enum: `Results` | `Detail`. Controls which pane has focus in cont3xt search.
- `Cont3xtIntegration` — Integration definition from `/api/integration`: name, doable, order, card (Option<Cont3xtCard>).
- `Cont3xtCard` — Card display definition: title, fields (Vec<CardField>).
- `CardField` — Card field: label, field (dot-joined path), field_type (string/url/date/ms/seconds/array/table/json/dnsRecords), join, fields (sub-fields for tables), defang, field_root, filter_empty.
- `Cont3xtResult` — Search result from one integration: name, indicator, itype, data (Value), has_data.
- `Cont3xtOverview` — Overview definition: id, name, title, itype, is_default, fields (Vec<Cont3xtOverviewField>).
- `Cont3xtOverviewField` — Overview field: field_type ("linked"/"custom"), from (integration name), field (card field label), alias (display name), custom (Option<Value>).
- `C3TreeItem` — Enum: `Indicator(itype, query)` | `Result(usize)`. Entry in the results tree; has `result_idx()` helper.
- `Cont3xtLink` — Link definition: name, url, itypes (Vec<String>), info (String). From link groups API.
- `Cont3xtLinkGroup` — Link group: name, links (Vec<Cont3xtLink>). Fetched from `/api/linkGroup`.
- `PlGroup` — Parliament group: title, description, clusters (Vec<PlCluster>).
- `PlCluster` — Parliament cluster: id, title, description, url, cluster_type (disabled/multiviewer/noAlerts/"").
- `PlClusterStats` — Cluster stats: status, health_error, stats_error, es_version, delta_bps, delta_tdps, monitoring, arkime_nodes, data_nodes, total_nodes.
- `PlIssue` — Issue: cluster_id, cluster, issue_type, title, text, message, severity (red/yellow), node, first_noticed, last_noticed, acknowledged, ignore_until.
- `PlIssueSort` — Enum: `Cluster` | `Title` | `Severity` | `FirstNoticed` | `LastNoticed`. Issue sort field selector.
- `ConfirmDialog` — Generic confirmation popup: `title` (String), `message` (String), `action` (String). Action string parsed by `handle_confirm()` (e.g., `"delete_esindex:name"`, `"esshards:name:node1:exclude"`). Used for index ops and node exclude/include.
- Session data is `serde_json::Value` (not typed structs) since Arkime fields are dynamic.
- Stats data is also `serde_json::Value` — column definitions are in `StatsTab::columns()`.

## Viewer keybindings

| Key | Action |
|---|---|
| Tab / Shift+Tab | Switch tabs |
| j / k / ↑ / ↓ | Navigate sessions/stats |
| Shift+↑ / Shift+↓ | Page up/down in list, detail, or packets |
| ← / → | Previous/next page (sessions); jump to top/bottom (detail/stats detail/arkime/packets); in expression input, move cursor |
| Shift+← / Shift+→ | First/last page; word jump in expression input |
| Home / End | First page; in expression input, cursor to start/end |
| PgUp / PgDn | Page up/down in detail, stats detail, or packets view |
| Enter | Open session/stats detail; in detail or summary, open expression menu |
| Esc | Close overlay |
| r | Refresh data |
| / or E | Search expression or filter (Enter to apply, Esc to cancel); in session detail, live-filter fields |
| t / T | Cycle time range forward/backward (sessions) |
| s | Next sort column |
| S | Toggle sort direction (asc/desc) |
| g | Cycle graph: Off → Small → Large → Off (sessions tab only) |
| G | Cycle graph type: Sessions → Packets → Bytes (sessions); cycle bar chart metric (arkime) |
| a | Session action menu (download pcap, add/remove tags) |
| A | All sessions action menu (download pcap, export csv, add/remove tags) — pcap/csv show Visible/Matching scope selector |
| 1 / 2 / 3 | Switch stats sub-tab (Capture/DB Nodes/DB Indices) |
| f | Open field selector (arkime tab) |
| p | View packet hex dump (session list or detail) |
| c | Columns & layouts menu |
| v | Views (select/create/delete views) |
| d | Delete index (DB Indices); confirm dialog |
| f | Force merge index (DB Indices); confirm dialog |
| C | Close open index (DB Indices); confirm dialog |
| O | Open closed index (DB Indices); confirm dialog |
| e | Toggle exclude/include node (DB Nodes); confirm dialog |
| x | Toggle exclude/include IP (DB Nodes); confirm dialog |
| D | HTTP debug log overlay (↑/↓ navigate, Enter expand, Esc collapse) |
| h / ? | Show context-sensitive help overlay |
| q | Quit |

## Cont3xt keybindings

| Key | Action |
|---|---|
| Tab / Shift+Tab | Switch tabs |
| j / k / ↑ / ↓ | Navigate results list or scroll detail |
| Shift+↑ / Shift+↓ | Page up/down; jump to next/prev indicator (results) |
| PgUp / PgDn | Page up/down (detail) |
| ← / → | Jump to top/bottom (results); scroll detail left/right |
| Shift+← / Shift+→ | Fast scroll detail left/right; word jump in expression |
| Home | Jump to top, reset horizontal scroll |
| End | Jump to bottom |
| Enter | Open detail panel (results); close detail uses Esc |
| Esc | Return to results from detail; close popups |
| / or E | Search expression or filter (Enter to apply, Esc to cancel) |
| / | Filter detail fields (when in detail panel) |
| E | Edit search indicator (when in detail panel) |
| R | Toggle raw JSON / card view; debug mode for overview |
| C | Card/overview definition popup (detail); s saves to /tmp/alkeme-card.txt |
| o | Select overview (when on indicator header); d:set default, /:filter, r:refresh, h/?:help |
| i | Integration filter popup (Space:toggle, a:all, n:none, !:invert, /:filter) |
| v / Shift+I | Open views popup (select/create/delete integration views) |
| r | Re-run search |
| l | Link groups for selected indicator (Enter opens in browser, / filter) |
| s | Next sort column (History/Stats); cycle source (WISE Query) |
| S | Toggle sort direction (History/Stats) |
| d | Delete history entry (History tab) |
| J | Save all results as JSON (prompts for filename) |
| t | Edit search tags (comma-separated, sent with queries) |
| d | Edit date range for links (start/stop, supports relative: -7d, -1h, now, or absolute: YYYY-MM-DD) |
| ← / → | Previous/next page (History); jump to top/bottom (results); scroll detail |
| D | HTTP debug log overlay (↑/↓ navigate, Enter expand, Esc collapse) |
| h / ? | Show help |
| q | Quit |

## Parliament keybindings

| Key | Action |
|---|---|
| Tab / Shift+Tab | Switch tabs (Dashboard/Issues/Settings) |
| j / k / ↑ / ↓ | Navigate clusters (Dashboard) or issues (Issues) |
| Shift+↑ / Shift+↓ | Page up/down (Issues) |
| Home / End | Jump to top/bottom (Issues) |
| Enter | Open cluster in Viewer mode (Dashboard) |
| i | Cluster detail overlay (Dashboard) |
| c | Open Cont3xt (if configured in Parliament settings) |
| w | Open WISE (if configured in Parliament settings) |
| Ctrl+p | Return to Parliament (from Viewer, Cont3xt, or WISE) |
| / or E | Filter issues (Issues tab) |
| s | Next sort column (Issues) |
| S | Toggle sort direction (Issues) |
| r | Refresh |
| D | HTTP debug log overlay (↑/↓ navigate, Enter expand, Esc collapse) |
| h / ? | Show help |
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
- All mutating calls require `x-arkime-cookie` or `x-cont3xt-cookie` header (CSRF token from `ARKIME-COOKIE` or `CONT3XT-COOKIE` response cookie)
- `ArkimeClient::fetch_cookie()` captures the cookie at startup for all auth modes
- `authenticated_get_with_cookie()` sends the appropriate cookie header for GET endpoints with `checkCookieToken`
- `extract_cookie()` handles both cookie names and sets `cookie_header_name` dynamically

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

- Has 3 sub-tabs switchable with `1/2/3` keys: Capture Stats, DB Nodes, DB Indices
- Each sub-tab has dynamic columns defined by `StatsColumnDef` (field, sort, label, width, format)
- `StatsFormat` enum controls cell rendering: String, Number, Bytes, BytesPerSec, MegaBytes, Percent, EpochSecs, SizeString
- `c` opens column/layout popup (same pattern as session columns): Edit Columns, Save Layout, Default, saved layouts
- Column layouts saved via shareable API with types: `capture-columns`, `esnodes-columns`, `esindices-columns`
- Stats tab has its own layout: no time range picker, no graph — just sub-tab bar + filter + table
- Filter (`/`) is passed as `filter` query param to the API (server-side filtering)
- Auto-refreshes every 30 seconds when on the Stats tab
- Numeric/size columns are right-justified
- Enter opens a detail overlay showing all fields for the selected row
- Detail overlay supports full navigation: ↑/↓ (line), Shift+↑/↓ and PgUp/PgDn (page), ←/Home (top), →/End (bottom)
- `/` filters fields in detail overlay; `h`/`?` shows help
- DB Nodes detail shows exclusion status banner (node/IP excluded state) with `e`/`x` toggle keys
- DB Indices detail supports `d` (delete), `f` (force merge), `C` (close), `O` (open) operations
- DB Nodes list and detail support `e` (toggle node exclude/include) and `x` (toggle IP exclude/include) via `POST /api/esshards/:type/:value/:action`
- DB Indices list supports `d` (delete via `DELETE /api/esindices/:index`), `f` (force merge), `C` (close), `O` (open) via `POST /api/esindices/:index/:action`
- All operations use `ConfirmDialog` for confirmation and auto-refresh stats after success
- Node exclude/include operations refresh detail in-place (preserving scroll and filter)

### Stats columns per sub-tab
- **Capture Stats** (13 default, 37 total): nodeName, currentTime, monitoring, freeSpaceM, cpu, memory, packetQueue, diskQueue, esQueue, deltaPackets, deltaBytesPerSec, deltaSessions, deltaDropped (+ 24 more)
- **DB Nodes** (10 default, 27 total): name, docs, storeSize, freeSize, heapSize, load, cpu, read, write, searches (+ 17 more)
- **DB Indices** (9 default, 16 total): index, docs.count, store.size, pri, segmentsCount, rep, memoryTotal, health, status (+ 7 more)

### Shareable API (stats column layouts)
- `GET /api/shareables?type={tab}-columns` — list saved layouts (no cookie needed)
- `POST /api/shareable` — create: `{name, type, data: {columns: [...], order: [[sortField, dir]]}}`
- `PUT /api/shareable/:id` — update (needs cookie)
- `DELETE /api/shareable/:id` — delete (needs cookie)
- `SavedShareable` struct: id, name, columns, sort_field, sort_dir, shared

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

## Cont3xt mode

### Integration search
- `POST /api/integration/search` with JSON body `{"query":"..."}` — streaming response
- Response is a JSON array, one object per line: `{"purpose":"init",...}`, `{"purpose":"data","name":"IntName","data":{...},"sent":N,"total":N}`, `{"purpose":"fail","name":"IntName","sent":N,"total":N}`, `{"purpose":"finish",...}`
- `FetchClient::fetch_post_json_streaming()` uses `reqwest::Response::bytes_stream()` via `futures_util::StreamExt`
- Parsed results pushed into `Arc<Mutex<Vec<Cont3xtResult>>>`, polled by main event loop every 100ms
- `c3_searching` flag tracks active search; results appear incrementally in the UI
- Progress gauge (`LineGauge`) shown between search bar and results while searching; uses `sent`/`total` from streaming `data`/`fail` messages via `Arc<AtomicU64>` shared with the streaming task
- `c3_search_sent` and `c3_search_total` track progress; both update from every `data` and `fail` message (total can change as sub-indicators are discovered)

### Integration list
- `GET /api/integration` returns `{"success":true,"integrations":{"Name":{doable,order,card,...},...}}`
- Cards are server-normalized: `field` → `path` (array), `fieldRoot` → `fieldRootPath` (array)
- `parse_card_field()` reads `path` array and joins to dot-notation for `get_by_path()` traversal

### Card rendering
- `render_card_lines()` builds `Vec<JsonLine>` from card definition + data
- Supports types: string, url, date, ms, seconds, array (with join), table (with sub-fields), json, dnsRecords
- `get_by_path()` tries full key first (handles flattened ES keys like `"source.ip"`), then nested dot traversal
- `align_table_columns()` post-processes lines to compute max column widths per table block
- `format_table_cells()` pads cells to computed widths; horizontal scroll via `c3_detail_hscroll`
- `R` toggles between card view and raw JSON; `flatten_json_to_lines()` renders raw

### Integration popup
- `i` key opens popup in Integrations mode; `Shift+I` opens directly in Views mode
- In Integrations mode: `/` enters filter mode for type-to-search
- Space/Enter toggles individual integrations; `a` enables all, `n` disables all, `!` inverts
- Manually toggling integrations clears the active view (sets label to "custom")
- `c3_disabled_integrations: HashSet<String>` filters results during streaming search; sent as `doIntegrations` to the search API
- In Views mode: lists saved integration views; selecting a view applies its integration settings
- `IntegrationPopupMode` — Enum: `Integrations` | `Views` | `SaveInput` | `ConfirmDelete`

### Cookie handling
- Cont3xt uses `CONT3XT-COOKIE` (not `ARKIME-COOKIE`)
- CSRF token header is `x-cont3xt-cookie` (not `x-arkime-cookie`)
- `extract_cookie()` handles both cookie names; `cookie_header_name` field set dynamically
- For Form auth, reqwest cookie jar handles session cookies; only CSRF token sent via custom header

### Cont3xt state fields
- `c3_integrations`, `c3_results`, `c3_selected`, `c3_detail_scroll`, `c3_detail_hscroll`, `c3_detail_filter`
- `c3_overviews` — Vec<Cont3xtOverview> fetched from `/api/overview`
- `c3_selected_overviews` — HashMap<String, String> mapping itype → selected overview id
- `c3_show_overview_popup`, `c3_overview_popup_selected`, `c3_overview_popup_filter`, `c3_overview_popup_filtering` — overview selector popup state
- `c3_search_total`, `c3_search_sent`, `c3_search_itype`, `c3_focus`, `c3_raw_view`, `c3_show_card_popup`, `c3_card_popup_scroll`
- `c3_disabled_integrations`, `show_integration_popup`, `integration_popup_selected`, `integration_popup_filter`, `integration_popup_filtering`
- `c3_searching`, `pending_c3_search`
- `c3_tags` — Vec<String> tags sent with search; `c3_tags_edit`, `c3_show_tags_popup` — tag editor state
- `c3_start_date`, `c3_stop_date` — DateTime<Utc> date range for link substitution (defaults: now-7d, now)
- `c3_show_date_popup`, `c3_date_start_edit`, `c3_date_stop_edit`, `c3_date_field` — date range popup state
- `c3_tree_order` — Vec<C3TreeItem> in tree display order; items are Indicator(itype, query) or Result(idx)
- `c3_tree_roots` — indices into `c3_tree_order` where each root indicator starts (for Shift+Up/Down jumping)
- `c3_indicator_parents` — HashMap<(indicator, itype), Vec<(parent_query, parent_itype)>> for tree nesting
- `c3_active_view_id`, `c3_active_view_name` — active integration view ID/name (shown in search bar label)
- `c3_stats_table_state` — TableState for auto-scrolling stats table
- `c3_link_groups`, `c3_show_link_popup`, `c3_link_popup_selected`, `c3_link_popup_filter`, `c3_link_popup_filtering`, `c3_link_flat`
- `c3_history_data`, `c3_history_total`, `c3_history_page` (1-indexed), `c3_history_selected`, `c3_history_table_state`
- `c3_history_filter`, `c3_history_filtering`, `c3_history_sort_col`, `c3_history_sort_desc`, `c3_history_loaded`

### Results tree hierarchy
- Results panel shows indicators in a tree structure: parent indicators contain child indicators
- Link messages in streaming response (`{purpose: "link"}`) establish parent-child relationships
- `c3_indicator_parents` maps (child_indicator, child_itype) → Vec<(parent_query, parent_itype)>
- Children appear under all their parents (multi-parent support for shared IPs, etc.)
- Parent indicators without results are injected into the tree (e.g., URL with no integrations)
- `c3_tree_order` stores `C3TreeItem` entries (Indicator or Result) in display order; 1:1 with display_rows
- `c3_tree_roots` stores indices into `c3_tree_order` where each root indicator starts — enables Shift+Up/Down to jump between indicators
- Both indicator headers and integration results are selectable; `c3_selected` indexes into `c3_tree_order`
- When an Indicator is selected, detail pane shows the overview; when a Result is selected, shows the integration card
- Enter switches focus to detail panel; Esc returns to results; Tab always switches tabs

### Overviews
- `GET /api/overview` returns `{success: true, overviews: [{_id, name, title, iType, fields: [...], ...}]}`
- Default overviews have `_id` == iType name (e.g., "domain", "ip"); `is_default` derived from this
- Each overview field: `{type: "linked", from: "IntegrationName", field: "CardFieldLabel", alias: "DisplayName"}`
- "linked" type means: find result from integration `from`, find card field with label `field`, display data with `alias`
- Overview title supports `%{query}` substitution for the indicator value
- `c3_selected_overviews` maps itype → chosen overview id; falls back to default then first match
- `o` key opens overview selector popup; `R` toggles debug mode showing all fields including missing data
- `C` on an indicator shows overview definition popup; on a result shows card definition

### Link groups
- `l` key opens link groups popup filtered by the selected indicator's itype
- `GET /api/linkGroup` returns `{success, linkGroups: [{name, links: [{name, url, itypes: [], infoField}]}]}`
- Links filtered by itype match; full placeholder substitution in URLs via `substitute_link_url()`
- Popup shows grouped links with description panel; Enter opens URL in browser
- Uses `open` (macOS) or `xdg-open` (Linux) via `std::process::Command`

### Link URL placeholders
- `${indicator}` — refanged query (hXXp→http, [.]→.)
- `${type}` — itype (ip, domain, url, email, hash, phone, text)
- `${numDays}`, `${numHours}` — date range span (defaults: 7 days, 168 hours)
- `${startDate}`, `${endDate}` — YYYY-MM-DD
- `${startTS}`, `${endTS}` — YYYY-mm-ddTHH.MM.SSZ
- `${startEpoch}`, `${endEpoch}` — seconds since epoch
- `${startSplunk}`, `${endSplunk}` — MM/DD/YYYY:HH:MM:SS
- `${start,{JSON}}`, `${end,{JSON}}` — custom date format with `format` and optional `timeSnap` (e.g., `${start,{"format":"DD.MM.YYYY","timeSnap":"1w"}}`)
- `${array,{JSON}}` — array of indicators by iType, with `iType` (required), `include` ("all" = all indicators in results tree including discovered children, "top" = only init packet indicators), `sep`, `quote`
- Helper functions: `refang()`, `convert_date_format()`, `parse_time_snap()`, `process_advanced_placeholder()`
- Date defaults: end=now, start=now-7d (no UI for date range yet)

## Parliament mode

- Tabs: Dashboard, Issues, Settings
- State fields use `pl_` prefix; API methods use `pl_` prefix
- Dashboard shows groups as titled sections with clusters listed below
- Each cluster shows: type icon (⊘ disabled, ⌂ multiviewer, 🔕noAlerts), health indicator (●green/●yellow/●red), title, stats (bps, drops/sec, sessions, nodes, ES info), issue count
- Navigation: ↑/↓ selects cluster via `pl_cluster_list` flat index (group_idx, cluster_idx pairs); dashboard auto-scrolls to keep selected cluster visible (`pl_dashboard_scroll`)
- `i` opens detail overlay with full stats and issues for selected cluster
- `Enter` on a cluster with a URL switches to Viewer mode: creates new `ArkimeClient` with cluster URL, calls `ensure_session()`/`fetch_cookie()`, switches `app_mode` to Viewer, loads fields+sessions. Parliament client saved in `pl_saved_client`. If session cookies don't work, temporarily exits TUI to prompt for username/password.
- `c` on Dashboard switches to Cont3xt mode using `cont3xtUrl` from parliament settings (if configured). Saves parliament client for return. Prompts for credentials if needed.
- `w` on Dashboard switches to WISE mode using `wiseUrl` from parliament settings (if configured). Saves parliament client for return. Prompts for credentials if needed.
- `Ctrl+P` or `q` in Viewer, Cont3xt, or WISE mode (when `pl_saved_client` is Some) restores the parliament client and switches back to Parliament Dashboard
- Issues tab: filterable, sortable table of all cluster issues with severity color coding; uses `TableState` (`pl_issues_table_state`) for automatic scroll tracking
- Filter uses expression handler (`/` or `E`), stored in `pl_issues_filter`
- Sort cycles through: Cluster, Title, Severity, FirstNoticed, LastNoticed via `PlIssueSort`; active sort column shown in Cyan with ▲/▼ arrow, other sortable columns in Yellow
- Auto-refresh: every 30 seconds (dashboard stats + issues), same pattern as viewer Stats tab

### Parliament API endpoints

- Parliament API methods use `pl_base()` which strips trailing `/parliament` from `base_url` to avoid double-path issues (e.g., user provides `http://host/parliament`, endpoints use `/parliament/api/...`)
- `GET /parliament/api/parliament` — returns `{groups: [{title, description, clusters: [{id, title, description, url, type}]}], settings: {general: {cont3xtUrl, ...}}}`
- `GET /parliament/api/parliament/stats` — returns `{results: {clusterId: {status, deltaBPS, deltaTDPS, monitoring, arkimeNodes, dataNodes, totalNodes, esVersion, healthError, statsError}}}`
- `GET /parliament/api/issues` — returns `{issues: [...], recordsFiltered}`. Query params: `map=true` returns `{results: {clusterId: [issues]}}`
- Issue types: esRed, esDown, esDropped, outOfDate, noPackets, lowDiskSpace, lowDiskSpaceES
- Issue severity: "red" or "yellow"
- Cluster types: "" (normal), "multiviewer" (no stats), "disabled" (no monitoring), "noAlerts" (no alerts)

## WISE mode

- Tabs: Stats (sources/types sub-tabs), Query, Settings
- State fields use `ws_` prefix; API methods use `ws_` prefix
- WISE may not require auth — if form auth fails when switching from Parliament, falls back to no auth
- Auto-refresh: stats auto-refresh every 30 seconds on the Stats tab
- `Ctrl+P` returns to Parliament from WISE mode (same as Viewer/Cont3xt)

### WISE keybindings

| Key | Action |
|---|---|
| Tab / Shift+Tab | Switch tabs (Stats/Query/Settings) |
| 1 / 2 | Sources / Types sub-tab (Stats) |
| j / k / ↑ / ↓ | Navigate rows |
| Shift+↑ / Shift+↓ | Page up / down |
| Home / End | Jump to top / bottom |
| / / E | Filter stats or edit query value |
| s | Cycle source (Query) |
| t | Cycle type (Query) |
| Enter | Run query (Query) |
| r | Refresh (Stats) |
| Ctrl+p | Return to Parliament |
| D | HTTP debug log |
| h / ? | Show help |
| q | Quit |

### WISE types

- `WsSourceStats` — Source statistics: source, request, cacheHit, cacheMiss, cacheRefresh, directHit, requestDropped, recentAverageMS, items
- `WsTypeStats` — Type statistics: type, request, found, cacheHit, cacheSrcHit, cacheSrcMiss, cacheSrcRefresh
- `WsStats` — API response: sources (Vec), types (Vec), startTime
- `WsQueryResult` — Query result item: field, value (JSON), len
- `WsStatsTab` — Enum: `Sources` | `Types`. Sub-tabs within WISE Stats tab.

### WISE API endpoints

- `GET /stats` — returns `{sources: [...], types: [...], startTime}`. Optional `?search=` filter.
- `GET /sources` — returns array of source names
- `GET /types` or `/types/:source` — returns array of type names
- `GET /:typeName/:value` — query all sources for a key
- `GET /:source/:typeName/:value` — query specific source for a key
- Query results: `[{field, value, len}, ...]` or plain text "Not found"

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
- Help popup: 64 chars wide (capped at terminal width), with text wrapping enabled
- Viewer contexts: Sessions list, Session detail, Packets view, Stats list, Stats detail, Arkime summary, Column editor, Layouts, Views
- Cont3xt contexts: separate help for Results panel and Detail panel (different key behaviors)
- Uses `macro_rules! hdr` for section headers to avoid closure lifetime issues

## User API

- User info comes from `result.user` in the `/api/appversion` response, stored as `serde_json::Value` in `App.user`
- `removeEnabled` controls whether "Remove Tags" appears in action menus
- `App::remove_enabled()` helper checks `user["removeEnabled"]`

## Okta authentication

- `AuthMode::Okta` — dedicated auth flow for Okta SSO, selected via `--auth okta`
- Like Web auth, Okta defers login to before entering raw mode (needs interactive stdin for prompts)
- Uses `reqwest` cookie store with redirect policy `none` (manual redirect following) and browser-like user agent
- `okta_login()` orchestrates the full flow:
  1. Navigate to app URL, follow redirects to Okta login page
  2. Extract `modelDataBag` JSON from page JavaScript (contains stateToken, baseUrl, org settings, labels)
  3. Fall back to `var stateToken = '...'` regex if modelDataBag not found
  4. Display brand name and app name from Okta page context
  5. Prompt for credentials using Okta's configured labels (e.g., "Email" instead of "Username")
  6. Extract `fromURI` (OAuth2 authorize URL) from hidden form input
  7. Authenticate via IDX API (Identity Engine) or classic authn API
  8. Exchange session token for session cookie via `/login/sessionCookieRedirect`
  9. Verify session established via `/api/appversion`
- `okta_idx_login()` — Identity Engine flow (modern Okta orgs):
  - POST `/idp/idx/introspect` with page's stateToken
  - POST `/idp/idx/identify` with username
  - POST `/idp/idx/challenge/answer` with password
  - Handles `select-authenticator-authenticate` remediation (navigates challenge flow)
  - Handles Okta Verify push: sends challenge, polls for approval (up to 60s)
  - Handles email magic link: shows OTP code, polls for verification
  - Handles TOTP (authenticator app): prompts for code
  - On success, follows `/idp/idx/challenge/answer` → success redirect chain
  - Returns `None` if cookies set directly (no sessionToken), or `Some(token)` for classic redirect
- `okta_classic_authn()` — Classic Okta flow (older orgs):
  - POST `/api/v1/authn` with username, password, and stateToken
  - Handles `SUCCESS` → returns sessionToken
  - Handles `MFA_REQUIRED` → delegates to `okta_handle_mfa()`
- `okta_handle_mfa()` — Classic MFA handling:
  - Prefers Okta Verify push, then TOTP, then fails with available factor list
  - Push: sends verify request, polls every 2s for up to 60s (WAITING/REJECTED/TIMEOUT)
  - TOTP: prompts for code from authenticator app
- IDX flow falls back to classic authn on protocol/setup errors (not auth failures)
- `decode_js_escapes()` — decodes `\xNN`, `\uNNNN`, `\n`, `\r`, `\t`, `\"`, `\\`, `\/` in Okta page JS strings

## Expression input

- Expression and stats filter inputs support full cursor movement (Left/Right/Home/End/Delete)
- `expression_cursor` tracks cursor position within the edit string
- Characters insert at cursor position; Backspace deletes before cursor; Delete deletes at cursor
- Horizontal scrolling: when cursor exceeds input box width, text scrolls so cursor remains visible
- Shift+Left/Right: word-at-a-time jumping (skips non-alphanumeric chars then alphanumeric chars)
- All expression inputs (viewer, cont3xt, stats, parliament issues) support horizontal scrolling

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
| `/api/appversion` | GET | App mode detection + user info | returns `app`, `user` |

### Cont3xt API endpoints

| Endpoint | Method | Purpose | Key params |
|---|---|---|---|
| `/api/integration` | GET | List available integrations | returns `{integrations: {...}}` with card definitions |
| `/api/integration/search` | POST | Search indicators | JSON body: `{query: "..."}`, streaming JSON response |
| `/api/linkGroup` | GET | List shared link groups | returns `{linkGroups: [...]}` |
| `/api/views` | GET | List saved views | returns `{data: [...]}` |
| `/api/view` | POST | Create a view | body: `{name, expression}` |
| `/api/view/:id` | DELETE | Delete a view | |
| `/api/audits` | GET | List audit/history entries | `sortBy`, `sortOrder`, `page` (1-indexed), `itemsPerPage`, `searchTerm`, `startMs`, `stopMs` |
| `/api/audit/:id` | DELETE | Delete a history entry | |
| `/api/overview` | GET | List overview definitions | returns `{overviews: [...]}` |
| `/api/integration/stats` | GET | Integration stats | returns `{stats: [...], itypeStats: [...]}` |

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

## Rendering performance
- **`needs_redraw` flag**: Skip `terminal.draw()` entirely when no state changed. Set true after key handling, background task completion, streaming result drain, auto-refresh.
- **Key event draining**: After processing the first key event, drain all pending key events (poll 0ms) before redrawing. Prevents frame-per-key lag during fast scrolling.
- **Popup double-buffering**: `popup_bg_cache: Option<Buffer>` in App caches the background frame when any popup is open. First frame with popup renders the full background and clones the buffer; subsequent frames restore the cached buffer (fast `clone_from_slice`) and only render the lightweight popup overlay. Cache invalidated on: popup close, terminal resize, streaming results arriving, search completion. This avoids re-rendering expensive tree/card views while scrolling through popups.
- **Link popup windowed rendering**: Instead of building all lines then slicing, counts line positions in a first pass to find scroll offset, then renders only visible lines in a second pass.

## Naming conventions

- All viewer-specific App fields and public methods use `vr_` prefix (e.g., `vr_sessions`, `vr_fetch_sessions()`)
- All cont3xt-specific App fields and public methods use `c3_` prefix (e.g., `c3_results`, `c3_fetch_views()`)
- All parliament-specific App fields and public methods use `pl_` prefix (e.g., `pl_groups`, `pl_fetch_data()`)
- Private methods and non-App struct fields do not use prefixes
- Common/shared fields (user, expression, mode, owl animation) have no prefix

## Crate versions
ratatui 0.29, crossterm 0.28, tokio 1 (full), reqwest 0.12 (rustls-tls), serde/serde_json 1, anyhow 1, urlencoding 2, digest_auth 0.3, rpassword 7, clap 4, chrono, regex 1

## Git rules
- **Never use `git add -A`** — always specify files explicitly when staging (e.g., `git add src/main.rs src/app/mod.rs`)
