# QenloDB Browser

<p align="center"><img src="../assets/brand/logo/lockup.svg" alt="Qenlo" width="320"></p>

A visual and terminal inspector for embedded vector collections, modeled after **DB Browser for SQLite**.

---

## Why this DB Browser exists

Embedded SQL databases like SQLite achieved widespread developer trust and adoption because developers could visually open, inspect, and query `.sqlite` files locally using tools like *DB Browser for SQLite*.

In the vector database landscape, embeddings and nearest-neighbor indices are frequently treated as opaque black boxes behind remote server infrastructures. **QenloDB Browser exists to bring radical transparency and inspectability to embedded vector search**:

1. **Inspect Canonical Rows & Normalization**:
   Directly inspect stored vectors, unit-normalization status, dimensions, user IDs, and signed timestamps directly from on-disk `.qenlo` directories or `.qn` portable snapshots.
2. **Observe Tombstones & WAL Compaction**:
   Watch how atomic deletions create durable tombstones, observe uncompacted WAL segments vs `.qdb` snapshots, and inspect recovery watermarks (`HEAD`) in real time without guessing database internals.
3. **Zero Black Box Vector Queries**:
   Input query vectors, apply compound metadata predicates (`user_id`, timestamp ranges), and test nearest-neighbor queries across backends (**CPU Exact AVX2/FMA**, **USearch HNSW**, and **GPU wgpu**) with full execution timings, distance metrics, and similarity score rankings.
4. **Hardware Acceleration Diagnostics**:
   Verify the host SIMD vector distance kernel (AVX2+FMA, AVX2, NEON, Scalar) and discrete/integrated GPU capabilities on your actual machine.

---

## Three Interfaces

QenloDB Browser is available in three complementary interfaces:

### 1. Claude Code-Style Terminal UI (TUI)

An interactive, keyboard-driven terminal interface built with `ratatui` and `crossterm`.

```powershell
# Open an existing collection in the TUI
cargo run -p qenlo-browser -- ./notes.qenlo

# Or create a new collection with 384 dimensions
cargo run -p qenlo-browser -- ./notes.qenlo --dimension 384 --create
```

#### TUI Keyboard Shortcuts:
| Key | Action |
|---|---|
| `Tab` / `Shift+Tab` | Cycle through navigation tabs |
| `1` – `4`, `?` | Jump directly to Rows, Search, Storage, Diagnostics, or Help |
| `j` / `k` / `↑` / `↓` | Navigate rows in the table |
| `n` / `p` | Next / previous page of records |
| `Enter` | Open full vector inspector modal |
| `a` | Add new record dialog (`Ctrl+R` for random vector) |
| `d` / `Delete` | Durable row deletion |
| `/` | Live filter table by User ID |
| `s` | Run vector cosine similarity search |
| `r` | Generate normalized random query vector |
| `f` | Flush and compact WAL to snapshot |
| `:` | **Claude Code Command Prompt** (`:open`, `:create`, `:search`, `:flush`, `:export`, `:quit`) |
| `q` / `Ctrl+C` | Quit |

---

### 2. Embedded Local Web UI

A zero-bloat, responsive single-page web dashboard served by a local Axum HTTP server directly from the compiled binary.

```powershell
# Launch the Web UI server on port 3456
cargo run -p qenlo-browser -- --web ./notes.qenlo --port 3456
```

Open **`http://127.0.0.1:3456`** in your browser to:
- Browse paginated rows with sparkline vector previews and active/tombstone indicators.
- Run vector queries with live visual similarity meters (`████████░░ 0.9541`) and latency breakdown.
- Inspect on-disk `.qdb`, `.wal`, `.lock`, and `HEAD` files.
- Flush and compact WAL logs or export to portable `.qn` archives.

---

### 3. Tauri v2 Desktop App

A native cross-platform desktop application under `apps/desktop` with OS folder pickers and window chrome.

```powershell
cd apps/desktop
pnpm install
cargo run -p qenlo-browser-desktop
```

---

## REST API Reference

When running in web mode (`--web`), `qenlo-browser` exposes a clean JSON REST API:

- `GET /api/status`: Collection metadata, dimension, live row count, tombstones, and generation.
- `POST /api/open`: Open collection by directory path or portable `.qn` file.
- `POST /api/create`: Create a new collection with a specified dimension.
- `GET /api/records?offset=0&limit=50&user_id=7`: Paginated rows with optional filters.
- `GET /api/records/:id`: Full vector float components for a single record.
- `POST /api/search`: Run cosine similarity search with user/time filters.
- `POST /api/mutate`: Execute atomic add/delete mutation batches.
- `POST /api/flush`: Force sync and WAL compaction to `.qdb` snapshot.
- `POST /api/export`: Export collection to portable `.qn` file.
- `GET /api/storage`: Detailed file listing, sizes, and load admission budget.
- `GET /api/diagnostics`: Host CPU architecture, SIMD distance kernel, and limits.
