"""Sincroniza ble_samples desde SQLite (Raspberry Pi) a PostgreSQL (cloud)."""
import argparse
import os
import sqlite3
from datetime import datetime, timezone

import psycopg2
from psycopg2.extras import execute_values

from logging_config import configure_logging, get_logger

log = get_logger(__name__)

DEFAULT_BATCH = 1000

# Timeout y PRAGMAs alineados con pi-service/db.rs y ml/pi_inference.py.
# El archivo SQLite se comparte por Docker volume con 3 contenedores; abrirlo
# sin busy_timeout cuelga inmediatamente ante SQLITE_BUSY (escrituras BLE
# concurrentes desde pi-service). Ver memoria del proyecto: "Pi SQLite
# concurrency - pi_data.db is shared by 3 containers; needs WAL + busy_timeout
# on every opener".
SQLITE_TIMEOUT_S = 5.0
SQLITE_BUSY_TIMEOUT_MS = 5000


def _connect_sqlite(path: str) -> sqlite3.Connection:
    """Abre SQLite con WAL + busy_timeout. Idempotente respecto a otros openers."""
    conn = sqlite3.connect(path, timeout=SQLITE_TIMEOUT_S)
    conn.execute("PRAGMA journal_mode=WAL")
    conn.execute(f"PRAGMA busy_timeout={SQLITE_BUSY_TIMEOUT_MS}")
    conn.execute("PRAGMA synchronous=NORMAL")
    return conn


def parse_sqlite_ts(ts: str) -> datetime:
    """Convierte el timestamp de SQLite (YYYY-MM-DD HH:MM:SS) a datetime UTC."""
    dt = datetime.strptime(ts, "%Y-%m-%d %H:%M:%S")
    return dt.replace(tzinfo=timezone.utc)


def sync(sqlite_path: str, pg_url: str, batch_size: int, dry_run: bool):
    # 1. Conectar a PostgreSQL y obtener el último received_at sincronizado
    pg_conn = psycopg2.connect(pg_url)
    pg_cur = pg_conn.cursor()
    pg_cur.execute("SELECT MAX(received_at) FROM ble_samples")
    row = pg_cur.fetchone()
    max_pg_ts = row[0] if row and row[0] else None
    log.info("postgres_last_received_at", value=str(max_pg_ts) if max_pg_ts else None)

    # 2. Conectar a SQLite (timeout + WAL) y leer muestras nuevas
    sqlite_conn = _connect_sqlite(sqlite_path)
    sqlite_cur = sqlite_conn.cursor()

    if max_pg_ts:
        # Comparar como texto (ISO-8601) funciona correctamente en SQLite
        sqlite_cur.execute(
            "SELECT id, device_mac, device_name, ble_ts, x, y, z, received_at "
            "FROM ble_samples WHERE received_at > ? ORDER BY received_at ASC",
            (max_pg_ts.strftime("%Y-%m-%d %H:%M:%S"),),
        )
    else:
        sqlite_cur.execute(
            "SELECT id, device_mac, device_name, ble_ts, x, y, z, received_at "
            "FROM ble_samples ORDER BY received_at ASC"
        )

    rows = sqlite_cur.fetchall()
    total = len(rows)
    log.info("sqlite_pending_samples", count=total)

    if total == 0:
        log.info("nothing_to_sync")
        return

    # Convertir timestamps a datetime UTC y preparar tuplas
    pg_rows = []
    for r in rows:
        _id, mac, name, ble_ts, x, y, z, received_at = r
        pg_rows.append(
            (
                _id,
                mac,
                name,
                ble_ts,
                float(x),
                float(y),
                float(z),
                parse_sqlite_ts(received_at),
            )
        )

    if dry_run:
        log.info("dry_run_summary", would_sync=total)
        return

    # 3. Insertar en PostgreSQL con ON CONFLICT DO NOTHING
    insert_sql = """
    INSERT INTO ble_samples (id, device_mac, device_name, ble_ts, x, y, z, received_at)
    VALUES %s
    ON CONFLICT DO NOTHING
    """

    synced = 0
    for i in range(0, len(pg_rows), batch_size):
        batch = pg_rows[i : i + batch_size]
        execute_values(pg_cur, insert_sql, batch, page_size=batch_size)
        pg_conn.commit()
        synced += len(batch)
        log.info("sync_progress", synced=synced, total=total)

    pg_cur.close()
    pg_conn.close()
    sqlite_conn.close()
    log.info("sync_complete", total_exported=synced)


def main():
    parser = argparse.ArgumentParser(
        description="Exporta ble_samples de la Raspberry Pi (SQLite) al PostgreSQL del cloud."
    )
    parser.add_argument(
        "--sqlite",
        default=os.getenv("DB_PATH", "pi_data.db"),
        help="Ruta a la base de datos SQLite local (default: DB_PATH o pi_data.db)",
    )
    parser.add_argument(
        "--pg-url",
        default=os.getenv(
            "DATABASE_URL", "postgres://knockshadow:knockshadow@127.0.0.1:5432/knockshadow"
        ),
        help="URL de conexión a PostgreSQL (default: DATABASE_URL)",
    )
    parser.add_argument(
        "--batch-size",
        type=int,
        default=DEFAULT_BATCH,
        help=f"Tamaño del lote de inserción (default: {DEFAULT_BATCH})",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Muestra cuántas filas se sincronizarían sin insertar",
    )
    args = parser.parse_args()

    configure_logging(service="sync_ble_to_cloud")
    sync(args.sqlite, args.pg_url, args.batch_size, args.dry_run)


if __name__ == "__main__":
    main()
