# KnockShadow Backend — Agent Notes

## Project Structure

```
backend/
├── Cargo.toml              # Workspace root
├── crates/
│   ├── api-db/             # REST API + WebSocket server (Axum + SQLx + PostgreSQL)
│   └── bt-reader/          # Bluetooth LE streamer (btleplug + tokio-postgres)
├── docs/
│   ├── API.md              # API documentation (English)
│   ├── Bluetooth Streamer.md
│   └── Red Neuronal.md
├── ml/                     # Python ML pipeline (PyTorch)
├── grafana/                # Dashboard provisioning
├── docker-compose.yaml
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

## Build & Run

```bash
cargo check --workspace    # Verify compilation
cargo run -p api-db        # Run API server
cargo run -p ble-stream    # Run BLE streamer
```

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `DATABASE_URL` | `postgres://knockshadow:knockshadow@127.0.0.1:5432/knockshadow` | PostgreSQL connection string |
| `PORT` | `3000` | HTTP server port |
| `JWT_SECRET` | `knockshadow_default_secret_change_me` | JWT signing secret |

## Dependencies of Note

- `bcrypt` — password hashing
- `jsonwebtoken` — JWT creation/validation
- `axum` — HTTP/WebSocket framework
- `sqlx` — async SQL with compile-time checks
