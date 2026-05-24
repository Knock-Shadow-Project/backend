# Pi Inference — Documentación

Script de inferencia CNN adaptado para correr **offline** en Raspberry Pi. Lee muestras BLE desde una **SQLite local** (en lugar de PostgreSQL), detecta golpes, clasifica y guarda resultados en la misma base de datos.

---

## Diferencias con `main.py` (modo cloud)

| Aspecto | `main.py` (cloud) | `pi_inference.py` (offline) |
|---------|-------------------|-----------------------------|
| Fuente de datos | PostgreSQL (`psycopg2`) | SQLite local (`sqlite3`) |
| Destino de resultados | API REST + WebSocket remoto | SQLite local (`detected_punches`) |
| Entrenamiento | Creado vía API remota | Autodescubierto desde `local_trainings` |
| Persistencia | Requiere conexión a internet | Funciona sin red |
| Modelo CNN | Igual (`model/punch_classifier.pt`) | Igual |

---

## Arquitectura

```
┌─────────────┐      BLE       ┌──────────────┐
│  Sensores   │ ─────────────► │  pi-service  │
│   (1-2x)    │                │   (Rust)     │
└─────────────┘                └──────┬───────┘
                                      │ INSERT ble_samples
                                      ▼
                               ┌─────────────┐
                               │   SQLite    │
                               │  pi_data.db │
                               └──────┬──────┘
                                      │ SELECT ble_samples
                                      ▼
                               ┌─────────────┐
                               │pi_inference │
                               │  (Python)   │
                               └──┬───────┬──┘
                                  │       │
                    MQTT publish  │       │ INSERT detected_punches
                    (real-time)   │       │ (persist for cloud sync)
                                  ▼       ▼
                           ┌──────────┐ ┌─────────────┐
                           │  nanomq  │ │   SQLite    │
                           │  broker  │ │  pi_data.db │
                           └────┬─────┘ └─────────────┘
                                │
                    MQTT subscribe
                                │
                                ▼
                         ┌──────────────┐
                         │  pi-service  │──► WS /live ──► App Móvil
                         │   (Rust)     │
                         └──────────────┘
```

Los golpes detectados se publican vía MQTT (`knockshadow/punches`) para que
`pi-service` los retransmita por WebSocket en tiempo real. SQLite solo se usa
para persistencia y sincronización posterior con el cloud.

---

## Funciones principales

### `load_model()`

Wrapper local sobre `model_loader.load_model()` (módulo compartido entre la
inferencia cloud y la de Pi). Carga el modelo `PunchCNN` y los parámetros de
normalización desde `model/`:

- `model/punch_classifier.pt` — pesos del modelo PyTorch
- `model/class_names.npy` — array de nombres de clase
- `model/norm_mean.npy` — media de normalización z-score
- `model/norm_std.npy` — desviación estándar de normalización

Si los artefactos no existen, `model_loader` lanza `ModelNotFoundError`; el
wrapper en `pi_inference.py` lo captura, lo loguea estructuralmente con
`logging_config.get_logger` y termina el proceso con `sys.exit(1)`. Esto
evita que el contenedor entre en un loop de reinicio silencioso cuando el
volumen `ml/model/` está vacío.

### `load_data_sqlite(start_time, end_time)`

Lee muestras BLE de ambos sensores desde la tabla `ble_samples` de SQLite.

Devuelve un `DataFrame` con columnas:
- `received_at`, `device_mac`, `x`, `y`, `z`

> **Concurrencia SQLite.** El archivo `pi_data.db` es compartido por
> `pi-service` (Rust), `pi-inference` (Python) y `ml-app` (Streamlit). Toda
> conexión nueva abierta por `pi_inference.py` aplica `PRAGMA journal_mode=WAL`,
> `PRAGMA busy_timeout=5000` y `PRAGMA synchronous=NORMAL` — las mismas
> configuraciones que aplica `pi-service` al inicializar el archivo. Forzarlo
> en cada opener es necesario porque `journal_mode=WAL` es persistente a
> nivel de archivo pero `busy_timeout` es per-conexión, y porque
> `pi-inference` puede arrancar antes que `pi-service` (orden de
> `docker-compose`, reboot).

