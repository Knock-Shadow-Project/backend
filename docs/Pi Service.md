# Pi Service — Documentación

Servicio **offline-first** para Raspberry Pi 4 que recibe datos de sensores BLE, persiste en SQLite local y expone una API HTTP/WebSocket para la aplicación móvil.

---

## Arquitectura general

```
┌─────────────┐   BLE   ┌──────────────┐   MQTT    ┌─────────────────┐
│  Sensores   │ ───────►│  pi-service  │◄──────────│  pi_inference   │
│   (1-2x)    │         │   (Rust)     │           │   (Python/CNN)  │
└─────────────┘         └──────┬───────┘           └────────┬────────┘
                               │                            │
                      ┌────────┼─────────┐     ┌────────────┼────────────┐
                      │        │         │     │            │            │
                      │  HTTP/WS    MQTT pub   │  MQTT pub        SQLite│
                      │  (punches   (accel     │  (punches)    (persist │
                      │   + accel)  downsamp.) │               for sync)│
                      ▼        │         │     ▼            │            │
               ┌─────────────┐ │  ┌──────┴─┐  ┌──────────┐ │ ┌─────────┐
               │  App Movil  │ │  │ nanomq │  │ nanomq   │ │ │ SQLite  │
               │ (mDNS desc) │ │  │ broker │  │ broker   │ │ │pi_data  │
               └─────────────┘ │  └────────┘  └──────────┘ │ └────┬────┘
                               │                            │      │
                               │ sync (si hay internet)     │      │
                               ▼                            ▼      ▼
                        ┌───────────────────────────────────────────────┐
                        │              api-db remota                    │
                        │         (PostgreSQL + API REST)               │
                        └───────────────────────────────────────────────┘
```

Los **punch events** fluyen en tiempo real por MQTT (`knockshadow/punches`):
`pi_inference` → nanomq → `pi-service` → WS broadcast → App Móvil.
SQLite solo almacena punches para sincronización posterior con el cloud.

---

## Módulos

### `bt.rs` — Comunicación BLE

Basado en `btleplug`. Soporta conexión a 1 o 2 sensores simultáneamente.

- **`run_ble(mac1, mac2, state)`**:
  1. Escanea y conecta los dispositivos configurados por MAC.
  2. Se suscribe a notificaciones de las características con UUID terminado en `-0001-11e1-ac36-0002a5d5c51b`.
  3. Inicia una tarea de batería cada 30 segundos por dispositivo.
  4. Mergea los streams de ambos sensores y guarda cada muestra en SQLite vía `db::insert_ble_sample`.
  5. Si el stream se corta, espera 5 segundos y reintenta.

- **`decode_data(payload)`**:
  - Soporta frame con timestamp (`[ts_lo, ts_hi, x_lo, x_hi, y_lo, y_hi, z_lo, z_hi]`) o sin timestamp.
  - Extrae `x`, `y`, `z` como `f32`.

---

### `db.rs` — Persistencia SQLite

Esquema autoinicializado al arrancar:

#### `ble_samples`
| Columna | Tipo | Descripción |
|---------|------|-------------|
| `id` | INTEGER PK | Autoincremental |
| `device_mac` | TEXT | MAC del sensor |
| `device_name` | TEXT | Nombre BLE del sensor |
| `ble_ts` | INTEGER | Timestamp del dispositivo (opcional) |
| `x`, `y`, `z` | REAL | Aceleración en unidades raw |
| `received_at` | DATETIME | Timestamp de recepción (default `CURRENT_TIMESTAMP`) |

#### `detected_punches`
| Columna | Tipo | Descripción |
|---------|------|-------------|
| `id` | INTEGER PK | Autoincremental |
| `user_id` | INTEGER | ID del usuario (viene de la app móvil) |
| `local_training_id` | INTEGER | FK a `local_trainings` |
| `class_name` | TEXT | Clase del golpe (Jab, Cross, etc.) |
| `limb` | TEXT | Extremidad (Izquierda/Derecha) |
| `position` | TEXT | Posición (Cabeza/Cuerpo) |
| `power` | REAL | Potencia/fuerza en G |
| `prob` | REAL | Confianza de la predicción |
| `detected_at` | DATETIME | Timestamp de detección |
| `synced` | BOOLEAN | `FALSE` hasta que se sincronice con la API remota |

#### `local_trainings`
| Columna | Tipo | Descripción |
|---------|------|-------------|
| `id` | INTEGER PK | Autoincremental |
| `user_id` | INTEGER | ID del usuario |
| `start_time` | DATETIME | Inicio del entrenamiento |
| `end_time` | DATETIME | Fin del entrenamiento (NULL = activo) |
| `training_type` | TEXT | Tipo de entrenamiento |
| `synced` | BOOLEAN | Estado de sync remoto |
| `remote_training_id` | INTEGER | ID asignado por la API remota tras sync |

