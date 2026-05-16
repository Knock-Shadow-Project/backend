# KnockShadow — Backend

Monorepo del backend de **KnockShadow**, un saco inteligente que captura
telemetría de golpes de boxeo por Bluetooth Low Energy, clasifica cada golpe
con una CNN 1D y almacena los resultados para revisión y analítica.

El repo incluye **dos stacks en paralelo** que comparten la mayor parte del
código:

| Stack | Corre en | Base de datos | Internet requerido |
|-------|----------|---------------|--------------------|
| **Cloud** (`docker-compose.yaml`) | VPS / portátil de desarrollo | PostgreSQL / TimescaleDB | Sí |
| **Raspberry Pi** (`docker-compose.pi.yaml`) | Raspberry Pi 4 (ARM64) | SQLite (offline-first) | No (sync opcional) |

El stack de la Pi es el objetivo de producción; el stack cloud se usa para
etiquetado, entrenamiento, almacenamiento central y experimentación de ML.
Los dos convergen cuando la Pi tiene internet y sincroniza los registros
encolados contra la API cloud.

---

## Arquitectura

```
┌─────────────────────────────────────────────────────────────────────────┐
│                              CLOUD / VPS                                │
│   ┌──────────────┐  ┌──────────────┐  ┌────────────┐  ┌──────────────┐  │
│   │  PostgreSQL  │  │   api-db     │  │ Prometheus │  │   Grafana    │  │
│   │  Timescale   │◄─┤  REST + WS   │─►│   /metrics │─►│  dashboards  │  │
│   └──────────────┘  └──────┬───────┘  └────────────┘  └──────────────┘  │
│                            │                                            │
│                            │   sync (cuando la Pi tiene internet)       │
└────────────────────────────┼────────────────────────────────────────────┘
                             ▲
┌────────────────────────────┼────────────────────────────────────────────┐
│                        Wi-Fi LOCAL                                      │
│                            │                                            │
│   ┌──────────────────────────────────────────────────────────────────┐  │
│   │                       RASPBERRY PI 4                             │  │
│   │  ┌─────────────┐  BLE  ┌──────────────┐  SQLite  ┌────────────┐  │  │
│   │  │  Sensores   │──────►│  pi-service  │◄────────►│pi-inference│  │  │
│   │  │  (1–2x)     │       │   (Rust)     │          │ (Py / CNN) │  │  │
│   │  └─────────────┘       └──────┬───────┘          └────────────┘  │  │
│   │                               │ HTTP / WS                        │  │
│   │                               ▼                                  │  │
│   │                        ┌──────────────┐                          │  │
│   │                        │ App Móvil    │   mDNS:                  │  │
│   │                        │ (descub.     │   _knockshadow._tcp      │  │
│   │                        │  mDNS)       │                          │  │
│   │                        └──────────────┘                          │  │
│   └──────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────┘
```

Ver [docs/Deployment.md](docs/Deployment.md) para el diagrama completo y el
runbook de despliegue.

---

## Estructura del repositorio

```
backend/
├── Cargo.toml                 # Raíz del workspace Rust
├── crates/
│   ├── api-db/                # API REST + WebSocket cloud (Axum + sqlx + PostgreSQL)
│   ├── bt-reader/             # Streamer BLE → PostgreSQL legacy (pre-Pi)
│   └── pi-service/            # Servicio offline-first para la Pi (BLE + SQLite + API local + mDNS)
├── ml/                        # Pipeline Python (PyTorch)
│   ├── pipeline/              # Procesamiento de señal + detección + I/O del dataset
│   ├── model/                 # Artefactos entrenados (.pt, class_names.npy, norm_*.npy)
│   ├── train.py               # Punto de entrada del entrenamiento de la CNN
│   ├── main.py                # Inferencia cloud (PostgreSQL) — legacy
│   ├── pi_inference.py        # Inferencia en la Pi (SQLite, offline)
│   ├── app.py                 # UI Streamlit para etiquetado
│   └── sync_ble_to_cloud.py   # Script one-shot: vuelca ble_samples Pi → cloud
├── db/init/                   # Esquema PostgreSQL (hypertables de TimescaleDB)
├── docker-compose.yaml        # Stack cloud (db + api-db + ml-app + Prom + Grafana)
├── docker-compose.pi.yaml     # Stack Pi (pi-service + pi-inference + ml-app)
├── Dockerfile.api-db          # Imagen de la API cloud
├── Dockerfile.pi-service      # Imagen del servicio Pi (ARM64)
├── Dockerfile.pi-inference    # Imagen de inferencia Pi (torch CPU-only)
├── Dockerfile.ble-stream      # Imagen legacy BLE → PostgreSQL
├── grafana/                   # Provisioning de Grafana (dashboards, datasources)
├── prometheus/                # Config de scrape de Prometheus
├── docs/                      # Documentación extensa (ver más abajo)
├── AGENTS.md                  # Notas para agentes (convenciones, build, run)
└── yaak_collection.json       # Colección de cliente API (estilo Postman)
```

---

## Quick start

### Stack cloud (PostgreSQL + API + UI de etiquetado)

```bash
git clone https://github.com/Knock-Shadow-Project/backend.git
cd backend

# Opcional: sobreescribe el JWT secret antes del primer arranque
export JWT_SECRET=tu-secret-fuerte

docker compose -f docker-compose.yaml up --build -d
```

Cuando los contenedores estén healthy:

| Servicio | URL |
|----------|-----|
| API REST + WebSocket | http://localhost:3000 |
| UI de etiquetado Streamlit | http://localhost:8501 |
| Prometheus | http://localhost:9090 |
| Grafana | http://localhost:3001 (admin / admin) |
| PostgreSQL | `localhost:5432` (knockshadow / knockshadow) |

