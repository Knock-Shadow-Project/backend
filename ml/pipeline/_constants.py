"""Constantes y defaults compartidos por el pipeline ML.

Mantener las constantes en un único lugar evita drift entre módulos cuando
añadimos features: `WINDOW_SIZE` cambia aquí y todos los consumidores lo
reciben automáticamente. Los valores se leen del entorno con defaults
estables para CI/desarrollo local.

Para configuración de runtime tipada usa `ml/config.py`. Este módulo expone
las constantes "frozen" necesarias en el código de procesamiento sin pasar
un objeto Config por cada llamada.
"""

from __future__ import annotations

import os

DB_URL: str = os.getenv(
    "DATABASE_URL", "postgres://knockshadow:knockshadow@127.0.0.1:5432/knockshadow"
)
DB_PATH: str | None = os.getenv("DB_PATH")
SENSOR_MAC_1: str = os.getenv("SENSOR_MAC_1", "DF:65:81:D0:D7:E5")
SENSOR_MAC_2: str = os.getenv("SENSOR_MAC_2", "CB:01:10:3E:0D:61")
SAMPLE_RATE: int = int(os.getenv("SAMPLE_RATE", "50"))
WINDOW_SIZE: int = int(os.getenv("WINDOW_SIZE", "64"))
HIT_THRESHOLD_G: float = float(os.getenv("HIT_THRESHOLD_G", "1.5"))
SENSOR_SCALE: float = float(os.getenv("SENSOR_SCALE", "1000.0"))

_DATA_DIR: str = os.path.join(
    os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "data"
)
DEFAULT_DATASET: str = os.path.join(_DATA_DIR, "dataset.npz")

# Columnas esperadas en el DataFrame fusionado de dos sensores.
FEATURE_COLS: list[str] = ["x1", "y1", "z1", "x2", "y2", "z2"]
