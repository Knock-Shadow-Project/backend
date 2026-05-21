"""Tests unitarios para `pipeline.py`: detección de picos y ventaneo.

Foco en las funciones puras (sin dependencia de DB ni del modelo):
    - `detect_hits`: detección de picos en magnitud.
    - `create_windows`: extracción de ventanas centradas en picos.
    - `merge_sensors`: fusión de dos streams BLE.

Reutilizan las constantes públicas del módulo (`WINDOW_SIZE`, `FEATURE_COLS`)
para que el test acompañe cualquier cambio de configuración.
"""

from __future__ import annotations

import numpy as np
import pandas as pd
import pytest

import pipeline
from pipeline import (
    FEATURE_COLS,
    WINDOW_SIZE,
    create_windows,
    detect_hits,
    merge_sensors,
)


def _make_df_with_peak(peak_at: int, length: int = 200) -> pd.DataFrame:
    """DataFrame con un pico claro en `peak_at` en la columna `mag`."""
    base = np.full(length, 0.5)
    base[peak_at] = 5.0  # bien por encima de HIT_THRESHOLD_G
    return pd.DataFrame(
        {
            "received_at": pd.date_range("2026-05-16", periods=length, freq="20ms"),
            "mag": base,
            # Columnas FEATURE_COLS necesarias para create_windows.
            **{col: np.zeros(length) for col in FEATURE_COLS},
        }
    )


class TestDetectHits:
    """`detect_hits` localiza picos por encima del threshold."""

    def test_returns_empty_for_empty_df(self):
        assert detect_hits(pd.DataFrame()).size == 0

    def test_returns_empty_when_mag_column_missing(self):
        df = pd.DataFrame({"x": [1.0, 2.0, 3.0]})
        assert detect_hits(df).size == 0

    def test_finds_single_peak_above_threshold(self):
        df = _make_df_with_peak(peak_at=100)
        peaks = detect_hits(df, threshold=1.5)
        assert len(peaks) == 1
        assert peaks[0] == 100

    def test_ignores_peaks_below_threshold(self):
        df = _make_df_with_peak(peak_at=100)
        # Bajamos el pico para que quede por debajo del threshold solicitado.
        df.loc[100, "mag"] = 0.5
        assert detect_hits(df, threshold=1.5).size == 0


class TestCreateWindows:
    """`create_windows` extrae ventanas centradas válidas alrededor de picos."""

    def test_returns_empty_array_when_no_peaks(self):
        df = _make_df_with_peak(peak_at=100)
        windows = create_windows(df, peaks=np.array([], dtype=int))
        assert windows.shape == (0, WINDOW_SIZE, len(FEATURE_COLS))

    def test_extracts_window_of_correct_shape(self):
        df = _make_df_with_peak(peak_at=100)
        windows = create_windows(df, peaks=np.array([100]))
        assert windows.shape == (1, WINDOW_SIZE, len(FEATURE_COLS))

    def test_drops_peaks_too_close_to_borders(self):
        df = _make_df_with_peak(peak_at=100)
        # Picos en 0 y el final no tienen ventana completa.
        windows = create_windows(df, peaks=np.array([0, 100, len(df) - 1]))
        # Sólo el de 100 sobrevive.
        assert windows.shape[0] == 1

    def test_return_valid_peaks_aligns_with_windows(self):
        df = _make_df_with_peak(peak_at=100)
        windows, valid = create_windows(
            df, peaks=np.array([0, 100]), return_valid_peaks=True
        )
        assert windows.shape[0] == 1
        assert list(valid) == [100]