Smoke test:

```bash
curl http://localhost:3000/                 # Raíz de la API
curl http://localhost:3000/metrics | head   # Métricas Prometheus
```

### Stack Raspberry Pi (offline-first)

En la Pi (Raspberry Pi OS 64-bit, con `bluetoothd` activo):

```bash
export DEVICE_MAC_1=DF:65:81:D0:D7:E5
export DEVICE_MAC_2=CB:01:10:3E:0D:61
# Opcional — activa la sincronización periódica con la API cloud
export API_BASE_URL=https://api.knockshadow.site

docker compose -f docker-compose.pi.yaml up --build -d
```

La app móvil descubre la Pi automáticamente por mDNS
(`knockshadow-pi.local`, tipo de servicio `_knockshadow._tcp`) y se conecta
a `http://<ip-pi>:8080`.

Runbook completo, compilación cruzada y troubleshooting en
[docs/Deployment.md](docs/Deployment.md).

### Flujo local de Rust

```bash
cargo check --workspace             # Compila cada crate
cargo run -p api-db                 # API cloud (necesita PostgreSQL corriendo)
cargo run -p pi-service             # Servicio Pi (necesita DEVICE_MAC_* en env)
cargo test  --workspace             # Tests
```

### Flujo local de Python

```bash
cd ml
uv sync                             # Resuelve desde uv.lock
uv run python train.py              # Entrena la CNN
uv run python pi_inference.py       # Inferencia contra pi_data.db
uv run streamlit run app.py         # UI de etiquetado
```

---

## Stack técnico

**Rust (workspace, edition 2024):**
`axum` (HTTP + WebSocket), `sqlx` (SQL async con compile-time checks),
`btleplug` (BLE), `tokio`, `reqwest`, `jsonwebtoken`, `bcrypt`,
`axum-prometheus`. El anuncio mDNS en la Pi se delega al `avahi-daemon` del
host vía DBus (`avahi-publish-service`) — **no hay** responder mDNS en el
proceso, lo que evita la colisión en UDP/5353 cuando se corre con
`network_mode: host`.

**Python (==3.14.*):**
`torch` (CPU en la Pi, GPU opcional en cloud), `numpy`, `pandas`, `scipy`,
`scikit-learn`, `streamlit`, `plotly`, `structlog`, `pydantic`, `psycopg2`,
`websockets`.

**Almacenamiento:**
PostgreSQL + hypertables de TimescaleDB en el cloud; SQLite (WAL,
`busy_timeout=5000`, `synchronous=NORMAL`) en la Pi, compartido entre
`pi-service`, `pi-inference` y `ml-app` mediante un volumen Docker.

**Observabilidad:** Prometheus scrapea `api-db /metrics`; Grafana aprovisiona
dashboards desde `grafana/provisioning/`.

---

## Documentación

| Doc | Contenido |
|-----|-----------|
| [docs/API.md](docs/API.md) | Referencia REST + WebSocket de la API cloud (`/login`, `/users`, `/trainings`, `/punches`, `/history`, `/ws`, `/metrics`). |
| [docs/Deployment.md](docs/Deployment.md) | Despliegue cloud vs Pi, compilación cruzada, troubleshooting, checklists. |
| [docs/Pi Service.md](docs/Pi%20Service.md) | Internos de `pi-service`: BLE, esquema SQLite, API local, mDNS (avahi), sync remoto. |
| [docs/Pi Inference.md](docs/Pi%20Inference.md) | Internos de `pi_inference.py`: carga de modelo, productor/consumidor asíncronos, CLI. |
| [docs/Red Neuronal.md](docs/Red%20Neuronal.md) | Paquete del pipeline ML, arquitectura `PunchCNN`, entrenamiento, UI de etiquetado. |
| [docs/Bluetooth Streamer.md](docs/Bluetooth%20Streamer.md) | Notas del `ble-stream` legacy (se mantiene como referencia; no forma parte del path de producción actual). |
| [AGENTS.md](AGENTS.md) | Convenciones y referencia rápida para agentes trabajando en este repo. |

---

## Desarrollo

### CI

GitHub Actions corre cuatro jobs en cada push y PR
([`.github/workflows/ci.yml`](.github/workflows/ci.yml)):

1. **Rust** — `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`.
2. **Python (`ml/`)** — `ruff check`, `ruff format --check`, `mypy`, `pytest` con gate de cobertura (≥30%).
3. **Auditoría de dependencias** — `cargo-audit` + `pip-audit` (no bloqueante hasta que el backlog de advisories esté en 0).
4. **Docker** — construye las imágenes `api-db`, `pi-service` y `pi-inference` como smoke test.

### Pre-commit

```bash
pre-commit install
pre-commit run --all-files
```

Los hooks reflejan el CI: ruff (Python), `cargo fmt` y `cargo clippy`
(Rust), más detección de secretos e higiene básica (trailing whitespace,
archivos grandes, claves privadas). Lo que falla en local también falla en
CI.

### Convenciones

- Los campos del payload de la API son **en inglés** (`email`, `first_name`,
  `user_id`, …).
- Las columnas de base de datos siguen **en español** (`correo`, `nombre`,
  …) y se mapean con `#[sqlx(rename = "...")]`.
- Las contraseñas se hashean con bcrypt antes de almacenarse; nunca se
  guarda plaintext.
- Todos los endpoints requieren JWT excepto `POST /login` y `POST /register`.
- La Pi debe seguir funcionando sin internet — la sincronización con cloud
  es un loop en background "best-effort", nunca una dependencia dura.

Ver [AGENTS.md](AGENTS.md) para el set completo de convenciones y el flujo
recomendado al modificar el código.

---

## Autores

- Victor Galan
- Cristian Davila
