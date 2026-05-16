"""Procesamiento de señal: filtrado y fusión de los dos sensores BLE.

Las funciones aquí son puras (input → output) y no leen el sistema de
archivos ni la base de datos. Son las funciones más testeadas — ver
`tests/test_pipeline.py`.
"""

from __future__ import annotations

import numpy as np
import pandas as pd
from scipy.signal import butter, filtfilt

from ._constants import FEATURE_COLS, SAMPLE_RATE, SENSOR_MAC_1, SENSOR_MAC_2, SENSOR_SCALE


def lowpass_filter(
    signal: np.ndarray, cutoff: float = 10.0, fs: int = SAMPLE_RATE
) -> np.ndarray:
    """Aplica un Butterworth low-pass orden 3 con filtfilt (fase cero).

    Devuelve la señal original sin tocar si es demasiado corta para
    `filtfilt`'s padlen (≤ 9 muestras). Eso evita un ValueError ruidoso
    durante la inferencia con buffers cortos al arranque.
    """
    if len(signal) <= 9:
        return signal
    b, a = butter(3, cutoff / (0.5 * fs), btype="low")
    return filtfilt(b, a, signal)


def merge_sensors(
    df: pd.DataFrame,
    mac1: str = SENSOR_MAC_1,
    mac2: str = SENSOR_MAC_2,
) -> pd.DataFrame:
    """Fusiona los dos streams BLE en un solo DataFrame con `FEATURE_COLS`.

    Estrategia: `merge_asof` con tolerancia de 100 ms (~6 muestras a 60 Hz).
    Filtra paso bajo y deriva magnitudes (`mag1`, `mag2`, `mag`).
    Devuelve DataFrame vacío si alguno de los sensores no tiene datos en
    el rango — el consumidor debe tratar el caso vacío.
    """
    df1 = df[df["device_mac"] == mac1][["received_at", "x", "y", "z"]].copy()
    df2 = df[df["device_mac"] == mac2][["received_at", "x", "y", "z"]].copy()

    df1 = df1.sort_values("received_at").rename(
        columns={"x": "x1", "y": "y1", "z": "z1"}
    )
    df2 = df2.sort_values("received_at").rename(
        columns={"x": "x2", "y": "y2", "z": "z2"}
    )

    if df1.empty or df2.empty:
        return pd.DataFrame()

    merged = pd.merge_asof(
        df1,
        df2,
        on="received_at",
        direction="nearest",
        tolerance=pd.Timedelta("100ms"),
    ).dropna()

    if merged.empty:
        return pd.DataFrame()

    for col in FEATURE_COLS:
        merged[col] = lowpass_filter(merged[col].values / SENSOR_SCALE)

    merged["mag1"] = np.sqrt(
        merged["x1"] ** 2 + merged["y1"] ** 2 + merged["z1"] ** 2
    )
    merged["mag2"] = np.sqrt(
        merged["x2"] ** 2 + merged["y2"] ** 2 + merged["z2"] ** 2
    )
    merged["mag"] = (merged["mag1"] + merged["mag2"]) / 2

    return merged.reset_index(drop=True)
