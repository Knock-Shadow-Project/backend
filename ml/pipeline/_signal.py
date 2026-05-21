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


def _spread_duplicate_timestamps(df: pd.DataFrame) -> pd.DataFrame:
    """Reparte uniformemente las filas que comparten un mismo `received_at`.

    Workaround para datos antiguos guardados antes de que `pi-service` pasara a
    insertar `received_at` con resolución de milisegundos: el DEFAULT
    `CURRENT_TIMESTAMP` de SQLite truncaba al segundo, así que las ~50
    muestras/s caían todas en el mismo timestamp y rompían `merge_asof`
    (todas se emparejaban con UNA sola muestra del otro sensor → meseta plana).

    Cada grupo de filas con el mismo `received_at` se distribuye linealmente
    dentro de ese segundo (offset = i / N segundos). Si no hay duplicados —
    caso de datos nuevos con timestamps sub-segundo — es un no-op funcional
    (todos los offsets son 0).

    Asume que el df ya viene ordenado por inserción dentro de cada grupo, que
    es el orden cronológico real entregado por el BLE.

    Reordena por `received_at` al final: cuando una consulta mezcla datos
    viejos (segundo entero) y nuevos (con ms), el spread del grupo viejo
    puede generar timestamps que caen entre los nuevos vecinos —
    p. ej. una fila vieja en `12.000` con N=50 acaba en `12.980` mientras
    que una nueva quedaba en `12.100`. Sin re-sort, `merge_asof` rompe con
    `ValueError: left keys must be sorted`.
    """
    if df.empty:
        return df
    df = df.copy()
    grp = df.groupby("received_at", sort=False)
    sizes = grp["received_at"].transform("size").to_numpy()
    idx = grp.cumcount().to_numpy()
    # i/N ∈ [0, 1) segundos → nanosegundos para timedelta
    offsets_ns = (idx.astype("int64") * 1_000_000_000) // np.maximum(sizes, 1).astype(
        "int64"
    )
    df["received_at"] = df["received_at"] + pd.to_timedelta(offsets_ns, unit="ns")
    return df.sort_values("received_at", kind="stable").reset_index(drop=True)


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

    # Despliega timestamps duplicados dentro del segundo. No-op para datos
    # nuevos con resolución sub-segundo; imprescindible para los antiguos.
    df1 = _spread_duplicate_timestamps(df1)
    df2 = _spread_duplicate_timestamps(df2)

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