class TestMergeSensors:
    """`merge_sensors` une dos streams BLE con tolerancia temporal."""

    def test_returns_empty_when_one_sensor_is_empty(self):
        # Sólo MAC1 tiene datos: sin coincidencia con MAC2 → vacío.
        ts = pd.date_range("2026-05-16", periods=10, freq="20ms")
        df = pd.DataFrame(
            {
                "received_at": ts,
                "device_mac": [pipeline.SENSOR_MAC_1] * 10,
                "x": np.zeros(10),
                "y": np.zeros(10),
                "z": np.zeros(10),
            }
        )
        merged = merge_sensors(df)
        assert merged.empty

    def test_pairs_close_samples_from_both_sensors(self):
        # Necesitamos suficientes muestras (>= 13) para que filtfilt aplique
        # padlen sin levantar ValueError. 30 deja margen sin alargar el test.
        n = 30
        ts = pd.date_range("2026-05-16", periods=n, freq="20ms")
        rows = []
        for i in range(n):
            rows.append(
                {
                    "received_at": ts[i],
                    "device_mac": pipeline.SENSOR_MAC_1,
                    "x": 1.0,
                    "y": 0.0,
                    "z": 0.0,
                }
            )
            rows.append(
                {
                    "received_at": ts[i],
                    "device_mac": pipeline.SENSOR_MAC_2,
                    "x": 0.0,
                    "y": 1.0,
                    "z": 0.0,
                }
            )
        merged = merge_sensors(pd.DataFrame(rows))
        # Debe haber emparejado las muestras y derivado magnitudes.
        assert len(merged) == n
        assert "mag" in merged.columns
        assert "mag1" in merged.columns
        assert "mag2" in merged.columns

    def test_does_not_raise_when_mixing_truncated_and_ms_timestamps(self):
        """Regresión: una consulta puede solapar datos viejos (segundo entero,
        pre-fix de `pi-service`) y nuevos (ms). Si el spread del grupo viejo
        empuja timestamps por delante de los nuevos vecinos sin re-sort,
        `merge_asof` rompe con 'left keys must be sorted'.
        """
        # Sensor 1: grupo viejo de 50 muestras todas en 12:00:00, luego una
        # muestra "nueva" con ms en 12:00:00.100, luego otro grupo viejo.
        old = pd.Timestamp("2026-05-16 12:00:00")
        new_in_middle = pd.Timestamp("2026-05-16 12:00:00.100")
        next_old = pd.Timestamp("2026-05-16 12:00:01")
        rows = []
        for _ in range(50):
            rows.append(
                {
                    "received_at": old,
                    "device_mac": pipeline.SENSOR_MAC_1,
                    "x": 1.0,
                    "y": 0.0,
                    "z": 0.0,
                }
            )
        rows.append(
            {
                "received_at": new_in_middle,
                "device_mac": pipeline.SENSOR_MAC_1,
                "x": 1.0,
                "y": 0.0,
                "z": 0.0,
            }
        )
        for _ in range(50):
            rows.append(
                {
                    "received_at": next_old,
                    "device_mac": pipeline.SENSOR_MAC_1,
                    "x": 1.0,
                    "y": 0.0,
                    "z": 0.0,
                }
            )
        # Sensor 2 análogo, igual de denso.
        for _ in range(50):
            rows.append(
                {
                    "received_at": old,
                    "device_mac": pipeline.SENSOR_MAC_2,
                    "x": 0.0,
                    "y": 1.0,
                    "z": 0.0,
                }
            )
        for _ in range(50):
            rows.append(
                {
                    "received_at": next_old,
                    "device_mac": pipeline.SENSOR_MAC_2,
                    "x": 0.0,
                    "y": 1.0,
                    "z": 0.0,
                }
            )
        # Antes del fix esto levantaba ValueError("left keys must be sorted").
        merged = merge_sensors(pd.DataFrame(rows))
        assert not merged.empty
        # Y received_at del resultado queda monotónico.
        ts = merged["received_at"]
        assert ts.is_monotonic_increasing, "merge_sensors no devuelve filas ordenadas"

    def test_preserves_sub_second_variation_with_truncated_timestamps(self):
        """Regresión del bug del Sensor 2 (commit del fix received_at).

        Cuando los datos viejos venían de SQLite con `received_at` truncado
        al segundo, las ~50 muestras/s caían en el mismo timestamp y
        `merge_asof` emparejaba TODAS las filas del Sensor 1 con UNA sola
        muestra del Sensor 2 (meseta plana en el plot). El spread interno
        debe preservar la variación intra-segundo de ambos sensores.
        """
        n = 50  # típico ~50 Hz en 1 s
        # TODAS las muestras del mismo segundo: simula el bug original.
        same_second = pd.Timestamp("2026-05-16 12:00:00")
        rows = []
        for i in range(n):
            # Sensor 1: oscilación clara muestra-a-muestra
            rows.append(
                {
                    "received_at": same_second,
                    "device_mac": pipeline.SENSOR_MAC_1,
                    "x": float(i % 5),
                    "y": 0.0,
                    "z": -1000.0,
                }
            )
            # Sensor 2: también con variación clara
            rows.append(
                {
                    "received_at": same_second,
                    "device_mac": pipeline.SENSOR_MAC_2,
                    "x": float(i),  # 0, 1, 2, ..., 49 — rampa monótona
                    "y": 0.0,
                    "z": 1000.0,
                }
            )
        merged = merge_sensors(pd.DataFrame(rows))
        # Sin el spread, x2 sería constante (meseta). Con el spread, debe
        # variar — y notablemente, no todas las filas tienen el mismo x2.
        assert len(merged) >= n - 5, "demasiadas filas descartadas por merge_asof"
        assert merged["x2"].nunique() > 1, (
            "Sensor 2 colapsado a un solo valor — el spread no se aplicó"
        )


@pytest.mark.parametrize(
    "window_size",
    [16, 32, 64, 128],
)
def test_window_size_is_respected(window_size: int):
    """`create_windows` honra `window_size` cuando se pasa explícito."""
    df = _make_df_with_peak(peak_at=200, length=400)
    windows = create_windows(df, peaks=np.array([200]), window_size=window_size)
    assert windows.shape == (1, window_size, len(FEATURE_COLS))
