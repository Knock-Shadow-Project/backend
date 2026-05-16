"""Smoke tests para la configuración SQLite compartida en Python.

Verifica que las funciones de conexión aplican WAL + busy_timeout, evitando
regresiones contra la nota de memoria del proyecto: *Pi SQLite concurrency —
pi_data.db is shared by 3 containers; needs WAL + busy_timeout on every
opener*.
"""

from __future__ import annotations

import sqlite3
from pathlib import Path

import pytest

import sync_ble_to_cloud


@pytest.fixture
def temp_db(tmp_path: Path) -> Path:
    """Archivo SQLite limpio para cada test (aislado por tmp_path)."""
    return tmp_path / "test.db"


def test_sync_ble_to_cloud_connect_enables_wal_and_busy_timeout(temp_db: Path):
    """`_connect_sqlite` debe dejar el archivo en journal_mode=WAL."""
    conn = sync_ble_to_cloud._connect_sqlite(str(temp_db))
    try:
        journal_mode = conn.execute("PRAGMA journal_mode").fetchone()[0]
        busy_timeout = conn.execute("PRAGMA busy_timeout").fetchone()[0]
        synchronous = conn.execute("PRAGMA synchronous").fetchone()[0]
    finally:
        conn.close()

    assert journal_mode.lower() == "wal"
    # busy_timeout se expresa en ms y debe ser >= 5000.
    assert busy_timeout >= 5000
    # synchronous=NORMAL = 1. Comprobamos que no sea 0 (OFF) ni FULL (2) por error.
    assert synchronous == 1


def test_wal_mode_persists_across_connections(temp_db: Path):
    """Una vez puesto en WAL, una nueva conexión sin PRAGMAs sigue viendo WAL.

    Esto es relevante porque demuestra que cuando pi-service o pi-inference
    inicializan el archivo, otros openers que no fuercen el PRAGMA siguen
    obteniendo el modo correcto — pero seguimos forzándolo para defenderse
    de la orden de arranque.
    """
    conn = sync_ble_to_cloud._connect_sqlite(str(temp_db))
    conn.close()

    # Abrir como conexión pelada (sin nuestros PRAGMAs).
    raw = sqlite3.connect(str(temp_db))
    try:
        mode = raw.execute("PRAGMA journal_mode").fetchone()[0]
    finally:
        raw.close()
    assert mode.lower() == "wal"
