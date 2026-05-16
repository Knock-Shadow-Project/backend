# KnockShadow — Backend

Backend monorepo for **KnockShadow**, a smart-bag system that captures
boxing-punch telemetry over Bluetooth Low Energy, classifies each punch with a
1D CNN, and stores the results for review and analytics.

The repo ships **two parallel stacks** that share most of the code:

| Stack | Runs on | Database | Internet required |
|-------|---------|----------|-------------------|
| **Cloud** (`docker-compose.yaml`) | VPS / dev laptop | PostgreSQL / TimescaleDB | Yes |
| **Raspberry Pi** (`docker-compose.pi.yaml`) | Raspberry Pi 4 (ARM64) | SQLite (offline-first) | No (optional sync) |

The Pi stack is the production target; the cloud stack is used for labelling,
training, central storage, and ML experimentation. The two converge when the
Pi has internet access and syncs queued records to the cloud API.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                              CLOUD / VPS                                │
│   ┌──────────────┐  ┌──────────────┐  ┌────────────┐  ┌──────────────┐  │
│   │  PostgreSQL  │  │   api-db     │  │ Prometheus │  │   Grafana    │  │
│   │  Timescale   │◄─┤  REST + WS   │─►│   /metrics │─►│  dashboards  │  │
│   └──────────────┘  └──────┬───────┘  └────────────┘  └──────────────┘  │
│                            │                                            │
│                            │   sync (when Pi has internet)              │
└────────────────────────────┼────────────────────────────────────────────┘
                             ▲
┌────────────────────────────┼────────────────────────────────────────────┐
│                        LOCAL Wi-Fi                                      │
│                            │                                            │
│   ┌──────────────────────────────────────────────────────────────────┐  │
│   │                       RASPBERRY PI 4                             │  │
│   │  ┌─────────────┐  BLE  ┌──────────────┐  SQLite  ┌────────────┐  │  │
│   │  │  Sensors    │──────►│  pi-service  │◄────────►│pi-inference│  │  │
│   │  │  (1–2x)     │       │   (Rust)     │          │ (Py / CNN) │  │  │
│   │  └─────────────┘       └──────┬───────┘          └────────────┘  │  │
│   │                               │ HTTP / WS                        │  │
│   │                               ▼                                  │  │
│   │                        ┌──────────────┐                          │  │
│   │                        │  Mobile App  │   mDNS:                  │  │
│   │                        │ (mDNS disc.) │   _knockshadow._tcp      │  │
│   │                        └──────────────┘                          │  │
│   └──────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────┘
```

See [docs/Deployment.md](docs/Deployment.md) for the full diagram and the
deployment runbook.

---

## Repository layout

```
backend/
├── Cargo.toml                 # Rust workspace root
├── crates/
│   ├── api-db/                # Cloud REST + WebSocket API (Axum + sqlx + PostgreSQL)
│   ├── bt-reader/             # Legacy BLE → PostgreSQL streamer (pre-Pi)
│   └── pi-service/            # Offline-first Pi service (BLE + SQLite + local API + mDNS)
├── ml/                        # Python pipeline (PyTorch)
│   ├── pipeline/              # Signal processing + detection + dataset I/O
│   ├── model/                 # Trained artefacts (.pt, class_names.npy, norm_*.npy)
│   ├── train.py               # CNN training entry point
│   ├── main.py                # Cloud inference (PostgreSQL) — legacy
│   ├── pi_inference.py        # Pi inference (SQLite, offline)
│   ├── app.py                 # Streamlit labelling UI
│   └── sync_ble_to_cloud.py   # One-shot script: copy ble_samples Pi → cloud
├── db/init/                   # PostgreSQL schema (TimescaleDB hypertables)
├── docker-compose.yaml        # Cloud stack (db + api-db + ml-app + Prom + Grafana)
├── docker-compose.pi.yaml     # Pi stack (pi-service + pi-inference + ml-app)
├── Dockerfile.api-db          # Cloud API image
├── Dockerfile.pi-service      # Pi service image (ARM64)
├── Dockerfile.pi-inference    # Pi inference image (CPU-only torch)
├── Dockerfile.ble-stream      # Legacy BLE → PostgreSQL image
├── grafana/                   # Grafana provisioning (dashboards, datasources)
├── prometheus/                # Prometheus scrape config
├── docs/                      # Long-form documentation (see below)
├── AGENTS.md                  # Agent-facing notes (conventions, build, run)
└── yaak_collection.json       # API client collection (Postman-style)
```

---

## Quick start

### Cloud stack (PostgreSQL + API + labelling UI)

```bash
git clone https://github.com/Knock-Shadow-Project/backend.git
cd backend

# Optional: override the JWT secret before first run
export JWT_SECRET=your-strong-secret

