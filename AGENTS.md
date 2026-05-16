# KnockShadow Backend — Agent Notes

## Project Structure

```
backend/
├── README.md               # Project overview, quick start, doc index
├── AGENTS.md               # This file — agent-facing notes
├── Cargo.toml              # Workspace root (edition 2024)
├── crates/
│   ├── api-db/             # REST API + WebSocket server (Axum + SQLx + PostgreSQL) — cloud
│   ├── bt-reader/          # Bluetooth LE streamer (btleplug + tokio-postgres) — legacy
│   └── pi-service/         # Servicio offline-first para Raspberry Pi (BLE + SQLite + API local + avahi mDNS)
├── docs/
│   ├── API.md              # API documentation (English + Spanish)
│   ├── Bluetooth Streamer.md
│   ├── Deployment.md       # Guía de despliegue cloud vs Raspberry Pi
│   ├── Pi Inference.md     # Documentación de la CNN offline
│   ├── Pi Service.md       # Documentación del servicio en Raspberry Pi
│   └── Red Neuronal.md     # ML pipeline + entrenamiento
├── ml/                     # Python ML pipeline (PyTorch, structlog, pydantic)
│   ├── pipeline/           # Paquete: signal, detect, dataset I/O (era pipeline.py)
│   ├── model/              # Artefactos: punch_classifier.pt, class_names.npy, norm_*.npy
│   ├── train.py            # Entrenamiento del CNN
│   ├── main.py             # Inferencia modo cloud (lee PostgreSQL) — legacy
│   ├── pi_inference.py     # Inferencia modo Raspberry (lee SQLite local)
│   ├── app.py              # UI Streamlit para etiquetado
│   ├── sync_ble_to_cloud.py# Script one-shot: vuelca SQLite → PostgreSQL
│   ├── model_loader.py     # Carga compartida del modelo (cloud + Pi)
│   ├── logging_config.py   # structlog config
│   └── tests/              # pytest (≥30% coverage gate en CI)
├── db/init/                # SQL de inicialización (TimescaleDB hypertables)
├── grafana/provisioning/   # Dashboards y datasources de Grafana
├── prometheus/             # Configuración de scrape de Prometheus
├── docker-compose.yaml     # Stack cloud (db + api + ml-app + prom + grafana)
├── docker-compose.pi.yaml  # Stack Raspberry Pi offline-first (pi-service + pi-inference + ml-app)
├── Dockerfile.api-db
├── Dockerfile.ble-stream   # Legacy
├── Dockerfile.pi-service
├── Dockerfile.pi-inference
├── .github/workflows/      # CI (Rust + Python + audit + Docker build smoke)
├── .pre-commit-config.yaml # Hooks que reflejan el CI (ruff, cargo fmt/clippy)
└── yaak_collection.json    # Postman-like collection for API testing
```

## API Conventions

- **All API parameters are in English** (`email`, `password`, `first_name`, `last_name`, `phone`, `age`, `weight`, `height`, `country`, `city`, `address`, `laterality`, `level`, `user_id`, `training_id`, `punch_id`, `start_time`, `end_time`, `training_type`, `calories`, `name`, `limb`, `position`, `power`).
- **Database columns remain in Spanish** (`correo`, `contrasena`, `nombre`, `apellido`, etc.) and are mapped via `#[sqlx(rename = "...")]`.
- **Endpoints are in English:**
  - `/users` (was `/usuarios`)
  - `/trainings` (was `/entrenamientos`)
  - `/punches` (was `/golpes`)
  - `/history` (was `/historial`)
- **Passwords are hashed with bcrypt** (salted, default cost) before storage. Plaintext passwords are never stored.

## Auth

- `POST /login` — accepts `{"email": "...", "password": "..."}`
- `POST /register` — accepts `CreateUser` JSON, returns JWT
- All other endpoints require `Authorization: Bearer <jwt>`
- JWT secret comes from `JWT_SECRET` env var (defaults to insecure default — change in production)

## Build & Run (local)

```bash
cargo check --workspace    # Verify compilation
cargo run -p api-db        # Run remote API server
cargo run -p pi-service    # Run Raspberry Pi offline-first service
```

## Docker

### Cloud stack (`docker-compose.yaml`)

Levanta PostgreSQL + API REST + Streamlit app (para etiquetar datos de entrenamiento).

> **Nota:** Los sensores BLE siempre se emparejan con la Raspberry Pi vía Bluetooth Low Energy. El cloud solo aloja la API central y la base de datos donde se sincronizan los datos.

```bash
docker compose -f docker-compose.yaml up --build
```

Servicios expuestos:
- API: `http://localhost:3000`
- Streamlit: `http://localhost:8501`
- PostgreSQL: `localhost:5432`

### Raspberry Pi stack (`docker-compose.pi.yaml`)

Levanta `pi-service` + `pi-inference` con SQLite local. **No requiere PostgreSQL ni conexión a internet**.

