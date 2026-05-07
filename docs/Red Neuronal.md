# Red Neuronal — Documentación

El módulo de **Machine Learning** clasifica los golpes de boxeo en tiempo real a partir de los datos de aceleración capturados por los sensores BLE. Consta de tres scripts principales: `pipeline.py`, `train.py` y `main.py`.

---

## Arquitectura general

```
┌─────────────────┐     Raw BLE      ┌─────────────────┐     Ventanas      ┌─────────────────┐
│  PostgreSQL     │  ──────────────► │  pipeline.py    │  ──────────────► │  PunchCNN       │
│  (ble_samples)  │                  │  (carga/filtro) │                  │  (PyTorch)      │
└─────────────────┘                  └─────────────────┘                  └─────────────────┘
                                                                                │
                                                                                │ softmax
                                                                                ▼
                                                                        ┌─────────────────┐
                                                                        │  jab_izq_arriba │ 95%
                                                                        │  cross_der_cabeza│ 3%
                                                                        └─────────────────┘
```

---

## 1. Pipeline de datos (`pipeline.py`)

Este módulo se encarga de la **adquisición, sincronización, filtrado y ventaneado** de las señales de los dos sensores.

### Variables de entorno

| Variable | Por defecto | Descripción |
|----------|-------------|-------------|
| `DATABASE_URL` | `postgres://knockshadow:knockshadow@127.0.0.1:5432/knockshadow` | Conexión PostgreSQL |
| `SENSOR_MAC_1` | `DF:65:81:D0:D7:E5` | MAC del sensor izquierdo |
| `SENSOR_MAC_2` | `CB:01:10:3E:0D:61` | MAC del sensor derecho |
| `SAMPLE_RATE` | `50` | Frecuencia de muestreo (Hz) |
| `WINDOW_SIZE` | `64` | Tamaño de la ventana de análisis (muestras) |
| `HIT_THRESHOLD_G` | `1.5` | Umbral mínimo de magnitud para detectar un golpe (G) |
| `SENSOR_SCALE` | `1000.0` | Factor de escala: unidades raw → G |

### Funciones principales

#### `load_data(start_time, end_time)`

Carga las muestras BLE de ambos sensores en un intervalo temporal. Devuelve un `DataFrame` de pandas con columnas:
- `received_at`, `device_mac`, `x`, `y`, `z`

#### `merge_sensors(df)`

1. Separa los datos de cada sensor por MAC.
2. Renombra ejes: sensor 1 → `x1, y1, z1`; sensor 2 → `x2, y2, z2`.
3. Sincroniza ambos DataFrames con `pd.merge_asof` (tolerancia de 100 ms).
4. Aplica un **filtro paso-bajo** Butterworth de orden 3 y frecuencia de corte 10 Hz.
5. Calcula la magnitud de cada sensor y la media:
   ```
   mag1 = sqrt(x1² + y1² + z1²)
   mag2 = sqrt(x2² + y2² + z2²)
   mag  = (mag1 + mag2) / 2
   ```

#### `detect_hits(df, threshold, distance)`

Usa `scipy.signal.find_peaks` sobre la columna `mag` para localizar los picos que superan el umbral. El parámetro `distance=15` evita picos múltiples muy cercanos (debouncing).

#### `create_windows(df, peaks)`

Para cada pico detectado, extrae una ventana simétrica de `WINDOW_SIZE` muestras (32 antes y 32 después del pico). Cada ventana tiene forma:

```
(window_size, 6)  →  [x1, y1, z1, x2, y2, z2]
```

Si la ventana queda fuera de los límites del DataFrame, se descarta.

---

## 2. Entrenamiento (`train.py`)

### Arquitectura: `PunchCNN`

Red neuronal convolucional 1D diseñada para series temporales multicanal:

```
Input:  (batch, 64, 6)          # 64 muestras, 6 canales

Conv1d(6 → 32, k=5) → BatchNorm → ReLU → MaxPool(2)   # 64 → 32
Conv1d(32 → 64, k=5) → BatchNorm → ReLU → MaxPool(2)  # 32 → 16
Conv1d(64 → 128, k=3) → ReLU                         # 16 → 16

Global Average Pooling → 128 valores
Linear(128 → 64) → ReLU → Dropout(0.3)
Linear(64 → num_classes)
```

| Capa | Parámetros entrenables |
|------|------------------------|
| Features (3 conv) | ~35 K |
| Classifier (2 linear) | ~9 K |
| **Total** | **~44 K** |

#### Preprocesamiento

1. **Normalización** z-score por canal y tiempo:
   ```python
   mean = X.mean(axis=(0, 1), keepdims=True)
   std  = X.std(axis=(0, 1), keepdims=True) + 1e-8
   X_norm = (X - mean) / std
   ```
2. **Codificación de etiquetas** con `sklearn.LabelEncoder`.

#### Aumentación de datos

Se añade ruido gaussiano (σ = 0.05) duplicando el conjunto de entrenamiento:
```python
X_aug = X + np.random.normal(0, 0.05, X.shape)
```

#### Hiperparámetros

| Parámetro | Valor |
|-----------|-------|
| Optimizador | Adam (lr = 1e-3) |
| Scheduler | ReduceLROnPlateau (factor 0.5, patience 5) |
| Pérdida | CrossEntropyLoss |
| Batch size (train) | 32 |
| Batch size (val) | 64 |
| Epochs máximas | 80 |
| Early stopping | patience = 30 epochs |
| Split train/val | 80/20 (estratificado) |

#### Artefactos generados

| Archivo | Contenido |
|---------|-----------|
| `model/punch_classifier.pt` | Pesos del modelo + metadatos |
| `model/class_names.npy` | Array de nombres de clase |
| `model/norm_mean.npy` | Media de normalización |
| `model/norm_std.npy` | Desviación estándar de normalización |
| `model/training_history.png` | Gráficas de accuracy y loss |
| `model/confusion_matrix.png` | Matriz de confusión en validación |

