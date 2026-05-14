"""Sincroniza ble_samples desde SQLite (Raspberry Pi) a PostgreSQL (cloud)."""
import argparse
import os
import sqlite3
from datetime import datetime, timezone

import psycopg2
from psycopg2.extras import execute_values

DEFAULT_BATCH = 1000


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
    print(f"Último received_at en PostgreSQL: {max_pg_ts}")

    # 2. Conectar a SQLite y leer muestras nuevas
    sqlite_conn = sqlite3.connect(sqlite_path)
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
    print(f"Muestras pendientes en SQLite: {total}")

    if total == 0:
        print("Nada que sincronizar.")
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
        print(f"[DRY-RUN] Se sincronizarían {total} filas.")
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
        print(f"  … sincronizadas {synced}/{total}")

    pg_cur.close()
    pg_conn.close()
    sqlite_conn.close()
    print(f"✅ Sincronización completa: {synced} muestras exportadas.")


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

    sync(args.sqlite, args.pg_url, args.batch_size, args.dry_run)


if __name__ == "__main__":
    main()