```bash
# En la Raspberry Pi 4 (ARM64)
export DEVICE_MAC_1=DF:65:81:D0:D7:E5
export DEVICE_MAC_2=CB:01:10:3E:0D:61
export API_BASE_URL=http://tu-servidor-cloud:3000   # opcional, para sync
docker compose -f docker-compose.pi.yaml up --build
```

Servicios:
- `pi-service`: API local en `http://<ip-de-la-pi>:8080`, mDNS `_knockshadow._tcp.local.`
- `pi-inference`: lee SQLite, clasifica golpes y escribe resultados.
- Volumen compartido `pi-data` para SQLite persistente (`/data/pi_data.db`).

**Notas de despliegue en Raspberry Pi:**
- `pi-service` usa `privileged: true` y `network_mode: host` para BLE y mDNS.
- Asegúrate de que `bluetoothd` está activo en el host antes de levantar el contenedor.
- Para compilar imágenes ARM64 desde x86_64, usa Docker Buildx:
  ```bash
  docker buildx build --platform linux/arm64 -f Dockerfile.pi-service -t knockshadow/pi-service:latest .
  ```

## Architecture — Raspberry Pi (Offline-First)

```
┌─────────────┐   BLE   ┌──────────────┐   SQLite   ┌─────────────────┐
│  Sensores   │ ───────►│  pi-service  │◄──────────►│  pi_inference   │
│   (2x)      │         │   (Rust)     │            │   (Python/CNN)  │
└─────────────┘         └──────────────┘            └─────────────────┘
                               │                           │
                               │ HTTP/WS                   │ write punches
                               ▼                           ▼
                        ┌─────────────┐            ┌─────────────┐
                        │  App Movil  │            │  SQLite DB  │
                        │ (mDNS desc) │            │  (offline)  │
                        └─────────────┘            └─────────────┘
                               │                           │
                               │ sync (si hay internet)    │
                               ▼                           ▼
                        ┌───────────────────────────────────────┐
                        │           api-db remota               │
                        │      (PostgreSQL + API REST)          │
                        └───────────────────────────────────────┘
```

### pi-service (Rust)

Servicio que corre en la Raspberry Pi 4. Funciona **sin conexión a internet**.

- **BLE**: conecta 1 o 2 sensores y guarda muestras en SQLite.
- **API local**: Axum en el puerto configurado (default `8080`).
  - `GET /` — dashboard web embebido (HTML estático servido vía `include_str!`).
  - `GET /health` — healthcheck JSON (`{"status":"ok","service":"pi-service"}`).
  - `GET /training/active` — devuelve el entrenamiento activo (o `active: false`).
  - `POST /training/start` — crea entrenamiento local. Body: `{ "user_id": i32, "jwt": "optional", "training_type": "Standard" }`.
  - `POST /training/stop` — finaliza entrenamiento local. Body: `{ "local_training_id": i64 }`.
  - `GET /trainings/:id/punches` — lista golpes detectados de un entrenamiento.
  - `WS /live` — stream en tiempo real de golpes detectados (JSON).
- **mDNS**: anuncia `_knockshadow._tcp.local.` para que la app móvil descubra la Raspberry automáticamente en la red WiFi.
- **Sync remoto**: si `API_BASE_URL` está configurado, sincroniza la cola `sync_queue` (entrenamientos y futuros datos) con la API remota cada 30 segundos.

### pi_inference.py (Python)

Script de inferencia CNN adaptado para correr **offline** en la Raspberry.

- Lee muestras BLE desde la **SQLite local** (en lugar de PostgreSQL).
- Detecta golpes, clasifica y calcula **potencia (G)**.
- Guarda resultados en la tabla `detected_punches` con `user_id` y `local_training_id`.
- Si no se pasa `--training-id`, **autodescubre** el entrenamiento activo consultando `local_trainings`.
- Usa el mismo modelo PyTorch entrenado (`model/punch_classifier.pt`).

#### Ejecución típica en la Raspberry

Terminal 1:
```bash
export DB_PATH=pi_data.db
export DEVICE_MAC_1=DF:65:81:D0:D7:E5
export DEVICE_MAC_2=CB:01:10:3E:0D:61
cargo run -p pi-service
```

Terminal 2:
```bash
export DB_PATH=pi_data.db
python ml/pi_inference.py --user-id 1
```

La app móvil:
1. Descubre `knockshadow-pi.local.` vía mDNS.
2. Se conecta a `http://<ip>:8080`.
3. Llama `POST /training/start` para iniciar sesión.
4. Se conecta a `WS /live` para ver golpes en tiempo real.
5. Llama `POST /training/stop` al finalizar.

## Entrenamiento de la red neuronal

### Opción A: Etiquetado en tiempo real en la Raspberry Pi (offline)

Desde la versión actual, la app Streamlit (`ml-app`) también puede correr directamente en la Raspberry Pi leyendo la **SQLite local** (`pi_data.db`). Esto permite etiquetar golpes en tiempo real **sin necesidad de PostgreSQL ni internet**.

1. **Levanta el stack completo de la Pi** (incluye `ml-app`):
   ```bash
   docker compose -f docker-compose.pi.yaml up --build -d
   ```
