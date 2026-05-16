"""Paquete pipeline: procesamiento de señal, detección y persistencia del dataset.

Antes de Phase C.1 todo esto vivía en un único `pipeline.py` de 344 líneas
mezclando constantes, lectura de DB, filtros, ventaneo y manejo de
`dataset.npz`. La división en submódulos privados con prefijo `_` permite:

    - Lectura modular: cada archivo cabe en una pantalla.
    - Tests focalizados: los nuevos tests apuntan a piezas concretas.
    - Migración futura sin tocar imports: si reemplazamos `_dataset.py` por
      un backend SQLite, los consumidores siguen viendo la misma API.

Las funciones públicas se re-exportan aquí para mantener back-compat con
todos los `from pipeline import X` existentes (`pi_inference.py`, `main.py`,
`train.py`, `app.py`, tests).
"""

from __future__ import annotations

# Constantes (re-export plano para que `pipeline.WINDOW_SIZE` siga funcionando).
from ._constants import (
    DB_PATH,
    DB_URL,
    DEFAULT_DATASET,
    FEATURE_COLS,
    HIT_THRESHOLD_G,
    SAMPLE_RATE,
    SENSOR_MAC_1,
    SENSOR_MAC_2,
    SENSOR_SCALE,
    WINDOW_SIZE,
)

# Detección y ventaneo.
from ._detect import create_windows, detect_hits

# I/O de muestras BLE.
from ._io import get_latest_sample_per_sensor, load_data

# Procesamiento de señal.
from ._signal import lowpass_filter, merge_sensors

# Persistencia del dataset etiquetado.
from ._dataset import (
    delete_last_samples,
    delete_samples_by_id,
    delete_samples_by_label,
    get_recent_samples,
    load_dataset,
    relabel_samples_by_label,
    save_dataset,
)

__all__ = [
    # constantes
    "DB_PATH",
    "DB_URL",
    "DEFAULT_DATASET",
    "FEATURE_COLS",
    "HIT_THRESHOLD_G",
    "SAMPLE_RATE",
    "SENSOR_MAC_1",
    "SENSOR_MAC_2",
    "SENSOR_SCALE",
    "WINDOW_SIZE",
    # signal
    "lowpass_filter",
    "merge_sensors",
    # detection
    "detect_hits",
    "create_windows",
    # io
    "load_data",
    "get_latest_sample_per_sensor",
    # dataset
    "save_dataset",
    "load_dataset",
    "delete_last_samples",
    "delete_samples_by_id",
    "delete_samples_by_label",
    "relabel_samples_by_label",
    "get_recent_samples",
]