#### `sync_queue`
| Columna | Tipo | Descripción |
|---------|------|-------------|
| `id` | INTEGER PK | Autoincremental |
| `method` | TEXT | HTTP method (POST, PUT, PATCH) |
| `endpoint` | TEXT | Ruta relativa de la API remota |
| `payload` | TEXT | Body JSON |
| `headers` | TEXT | Headers JSON (opcional) |
| `created_at` | DATETIME | Timestamp de encolado |

---

### `api.rs` — API local (Axum)

Todas las rutas incluyen CORS abierto para la app móvil.

| Método | Ruta | Descripción |
|--------|------|-------------|
| `GET` | `/` | Dashboard web embebido (HTML estático en `src/static/index.html`) |
| `GET` | `/health` | Healthcheck JSON |
| `GET` | `/training/active` | Devuelve si hay entrenamiento activo |
| `POST` | `/training/start` | Inicia entrenamiento local |
| `POST` | `/training/stop` | Finaliza entrenamiento local |
| `GET` | `/trainings/:id/punches` | Lista golpes de un entrenamiento |
| `GET` | `/live` | WebSocket con stream en tiempo real de golpes |
| `GET` | `/live/accel` | WebSocket con stream de acelerómetro (downsample 1:3) |

El dashboard se sirve directamente desde el binario (`include_str!`) y se conecta al WebSocket `/live` para mostrar el último golpe detectado, una lista de los últimos 10 y controles para iniciar/parar el entrenamiento. Abrir en el navegador: `http://<ip-de-la-pi>:8080/` (o `http://knockshadow-pi.local:8080/` vía mDNS).

#### `POST /training/start`

```json
{
  "user_id": 42,
  "jwt": "eyJhbG...",
  "training_type": "Standard"
}
```

Respuesta:
```json
{ "local_training_id": 7 }
```

#### `GET /training/active`

Respuesta si hay entrenamiento activo:
```json
{
  "active": true,
  "local_training_id": 7,
  "user_id": 42,
  "start_time": "2024-06-01T10:00:00Z"
}
```

Respuesta si no hay:
```json
{
  "active": false,
  "local_training_id": null,
  "user_id": null,
  "start_time": null
}
```

#### `WS /live`

Cada golpe detectado se emite como JSON:

```json
{
  "class_name": "Jab",
  "limb": "Derecha",
  "position": "Cabeza",
  "power": 3.45,
  "prob": 0.98,
  "detected_at": "2024-06-01T10:05:23Z"
}
```

El servidor recibe golpes en tiempo real vía MQTT (topic `knockshadow/punches`, publicado por `pi_inference`) y los retransmite por broadcast a todos los clientes WebSocket conectados. Esto reemplaza el antiguo polling de SQLite cada 100 ms, reduciendo latencia y carga en la base de datos.

---

### `mdns.rs` — Descubrimiento automático

Registra el servicio `_knockshadow._tcp.local.` **delegando en el `avahi-daemon`
del host** mediante un subproceso `avahi-publish-service` que habla por DBus.

- **No usamos un responder mDNS en el proceso** (`mdns-sd` u otros). En la
  Raspberry corre `network_mode: host` junto al `avahi-daemon` del sistema,
  así que cualquier socket UDP/5353 que abramos colisiona y los broadcasts se
  pierden en silencio. Apoyarnos en avahi vía DBus evita ese choque y permite
  reutilizar el descubrimiento ya configurado del host.
- El contenedor monta `/var/run/dbus` desde el host (ver
  `docker-compose.pi.yaml`) y la imagen incluye `avahi-utils`.
- La app móvil descubre la Pi en la red WiFi sin conocer su IP.
- El nombre del servicio se configura con `MDNS_HOSTNAME` (default:
  `knockshadow-pi`); el record TXT se publica con `path=/` y `version=1.0`.
- El handle devuelto por `mdns::announce()` mantiene vivo el subproceso. Al
  dropearlo (Ctrl-C o salida del binario) se mata `avahi-publish-service` y
  el servicio se de-registra automáticamente.

---

### `sync.rs` — Sincronización remota

Si `API_BASE_URL` está configurado, cada 30 segundos:

1. Lee hasta 50 elementos pendientes de `sync_queue` ordenados por `id`.
2. Envía cada petición a la API remota con el método y headers correspondientes.
3. Si la respuesta es exitosa (2xx), elimina el elemento de la cola.
4. Si falla por red o error 5xx, detiene el batch y reintentará en el próximo ciclo.

