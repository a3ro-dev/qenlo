# QenloDB browser

<p align="center"><img src="../assets/brand/logo/lockup.svg" alt="Qenlo" width="320"></p>

A visual and terminal inspector for embedded vector collections, modeled after DB Browser for SQLite.

---

## Why this database browser exists

Embedded SQL databases like SQLite earned widespread developer trust because anyone could open, inspect, and query `.sqlite` files locally using tools like DB Browser for SQLite.

Most vector databases treat embeddings and nearest-neighbor indices as opaque black boxes behind remote servers. QenloDB Browser provides that same local visibility for embedded vector search:

- Inspect stored vectors, unit-normalization status, dimensions, user IDs, and signed timestamps directly from on-disk `.qenlo` directories or `.qn` snapshots.
- Observe how atomic deletions produce durable tombstones, compare uncompacted WAL segments against `.qdb` snapshots, and check recovery watermarks (`HEAD`) directly on disk.
- Test vector queries with compound metadata filters, such as `user_id` and timestamp ranges, across backends including CPU Exact (AVX2/FMA), USearch HNSW, and GPU (wgpu). The browser reports execution timings, distance metrics, and similarity rankings for each query.
- Verify which SIMD vector distance kernel (AVX2+FMA, AVX2, NEON, or Scalar) and GPU adapters your machine runs.

---

## Three interfaces

QenloDB Browser has three interfaces:

### 1. Terminal user interface (TUI)

An interactive, keyboard-driven terminal interface built with `ratatui` and `crossterm`.

```powershell
# Open an existing collection in the TUI
cargo run -p qenlo-browser -- ./notes.qenlo

# Or create a new collection with 384 dimensions
cargo run -p qenlo-browser -- ./notes.qenlo --dimension 384 --create
```

#### TUI keyboard shortcuts
| Key | Action |
|---|---|
| `Tab` / `Shift+Tab` | Cycle through navigation tabs |
| `1` to `4`, `?` | Jump directly to Rows, Search, Storage, Diagnostics, or Help |
| `j` / `k` / `↑` / `↓` | Navigate rows in the table |
| `n` / `p` | Next / previous page of records |
| `Enter` | Open full vector inspector modal |
| `a` | Add new record dialog (`Ctrl+R` for random vector) |
| `d` / `Delete` | Durable row deletion |
| `/` | Live filter table by user ID |
| `s` | Run vector cosine similarity search |
| `r` | Generate normalized random query vector |
| `f` | Flush and compact WAL to snapshot |
| `:` | Command prompt (`:open`, `:create`, `:search`, `:flush`, `:export`, `:quit`) |
| `q` / `Ctrl+C` | Quit |

---

### 2. Embedded local web UI

A responsive single-page web dashboard served by an internal Axum HTTP server.

```powershell
# Launch the Web UI server on port 3456
cargo run -p qenlo-browser -- --web ./notes.qenlo --port 3456
```

Open `http://127.0.0.1:3456` in your browser to:
- Browse paginated rows with sparkline vector previews and active or tombstone indicators.
- Run vector queries with visual similarity meters (`████████░░ 0.9541`) and latency breakdowns.
- Inspect on-disk `.qdb`, `.wal`, `.lock`, and `HEAD` files.
- Flush and compact WAL logs or export collections to portable `.qn` archives.

---

### 3. Tauri v2 desktop app

A native cross-platform desktop application under `apps/desktop` with OS folder pickers and window chrome.

```powershell
cd apps/desktop
pnpm install
cargo run -p qenlo-browser-desktop
```

---

## REST API reference

When running in web mode (`--web`), `qenlo-browser` exposes a JSON REST API:

- `GET /api/status` returns collection metadata, dimension, live row count, tombstones, and generation.
- `POST /api/open` opens a collection from a directory path or a portable `.qn` file.
- `POST /api/create` creates a new collection with a specified dimension.
- `GET /api/records?offset=0&limit=50&user_id=7` returns paginated rows with optional filters.
- `GET /api/records/:id` returns full vector float components for a single record.
- `POST /api/search` runs cosine similarity search with user and time filters.
- `POST /api/mutate` executes atomic add and delete mutation batches.
- `POST /api/flush` forces a sync and WAL compaction to a `.qdb` snapshot.
- `POST /api/export` exports the collection to a portable `.qn` file.
- `GET /api/storage` returns file listings, sizes, and load admission budgets.
- `GET /api/diagnostics` reports host CPU architecture, SIMD distance kernels, and system limits.