#### Guardado del modelo

```python
torch.save({
    "model_state": best_state,
    "num_classes": num_classes,
    "in_channels": len(FEATURE_COLS),  # 6
    "class_names": class_names.tolist(),
}, MODEL_PATH)
```

---

## 3. Inferencia (`main.py`)

### `AsyncInferenceEngine`

Clase que orquesta la inferencia en tiempo real mediante **dos tareas asyncio concurrentes**:

#### `_producer()`
- Lee de PostgreSQL cada **1 segundo** usando `asyncio.to_thread(load_data, ...)`.
- Añade los nuevos datos a una cola (`deque`) thread-safe.

#### `_consumer()`
- Se ejecuta cada **100 ms**.
- Concatena los DataFrames nuevos al acumulado en memoria.
- Elimina duplicados exactos y recorta datos más antiguos que **5 segundos**.
- Ejecuta `merge_sensors → detect_hits → create_windows → predict_windows`.
- **Deduplicación**: usa un `set` de `pd.Timestamp` de los picos ya procesados. Cada golpe se clasifica una sola vez.
- Envía resultados inmediatamente por WebSocket y/o API REST.
- Limpia timestamps procesados antiguos cada ciclo (TTL = 10 s).

### Flujo de ejecución (modo continuo)

```
1. Carga el modelo PunchCNN y los parámetros de normalización.
2. Arranca dos tareas concurrentes con asyncio:
   a. Productor: lee de PostgreSQL cada 1 segundo y acumula en un buffer circular.
   b. Consumidor: procesa continuamente el buffer, detecta picos y clasifica.
3. El buffer mantiene los últimos 5 segundos de datos en memoria.
4. Cada vez que se detecta un golpe nuevo:
   a. Extrae la ventana centrada en el pico.
   b. Normaliza y pasa por el modelo.
   c. Obtiene la top-3 predicción con probabilidad y la potencia (magnitud pico).
   d. Envía inmediatamente por WebSocket y/o guarda en la API REST.
   e. Marca el timestamp del pico como "procesado" para evitar duplicados.
5. Los timestamps procesados se limpian automáticamente tras 10 segundos.
```

### CLI

```bash
python main.py [opciones]
```

| Flag | Descripción |
|------|-------------|
| `--api` | Crea entrenamiento + historial en la API |
| `--ws` | Strea resultados por WebSocket |
| `--api-user-id` | ID de usuario para el entrenamiento (default: 1) |

### Mapeo de etiquetas ML → base de datos

| Etiqueta ML | `nombre` | `extremidad` | `posicion` |
|-------------|----------|--------------|------------|
| `jab_izquierda_arriba` | Jab | Izquierda | Cabeza |
| `jab_derecha_abajo` | Jab | Derecha | Cuerpo |
| `cross_izquierda_arriba` | Cross | Izquierda | Cabeza |
| `hook_derecha_abajo` | Gancho | Derecha | Cuerpo |
| `uppercut_derecha_arriba` | Upper | Derecha | Cabeza |

> El mapeo se realiza en `api_client.py` comparando `(nombre, extremidad, posicion)` con la tabla `GOLPE` de la API.

---

## 4. Aplicación de etiquetado (`app.py`)

Interfaz web basada en **Streamlit** para crear y gestionar el dataset de entrenamiento.

### Funcionalidades

- **Grabación** — inicia/para la captura de datos BLE.
- **Visualización** — gráficos interactivos con Plotly de la señal cruda y los picos detectados.
- **Etiquetado** — asigna tipo (`jab`, `cross`, `hook`, `uppercut`) y posición a cada golpe detectado.
- **Gestión del dataset** — añade, elimina, re-etiqueta o borra muestras por etiqueta.
- **Atajos de teclado** — `1-4` para tipo, `Q-Y` para posición.

### Atajos

| Tecla | Tipo | Tecla | Posición |
|-------|------|-------|----------|
| `1` | jab | `Q` | izquierda_arriba |
| `2` | cross | `W` | izquierda_abajo |
| `3` | hook | `E` | frente_arriba |
| `4` | uppercut | `R` | frente_abajo |
| | | `T` | derecha_arriba |
| | | `Y` | derecha_abajo |

---

## Dependencias principales

| Paquete | Versión | Uso |
|---------|---------|-----|
| `torch` | ≥2.9 | Framework de deep learning |
| `numpy` | ≥1.24 | Arrays y operaciones numéricas |
| `pandas` | ≥2.0 | Manipulación de series temporales |
| `scipy` | ≥1.11 | Filtros Butterworth, detección de picos |
| `scikit-learn` | ≥1.4 | Split, encoding, métricas |
| `psycopg2` | ≥2.9 | Conexión PostgreSQL |
| `requests` | ≥2.32 | Cliente API REST |
| `websockets` | ≥15.0 | Cliente WebSocket |
| `streamlit` | ≥1.35 | App web de etiquetado |
| `plotly` | ≥5.18 | Gráficos interactivos |
| `matplotlib` / `seaborn` | ≥3.8 / ≥0.13 | Visualizaciones de entrenamiento |

---

## Flujo de trabajo recomendado

1. **Calibrar sensores** — asegúrate de que ambos MACs envían datos.
2. **Etiquetar muestras** — usa `app.py` para grabar sesiones y etiquetar golpes.
3. **Entrenar modelo** — ejecuta `python train.py` hasta convergencia.
4. **Inferencia en tiempo real** — ejecuta `python main.py --loop --api --ws`.
5. **Consultar resultados** — consume la API REST o el WebSocket desde el frontend.
