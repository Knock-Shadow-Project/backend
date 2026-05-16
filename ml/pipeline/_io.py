"""Lectores de muestras BLE desde SQLite (Pi) y PostgreSQL (cloud).

`load_data` selecciona automáticamente la fuente según `DB_PATH` (Pi-offline)
o `DATABASE_URL` (cloud). Esto permite que `pi_inference.py` y `main.py`
usen la misma interfaz sin saber el modo del despliegue.
"""

from __future__ import annotations

import datetime as _dt
import sqlite3

import pandas as pd

from ._constants import DB_PATH, DB_URL, SENSOR_MAC_1, SENSOR_MAC_2


def _fmt_sqlite_ts(dt) -> str:
    """SQLite's CURRENT_TIMESTAMP stores `YYYY-MM-DD HH:MM:SS` (space, no TZ,
    second precision). Comparisons are lexicographic, so query params MUST be
    formatted the same way — `isoformat()` ('T' separator + microseconds +
    `+00:00`) collates BEFORE every stored row and the BETWEEN returns nothing.
    Coerce to UTC and emit the SQLite canonical form."""
    if dt.tzinfo is None:
        dt = dt.replace(tzinfo=_dt.timezone.utc)
    return dt.astimezone(_dt.timezone.utc).strftime("%Y-%m-%d %H:%M:%S")


def _load_data_sqlite(start_time, end_time) -> pd.DataFrame:
    # Modo read-only + timeout para evitar "database is locked" cuando
    # pi-service escribe simultaneamente (aunque con WAL ya casi nunca pasa).
    conn = sqlite3.connect(f"file:{DB_PATH}?mode=ro", uri=True, timeout=20.0)
    query = """
    SELECT received_at, device_mac, x, y, z
    FROM ble_samples
    WHERE received_at BETWEEN ? AND ?
      AND device_mac IN (?, ?)
    ORDER BY received_at ASC;
    """
    df = pd.read_sql(
        query,
        conn,
        params=(
            _fmt_sqlite_ts(start_time),
            _fmt_sqlite_ts(end_time),
            SENSOR_MAC_1,
            SENSOR_MAC_2,
        ),
    )
    conn.close()
    df["received_at"] = pd.to_datetime(df["received_at"], utc=True)
    return df


def _load_data_postgres(start_time, end_time, db_url: str) -> pd.DataFrame:
    import psycopg2

    conn = psycopg2.connect(db_url)
    query = """
    SELECT received_at, device_mac, x, y, z
    FROM ble_samples
    WHERE received_at BETWEEN %s AND %s
      AND device_mac IN (%s, %s)
    ORDER BY received_at ASC;
    """
    df = pd.read_sql(
        query, conn, params=(start_time, end_time, SENSOR_MAC_1, SENSOR_MAC_2)
    )
    conn.close()
    df["received_at"] = pd.to_datetime(df["received_at"], utc=True)
    return df


def load_data(start_time, end_time, db_url: str = DB_URL) -> pd.DataFrame:
    if DB_PATH:
        return _load_data_sqlite(start_time, end_time)
    return _load_data_postgres(start_time, end_time, db_url)


def get_latest_sample_per_sensor(
    macs: list[str] | None = None,
    db_url: str = DB_URL,
) -> dict[str, pd.Timestamp | None]:
    """Devuelve el timestamp UTC más reciente para cada MAC (None si no hay datos).
    Pensado para alimentar un indicador de "sensor vivo" — barato, no bloquea."""
    if macs is None:
        macs = [SENSOR_MAC_1, SENSOR_MAC_2]
    result: dict[str, pd.Timestamp | None] = {mac: None for mac in macs}

    if DB_PATH:
        conn = sqlite3.connect(f"file:{DB_PATH}?mode=ro", uri=True, timeout=5.0)
        try:
            placeholders = ",".join("?" * len(macs))
            rows = conn.execute(
                f"SELECT device_mac, MAX(received_at) FROM ble_samples "
                f"WHERE device_mac IN ({placeholders}) GROUP BY device_mac",
                macs,
            ).fetchall()
        finally:
            conn.close()
    else:
        import psycopg2

        conn = psycopg2.connect(db_url)
        try:
            cur = conn.cursor()
            cur.execute(
                "SELECT device_mac, MAX(received_at) FROM ble_samples "
                "WHERE device_mac = ANY(%s) GROUP BY device_mac",
                (macs,),
            )
            rows = cur.fetchall()
        finally:
            conn.close()

    for mac, last_seen in rows:
        if last_seen is not None:
            result[mac] = pd.to_datetime(last_seen, utc=True)
    return result