2. **Abre la app de etiquetado** desde cualquier dispositivo en la misma red:
   ```
   http://<ip-de-la-pi>:8501
   ```
3. **Graba y etiqueta** — pulsa *Iniciar grabación*, pega los golpes, detén, y asigna tipo/posición a cada pico detectado.
4. **Entrena en la Pi** (o copia `ml/data/dataset.npz` a tu PC para entrenar más rápido):
   ```bash
   docker exec knockshadow-ml-app python train.py
   ```
   El modelo se guardará en `ml/model/punch_classifier.pt` dentro del volumen compartido.
5. **Reinicia `pi-inference`** para que use el nuevo modelo:
   ```bash
   docker compose -f docker-compose.pi.yaml restart pi-inference
   ```

### Opción B: Flujo cloud (PC más potente)

Si prefieres entrenar en tu PC por rendimiento:

1. **Capturar datos en la Raspberry** — `pi-service` guarda muestras BLE en `pi_data.db`.
2. **Exportar al cloud** — usa el script `ml/sync_ble_to_cloud.py` para volcar `ble_samples` desde SQLite a PostgreSQL:
   ```bash
   # Desde tu PC (con la base SQLite copiada de la Pi)
   python ml/sync_ble_to_cloud.py \
       --sqlite pi_data.db \
       --pg-url postgres://knockshadow:knockshadow@<host-cloud>:5432/knockshadow
   ```
3. **Etiquetar** — levanta el stack cloud (`docker-compose.yaml`) y abre `http://localhost:8501` para usar la app Streamlit.
4. **Entrenar** — ejecuta `python ml/train.py`. Genera `model/punch_classifier.pt`.
5. **Desplegar en la Pi** — copia la carpeta `ml/model/` a la Raspberry y reconstruye `docker-compose.pi.yaml`.

## Environment Variables

### api-db (remoto)

| Variable | Default | Description |
|----------|---------|-------------|
| `DATABASE_URL` | `postgres://knockshadow:knockshadow@127.0.0.1:5432/knockshadow` | PostgreSQL connection string |
| `PORT` | `3000` | HTTP server port |
| `JWT_SECRET` | `knockshadow_default_secret_change_me` | JWT signing secret |

### pi-service (Raspberry Pi)

| Variable | Default | Description |
|----------|---------|-------------|
| `DB_PATH` | `pi_data.db` | Ruta a la SQLite local |
| `PORT` | `8080` | Puerto del API local |
| `DEVICE_MAC_1` | — | MAC del sensor izquierdo |
| `DEVICE_MAC_2` | — | MAC del sensor derecho |
| `API_BASE_URL` | — | URL base de la API remota (opcional, ej: `http://api.knockshadow.com:3000`) |
| `MDNS_HOSTNAME` | `knockshadow-pi` | Nombre mDNS del servicio |

### pi_inference.py (Raspberry Pi)

| Variable | Default | Description |
|----------|---------|-------------|
| `DB_PATH` | `pi_data.db` | Misma SQLite que pi-service |
| `SENSOR_MAC_1` | `DF:65:81:D0:D7:E5` | MAC sensor izquierdo |
| `SENSOR_MAC_2` | `CB:01:10:3E:0D:61` | MAC sensor derecho |

## Dependencies of Note

- `bcrypt` — password hashing
- `jsonwebtoken` — JWT creation/validation
- `axum` — HTTP/WebSocket framework
- `axum-prometheus` — `/metrics` endpoint and HTTP latency histograms
- `sqlx` — async SQL with compile-time checks
- `btleplug` — BLE abstraction (Linux uses BlueZ via DBus)
- `reqwest` — HTTP client for remote sync

> **mDNS on the Pi is not a cargo dependency.** `pi-service` shells out to
> `avahi-publish-service` (provided by `avahi-utils` in the runtime image)
> and lets the host's `avahi-daemon` own UDP/5353. An in-process responder
> would collide with the host daemon because the container runs with
> `network_mode: host`. See `crates/pi-service/src/mdns.rs`.

## CI / pre-commit

GitHub Actions (`.github/workflows/ci.yml`) runs four jobs on push and PR:

1. **Rust** — `cargo fmt --check`, `cargo clippy --workspace -D warnings`, `cargo test`.
2. **Python (`ml/`)** — `ruff check`, `ruff format --check`, `mypy`, `pytest --cov` (≥30% gate).
3. **Audit** (non-blocking) — `cargo-audit` + `pip-audit` on each `requirements_*.txt`.
4. **Docker** — buildx smoke build for `api-db`, `pi-service`, `pi-inference`.

Mirror locally with `pre-commit install && pre-commit run --all-files`.

## Observability

The cloud stack ships Prometheus + Grafana out of the box (see
`docker-compose.yaml`). `api-db` exposes `/metrics` through
`PrometheusMetricLayer`, intentionally **outside** the auth middleware so the
scraper does not need a JWT. Grafana provisioning lives in
`grafana/provisioning/`; default credentials are `admin/admin` (override with
`GRAFANA_ADMIN_USER` / `GRAFANA_ADMIN_PASSWORD`).
