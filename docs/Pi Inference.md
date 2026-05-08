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
                               └──────┬──────┘
                                      │ INSERT detected_punches
                                      ▼
                               ┌─────────────┐
                               │   SQLite    │
                               │  pi_data.db │
                               └─────────────┘
```

---

## Funciones principales

### `load_model()`

Carga el modelo `PunchCNN` y los parámetros de normalización desde `model/`:

- `model/punch_classifier.pt` — pesos del modelo PyTorch
- `model/class_names.npy` — array de nombres de clase
- `model/norm_mean.npy` — media de normalización z-score
- `model/norm_std.npy` — desviación estándar de normalización

### `load_data_sqlite(start_time, end_time)`

Lee muestras BLE de ambos sensores desde la tabla `ble_samples` de SQLite.

Devuelve un `DataFrame` con columnas:
- `received_at`, `device_mac`, `x`, `y`, `z`

### `save_detected_punch(...)`

Inserta un golpe clasificado en `detected_punches`:

```sql
INSERT INTO detected_punches
(user_id, local_training_id, class_name, limb, position, power, prob)
VALUES (?, ?, ?, ?, ?, ?, ?)
```

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
- Guarda cada golpe en SQLite con `user_id` y `local_training_id`.

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

---

## Docker

### Dockerfile

`Dockerfile.pi-inference` está basado en `python:3.12-slim`:

1. Instala `gcc` y `g++` (necesarios para compilar dependencias de `scipy`/`numpy`).
2. Instala dependencias Python desde `ml/requirements.txt`.
3. Copia `pipeline.py`, `train.py`, `pi_inference.py` y el directorio `model/`.
4. Define `DB_PATH=/data/pi_data.db`.

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
    command:
      - python
      - pi_inference.py
      - --user-id
      - "1"
```

> **Nota:** Este contenedor **no necesita privilegios de Bluetooth** ni `network_mode: host`. Solo requiere acceso al volumen compartido `pi-data` donde reside la SQLite.

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
| `scikit-learn` | ≥1.4 | Split, encoding, métricas (training) |
| `matplotlib` / `seaborn` | ≥3.8 / ≥0.13 | Visualizaciones (training) |

> `psycopg2`, `requests`, `websockets` y `streamlit` no se usan en `pi_inference.py` pero están incluidos en el `requirements.txt` compartido.
