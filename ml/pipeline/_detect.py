"""Detección de picos (golpes) y extracción de ventanas centradas.

Se separa de `_signal.py` porque la detección/ventaneo es conceptualmente
posterior al filtrado: trabaja sobre el DataFrame ya fusionado y filtrado.
"""

from __future__ import annotations

import numpy as np
import pandas as pd
from scipy.signal import find_peaks

from ._constants import FEATURE_COLS, HIT_THRESHOLD_G, WINDOW_SIZE


def detect_hits(
    df: pd.DataFrame,
    threshold: float = HIT_THRESHOLD_G,
    distance: int = 15,
) -> np.ndarray:
    """Devuelve los índices de picos en `df['mag']` por encima de `threshold`.

    `distance` es el mínimo de muestras entre picos (anti-rebote, equivalente
    a ~250 ms a 60 Hz). Devuelve array vacío si el DataFrame no tiene columna
    `mag` — eso pasa cuando `merge_sensors` no tuvo datos suficientes.
    """
    if df.empty or "mag" not in df.columns:
        return np.array([], dtype=int)
    peaks, _ = find_peaks(df["mag"].values, height=threshold, distance=distance)
    return peaks


def create_windows(
    df: pd.DataFrame,
    peaks: np.ndarray,
    window_size: int = WINDOW_SIZE,
    return_valid_peaks: bool = False,
):
    """Extrae ventanas centradas en cada pico, descartando las que no caben.

    Returns:
        Si `return_valid_peaks` es False: ndarray shape (n, window_size, len(FEATURE_COLS)).
        Si True: tupla (windows, valid_peaks) donde `valid_peaks` son los
        índices de pico que produjeron ventana válida (en orden).
    """
    windows = []
    valid_peaks = []
    half = window_size // 2
    n = len(df)

    for p in peaks:
        start = p - half
        end = p + half
        if start < 0 or end > n:
            continue
        window = df.iloc[start:end][FEATURE_COLS].values
        if len(window) == window_size:
            windows.append(window)
            valid_peaks.append(int(p))

    if not windows:
        empty = np.empty((0, window_size, len(FEATURE_COLS)), dtype=np.float32)
        if return_valid_peaks:
            return empty, np.array([], dtype=int)
        return empty

    result = np.array(windows, dtype=np.float32)
    if return_valid_peaks:
        return result, np.array(valid_peaks, dtype=int)
    return result