### `save_detected_punch(...)`

Inserta un golpe clasificado en `detected_punches` (solo durante entrenamiento
activo, para sincronización posterior con el cloud):

```sql
INSERT INTO detected_punches
(user_id, local_training_id, class_name, limb, position, power, prob)
VALUES (?, ?, ?, ?, ?, ?, ?)
```

### MQTT publish

Cada golpe detectado se publica inmediatamente al topic MQTT
`knockshadow/punches` con QoS 1 (at-least-once):

```json
{
  "class_name": "Jab",
  "limb": "Derecha",
  "position": "Cabeza",
  "power": 3.45,
  "prob": 0.9812,
  "detected_at": "2024-06-01T10:05:23.456Z"
}
```

`pi-service` se suscribe a este topic y reenvía los eventos por WebSocket.
Si el broker MQTT no está disponible, la inferencia sigue funcionando
(los punches se guardan en SQLite pero no llegan en tiempo real al WS).

### `get_active_training()`

Busca el entrenamiento activo (sin `end_time`) más reciente:

```sql
SELECT id, user_id FROM local_trainings
WHERE end_time IS NULL
ORDER BY start_time DESC LIMIT 1
```

Si encuentra uno, retorna `(local_training_id, user_id)`. Si no, retorna `(None, None)`.

### `parse_label(label: str) -> (name, limb, position)`

Convierte etiquetas ML como `jab_derecha_arriba` en tripletas:
- `name`: Jab / Cross / Gancho / Upper
- `limb`: Izquierda / Derecha
- `position`: Cabeza / Cuerpo

---

## AsyncInferenceEngine

Motor de inferencia continua con buffer circular.

### `_producer()`
- Lee de SQLite cada **1 segundo**.
- Añade nuevos DataFrames a una cola (`deque`).

### `_consumer()`
- Se ejecuta cada **100 ms**.
- Concatena nuevos datos al buffer acumulado.
- Elimina duplicados y recorta datos más antiguos que **5 segundos**.
- Ejecuta: `merge_sensors → detect_hits → create_windows → predict_windows`.
- **Autodetección de entrenamiento**: si no se pasó `--training-id`, re-verifica `local_trainings` cada ~10 segundos.
- Deduplicación por timestamp del pico (TTL = 10 s).
- Publica cada golpe vía MQTT (`knockshadow/punches`) para display en tiempo real.
- Guarda cada golpe en SQLite con `user_id` y `local_training_id` (solo durante training activo, para sync).

---

## CLI

```bash
python ml/pi_inference.py [opciones]
```

| Flag | Default | Descripción |
|------|---------|-------------|
| `--user-id` | `1` | ID de usuario para asociar los golpes |
| `--training-id` | `None` | ID local de entrenamiento (autodescubierto si no se indica) |

### Ejemplos

**Con entrenamiento explícito:**
```bash
export DB_PATH=pi_data.db
python ml/pi_inference.py --user-id 42 --training-id 7
```

**Autodetección de entrenamiento:**
```bash
export DB_PATH=pi_data.db
python ml/pi_inference.py --user-id 42
```

---

## Variables de entorno

| Variable | Default | Descripción |
|----------|---------|-------------|
| `DB_PATH` | `pi_data.db` | Ruta a la SQLite local |
| `SENSOR_MAC_1` | `DF:65:81:D0:D7:E5` | MAC del sensor izquierdo |
| `SENSOR_MAC_2` | `CB:01:10:3E:0D:61` | MAC del sensor derecho |
| `MQTT_HOST` | `127.0.0.1` | Host del broker MQTT (nanomq) |
| `MQTT_PORT` | `1883` | Puerto del broker MQTT |

---

## Docker

### Dockerfile

`Dockerfile.pi-inference` está basado en `python:3.12-slim` y mantiene la
imagen lo más ligera posible para correr en una Raspberry Pi 4:

1. **Sin compiladores.** No instalamos `gcc`/`g++`: `torch`, `numpy`, `scipy`
   y `pandas` publican wheels `aarch64` precompilados. Forzar builds desde
   código fuente solo hinchaba la imagen y aumentaba el tiempo de despliegue.