docker compose -f docker-compose.yaml up --build -d
```

Once the containers are healthy:

| Service | URL |
|---------|-----|
| REST + WebSocket API | http://localhost:3000 |
| Streamlit labelling UI | http://localhost:8501 |
| Prometheus | http://localhost:9090 |
| Grafana | http://localhost:3001 (admin / admin) |
| PostgreSQL | `localhost:5432` (knockshadow / knockshadow) |

Smoke test:

```bash
curl http://localhost:3000/                 # API root
curl http://localhost:3000/metrics | head   # Prometheus metrics
```

### Raspberry Pi stack (offline-first)

On the Pi (Raspberry Pi OS 64-bit, with `bluetoothd` active):

```bash
export DEVICE_MAC_1=DF:65:81:D0:D7:E5
export DEVICE_MAC_2=CB:01:10:3E:0D:61
# Optional — enables periodic sync to the cloud API
export API_BASE_URL=https://api.knockshadow.site

docker compose -f docker-compose.pi.yaml up --build -d
```

The mobile app auto-discovers the Pi via mDNS (`knockshadow-pi.local`,
service type `_knockshadow._tcp`) and talks to `http://<pi-ip>:8080`.

Full runbook, cross-compilation, and troubleshooting in
[docs/Deployment.md](docs/Deployment.md).

### Local Rust workflow

```bash
cargo check --workspace             # Compile every crate
cargo run -p api-db                 # Cloud API (needs PostgreSQL running)
cargo run -p pi-service             # Pi service (needs DEVICE_MAC_* env)
cargo test  --workspace             # Tests
```

### Local Python workflow

```bash
cd ml
uv sync                             # Resolve from uv.lock
uv run python train.py              # Train the CNN
uv run python pi_inference.py       # Inference against pi_data.db
uv run streamlit run app.py         # Labelling UI
```

---

## Tech stack

**Rust (workspace, edition 2024):**
`axum` (HTTP + WebSocket), `sqlx` (async SQL with compile-time checks),
`btleplug` (BLE), `tokio`, `reqwest`, `jsonwebtoken`, `bcrypt`,
`axum-prometheus`. mDNS publishing on the Pi is delegated to the host's
`avahi-daemon` via DBus (`avahi-publish-service`) — there is **no** in-process
mDNS responder, which avoids the UDP/5353 bind conflict when running with
`network_mode: host`.

**Python (==3.14.*):**
`torch` (CPU on Pi, GPU optional on cloud), `numpy`, `pandas`, `scipy`,
`scikit-learn`, `streamlit`, `plotly`, `structlog`, `pydantic`, `psycopg2`,
`websockets`.

**Storage:**
PostgreSQL + TimescaleDB hypertables in the cloud; SQLite (WAL,
`busy_timeout=5000`, `synchronous=NORMAL`) on the Pi, shared between
`pi-service`, `pi-inference`, and `ml-app` through a Docker volume.

**Observability:** Prometheus scrapes `api-db /metrics`; Grafana provisions
dashboards out of `grafana/provisioning/`.

---

## Documentation

| Doc | What's in it |
|-----|--------------|
| [docs/API.md](docs/API.md) | Cloud REST + WebSocket reference (`/login`, `/users`, `/trainings`, `/punches`, `/history`, `/ws`, `/metrics`). |
| [docs/Deployment.md](docs/Deployment.md) | Cloud vs Pi deployment, cross-compilation, troubleshooting, checklists. |
| [docs/Pi Service.md](docs/Pi%20Service.md) | `pi-service` internals: BLE, SQLite schema, local API, mDNS (avahi), remote sync. |
| [docs/Pi Inference.md](docs/Pi%20Inference.md) | `pi_inference.py` internals: model loading, async producer/consumer, CLI. |
| [docs/Red Neuronal.md](docs/Red%20Neuronal.md) | ML pipeline package, `PunchCNN` architecture, training, labelling UI. |
| [docs/Bluetooth Streamer.md](docs/Bluetooth%20Streamer.md) | Legacy `ble-stream` notes (kept for reference; not part of the current production path). |
| [AGENTS.md](AGENTS.md) | Conventions and quick reference for agents working on this repo. |

---

## Development

### CI

GitHub Actions runs four jobs on every push and PR ([`.github/workflows/ci.yml`](.github/workflows/ci.yml)):

1. **Rust** — `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`.
2. **Python (`ml/`)** — `ruff check`, `ruff format --check`, `mypy`, `pytest` with coverage gate (≥30%).
3. **Dependency audit** — `cargo-audit` + `pip-audit` (non-blocking until the advisory backlog hits 0).
4. **Docker** — builds `api-db`, `pi-service`, and `pi-inference` images as a smoke test.

### Pre-commit

```bash
pre-commit install
pre-commit run --all-files
```

The hooks mirror CI: ruff (Python), `cargo fmt` and `cargo clippy` (Rust),
plus secret-scanning and basic hygiene (trailing whitespace, large files,
private keys). What fails locally also fails in CI.

### Conventions

- API payload fields are **English** (`email`, `first_name`, `user_id`, …).
- Database columns remain **Spanish** (`correo`, `nombre`, …) and are mapped
  via `#[sqlx(rename = "...")]`.
- Passwords are bcrypt-hashed before storage; never store plaintext.
- JWTs are required on every endpoint except `POST /login` and `POST /register`.
- The Pi must keep working without internet — the cloud sync is a best-effort
  background loop, not a hard dependency.

See [AGENTS.md](AGENTS.md) for the full set of conventions and the recommended
workflow when modifying the codebase.

---

## Authors

- Victor Galan
- Cristian Davila