> **Nota:** Actualmente solo se encolan las actualizaciones de `end_time` de entrenamientos. La sincronización de punches (historial) puede añadirse extendiendo la lógica en `api.rs` o creando un proceso de backfill.

---

## Variables de entorno

| Variable | Default | Descripción |
|----------|---------|-------------|
| `DB_PATH` | `pi_data.db` | Ruta a la SQLite local |
| `PORT` | `8080` | Puerto del API HTTP/WS |
| `DEVICE_MAC_1` | — | MAC del primer sensor BLE |
| `DEVICE_MAC_2` | — | MAC del segundo sensor BLE |
| `API_BASE_URL` | — | URL base de la API remota (opcional) |
| `MDNS_HOSTNAME` | `knockshadow-pi` | Nombre mDNS del servicio |
| `MQTT_HOST` | `127.0.0.1` | Host del broker MQTT (nanomq) |
| `MQTT_PORT` | `1883` | Puerto del broker MQTT |
| `RUST_LOG` | `info` | Nivel de logging de tracing |

---

## Docker

### Dockerfile

`Dockerfile.pi-service` usa multi-stage build:

1. **Builder**: `rust:1.94-bookworm` con dependencias de BlueZ/D-Bus y OpenSSL.
2. **Runtime**: `debian:bookworm-slim` con `bluez`, `libdbus-1-3`, `libssl3` y
   `avahi-utils` (provee `avahi-publish-service`, requerido por
   `mdns::announce`).

### docker-compose.pi.yaml

```yaml
services:
  pi-service:
    build:
      context: .
      dockerfile: Dockerfile.pi-service
    privileged: true
    network_mode: host
    volumes:
      - pi-data:/data
      - /var/run/dbus:/var/run/dbus:ro       # avahi DBus socket
      - /dev/bus/usb:/dev/bus/usb            # BLE adapter on USB hosts
    environment:
      DB_PATH: /data/pi_data.db
      DEVICE_MAC_1: DF:65:81:D0:D7:E5
      DEVICE_MAC_2: CB:01:10:3E:0D:61
```

> **Importante:**
> - `privileged: true` y el mount de `/dev/bus/usb` son necesarios para que
>   `btleplug` hable con BlueZ del host.
> - `network_mode: host` deja a `avahi-daemon` del host atender los
>   broadcasts mDNS que llegan por la WiFi del usuario.
> - `/var/run/dbus` montado en read-only es lo que permite a
>   `avahi-publish-service` (dentro del contenedor) registrar el servicio en
>   el avahi del host. Sin este mount el announce falla en silencio.

### Compilación cruzada ARM64

Para compilar la imagen desde x86_64 hacia Raspberry Pi 4 (ARM64):

```bash
docker buildx build --platform linux/arm64 \
  -f Dockerfile.pi-service \
  -t knockshadow/pi-service:latest .
```

---

## Flujo de datos completo

```
1. App móvil descubre la Raspberry vía mDNS.
2. Llama POST /training/start con user_id.
3. pi-service crea fila en local_trainings y marca entrenamiento como activo.
4. Sensores BLE envían muestras cada ~16 ms.
5. pi-service decodifica y guarda en ble_samples (SQLite).
6. pi-service publica accel downsampled (1:5) a MQTT topic knockshadow/sensors/{mac}/accel.
7. pi_inference.py lee de ble_samples, detecta golpes con CNN.
8. pi_inference.py publica golpe a MQTT topic knockshadow/punches.
9. pi_inference.py guarda golpe en SQLite detected_punches (para sync posterior).
10. pi-service recibe golpe vía MQTT y emite por WS /live.
11. App móvil recibe el golpe en tiempo real.
12. App llama POST /training/stop al finalizar.
13. Si hay API_BASE_URL, sync.rs sube datos procesados a la API remota.
```

---

## Dependencias principales

| Crate | Versión | Uso |
|-------|---------|-----|
| `btleplug` | 0.12 | BLE multiplataforma |
| `axum` | 0.8 | HTTP + WebSocket server |
| `tower-http` | 0.6 | CORS middleware |
| `sqlx` | 0.8 | SQLite async con compile-time checks |
| `reqwest` | 0.12 | Cliente HTTP para sync remoto |
| `rumqttc` | 0.24 | Cliente MQTT async (publish accel + subscribe punches) |
| `tokio` | 1.52 | Runtime async |
| `serde` / `serde_json` | 1 | Serialización JSON |
| `chrono` | 0.4 | Timestamps |
| `uuid` | 1 | IDs de eventos / requests |

> mDNS no es una dependencia de cargo: lo provee el host vía `avahi-utils`
> (`avahi-publish-service`), invocado como subproceso desde
> `mdns::announce()`.