2. **Dependencias ligeras primero** desde `ml/requirements_inference.txt`
   (un subset del `requirements.txt` general — sin `streamlit`, `matplotlib`,
   etc.).
3. **PyTorch CPU-only**: instalamos `torch` desde
   `https://download.pytorch.org/whl/cpu`. Los wheels Linux por defecto en
   PyPI traen `nvidia-cublas` / `nvidia-cuda-runtime` / `nvidia-cudnn` y
   pesan ~2 GB; el índice `cpu` envía un único wheel de ~200 MB sin CUDA.
4. **Copia mínima de código.** El Dockerfile copia exactamente los módulos
   que importa el path de inferencia: el paquete `pipeline/`, `model_def.py`,
   `model_loader.py`, `logging_config.py`, `pi_inference.py` y el directorio
   `model/`. `train.py` se excluye a propósito — arrastraría `matplotlib`,
   `seaborn` y `scikit-learn`, que la inferencia nunca importa.

   > **Nota:** desde Phase C.1 `pipeline` es un paquete Python (carpeta
   > `ml/pipeline/` con submódulos `_constants.py`, `_io.py`, `_signal.py`,
   > `_detect.py`, `_dataset.py`). Si tu Dockerfile aún hace
   > `COPY ml/pipeline.py …` falla en build — actualízalo a
   > `COPY ml/pipeline ./pipeline`.
5. Define `DB_PATH=/data/pi_data.db` y `PYTHONUNBUFFERED=1`.

### Uso con docker-compose

En `docker-compose.pi.yaml`:

```yaml
  pi-inference:
    build:
      context: .
      dockerfile: Dockerfile.pi-inference
    environment:
      DB_PATH: /data/pi_data.db
      SENSOR_MAC_1: DF:65:81:D0:D7:E5
      SENSOR_MAC_2: CB:01:10:3E:0D:61
    volumes:
      - pi-data:/data
    network_mode: host
    command:
      - python
      - pi_inference.py
      - --user-id
      - "1"
```

> **Nota:** Este contenedor **no necesita privilegios de Bluetooth**. Usa
> `network_mode: host` para alcanzar nanomq en `localhost:1883` y publicar
> punches vía MQTT. Solo requiere acceso al volumen compartido `pi-data`
> donde reside la SQLite.

---

## Flujo de trabajo recomendado

1. **Iniciar pi-service** (Terminal 1):
   ```bash
   export DB_PATH=pi_data.db
   export DEVICE_MAC_1=DF:65:81:D0:D7:E5
   export DEVICE_MAC_2=CB:01:10:3E:0D:61
   cargo run -p pi-service
   ```

2. **Iniciar inferencia** (Terminal 2):
   ```bash
   export DB_PATH=pi_data.db
   python ml/pi_inference.py --user-id 1
   ```

3. **App móvil** inicia entrenamiento vía `POST /training/start`.

4. `pi_inference.py` autodescubre el entrenamiento activo y empieza a guardar golpes.

5. La app recibe golpes en tiempo real por `WS /live`.

6. Al finalizar, la app llama `POST /training/stop`.

---

## Dependencias principales

| Paquete | Versión | Uso |
|---------|---------|-----|
| `torch` | ≥2.9 | Framework de deep learning |
| `numpy` | ≥1.24 | Arrays y operaciones numéricas |
| `pandas` | ≥2.0 | Manipulación de series temporales |
| `scipy` | ≥1.11 | Filtros Butterworth, detección de picos |
| `paho-mqtt` | ≥2.1 | Cliente MQTT para publicar golpes en tiempo real |
| `scikit-learn` | ≥1.4 | Split, encoding, métricas (training) |
| `matplotlib` / `seaborn` | ≥3.8 / ≥0.13 | Visualizaciones (training) |

> `psycopg2`, `requests`, `websockets` y `streamlit` no se usan en
> `pi_inference.py`. La imagen de Pi instala únicamente
> `requirements_inference.txt` (el subset usado por la inferencia) +
> `torch` desde el índice CPU de PyTorch, manteniendo el image footprint
> bajo. El `requirements.txt` completo se usa en cloud / training / la app
> Streamlit.
