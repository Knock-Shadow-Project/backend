"""Janitor periódico para la base SQLite del Pi (`pi_data.db`).

Por qué existe:
    El Pi escribe en `ble_samples` ~50 muestras/s × 2 sensores. En unos días
    `pi_data.db` (+ WAL) supera fácilmente 1 GB y satura la microSD. Sólo se
    necesitan las últimas horas de muestras crudas: la inferencia local
    consume una ventana de segundos (ver `BUFFER_SECONDS` en
    `pi_inference.py`) y los punches ya detectados se persisten aparte en
    `detected_punches`. Este script borra periódicamente las muestras viejas
    y trunca el WAL para devolver espacio al filesystem.

Lo que NO hace:
    - No toca `detected_punches`, `local_trainings` ni `sync_queue`:
      contienen el estado lógico que la Pi necesita preservar para
      sincronizar con el cloud.
    - No corre `VACUUM` en estado estable: tiene que reescribir todo el
      archivo y requiere lock exclusivo (incompatible con pi-service como
      writer). Sólo lo lanza una vez al arrancar si `VACUUM_ON_START=1`,
      justo para reclamar el espacio acumulado de la primera ejecución.

Concurrencia (memoria del proyecto "Pi SQLite concurrency"):
    `pi_data.db` lo comparten 3 contenedores por Docker volume. Cualquier
    opener necesita WAL + busy_timeout, si no el primer conflicto de
    escritura ve `SQLITE_BUSY` y muere. Replicamos el setup de
    `crates/pi-service/src/db.rs` y `ml/pi_inference.py`.

Diseño operacional:
    Está pensado para correr como sidecar permanente en
    `docker-compose.pi.yaml` (`restart: unless-stopped`). El bucle hace una
    pasada al arrancar (para no esperar `SLEEP_SECONDS`) y luego cada
    `SLEEP_SECONDS`. SIGTERM se respeta entre iteraciones para que
    `docker compose down` no tenga que esperar al SIGKILL.
"""

from __future__ import annotations

import logging
import os
import signal
import sqlite3
import sys
import time
from types import FrameType
from typing import Optional

# Logging stdlib (no structlog) para que el script corra en `python:3.12-slim`
# sin instalar dependencias. El formato espeja el de los logs de Docker para
# que `docker logs` siga siendo legible junto con los del resto del stack.
log = logging.getLogger("pi_db_janitor")
logging.basicConfig(
    level=os.getenv("LOG_LEVEL", "INFO"),
    format="%(asctime)s %(levelname)s %(name)s %(message)s",
    stream=sys.stdout,
)

DB_PATH = os.getenv("DB_PATH", "/data/pi_data.db")
RETENTION_HOURS = float(os.getenv("RETENTION_HOURS", "1"))
SLEEP_SECONDS = int(os.getenv("SLEEP_SECONDS", "300"))
VACUUM_ON_START = os.getenv("VACUUM_ON_START", "0").lower() in ("1", "true", "yes")
# Tamaño del chunk de DELETE. Probado en la Pi: 100k filas → ~2 s. Esto
# limita el tiempo que mantenemos el "writer lock" de SQLite y deja
# espacio entre chunks para que pi-service no entre en SQLITE_BUSY. Un
# valor mayor reduce overhead pero alarga cada transacción; este punto
# medio funcionó bien en el primer despliegue (14M filas borradas en ~5 min).
DELETE_CHUNK_ROWS = int(os.getenv("DELETE_CHUNK_ROWS", "100000"))
# Tope de filas a borrar en una sola pasada. Sin esto, el primer despliegue
# (con millones de filas atrasadas) podría correr 5+ minutos seguidos
# escribiendo al WAL. El default es generoso: borra hasta ~2M filas por
# pasada (~40 s a 50k/s); el resto se va en pasadas siguientes (cada
# SLEEP_SECONDS) hasta drenarlo. En estado estable nunca se alcanza.
MAX_ROWS_PER_PASS = int(os.getenv("MAX_ROWS_PER_PASS", "2000000"))
# 30 s de busy_timeout: en estado estable pi-service inserta a bursts; si
# justo coincide con nuestro DELETE, queremos esperar, no rebotar.
SQLITE_BUSY_TIMEOUT_MS = 30_000

# Tablas que se envejecen por su columna timestamp. `ble_samples` es la única
# con volumen problemático (~1.4 M filas/día). El resto se conservan porque
# son críticas para la lógica de sincronización con el cloud.
AGE_OUT_TABLES: tuple[tuple[str, str], ...] = (
    ("ble_samples", "received_at"),
)


_stop = False


def _on_signal(signum: int, _frame: Optional[FrameType]) -> None:
    """Handler de SIGTERM/SIGINT.

    Sólo levanta el flag; el bucle principal lo lee entre iteraciones para
    evitar interrumpir un `DELETE` a la mitad (lo cual no rompería la DB
    gracias a la transacción, pero genera ruido en los logs).
    """
    global _stop
    _stop = True
    log.info("signal_received signum=%s will_exit_between_iterations", signum)


signal.signal(signal.SIGTERM, _on_signal)
signal.signal(signal.SIGINT, _on_signal)


def _connect() -> sqlite3.Connection:
    """Abre la DB con los PRAGMA requeridos por el modo compartido."""
    conn = sqlite3.connect(DB_PATH, timeout=30.0)
    # WAL es persistente a nivel de archivo (se mantiene entre conexiones);
    # `busy_timeout` y `synchronous` son por conexión. Aplicarlos aquí es
    # idempotente y nos protege si el janitor abre antes que pi-service.
    conn.execute("PRAGMA journal_mode=WAL")
    conn.execute(f"PRAGMA busy_timeout={SQLITE_BUSY_TIMEOUT_MS}")
    conn.execute("PRAGMA synchronous=NORMAL")
    return conn


def _checkpoint_wal(conn: sqlite3.Connection, where: str) -> None:
    """Trunca el WAL (lo reduce a 0 bytes) y aplica páginas al archivo principal.

    Lo extraemos a función porque lo llamamos dos veces: entre chunks de un
    borrado masivo (para que el WAL no se infle a varios GB en una sola
    pasada) y al final de cada `_cleanup_once`.
    """
    busy, log_pages, checkpointed = conn.execute(
        "PRAGMA wal_checkpoint(TRUNCATE)"
    ).fetchone()
    log.info(
        "wal_checkpoint where=%s busy=%s log_pages=%s checkpointed=%s",
        where,
        busy,
        log_pages,
        checkpointed,
    )


def _delete_old_chunked(
    conn: sqlite3.Connection, table: str, ts_col: str, cutoff_sql: str
) -> int:
    """Borra filas viejas en chunks de `DELETE_CHUNK_ROWS`.

    Por qué no un sólo `DELETE`: con 14M+ filas atrasadas (caso del primer
    despliegue) un único `DELETE` mantiene la transacción abierta varios
    minutos. Mientras tanto el WAL crece sin poder hacer checkpoint, y
    cualquier `INSERT` de pi-service se queda esperando el writer lock.
    Con chunks de 100k:
        - cada transacción dura ~2 s
        - hacemos checkpoint entre chunks para liberar espacio al FS
        - pi-service puede meter sus INSERTs entre nuestros chunks
    Retorna el total de filas borradas en esta pasada (≤ MAX_ROWS_PER_PASS).
    """
    cur = conn.cursor()
    total_deleted = 0
    while total_deleted < MAX_ROWS_PER_PASS and not _stop:
        t0 = time.monotonic()
        # Subquery con `LIMIT` sobre la PK numérica: el plan usa
        # `idx_ble_samples_time` para el filtro temporal y devuelve hasta
        # `DELETE_CHUNK_ROWS` IDs; luego el DELETE va por PK (rapidísimo).
        cur.execute(
            f"DELETE FROM {table} WHERE id IN ("
            f"  SELECT id FROM {table} WHERE {ts_col} < {cutoff_sql} "
            f"  LIMIT {DELETE_CHUNK_ROWS}"
            f")"
        )
        deleted = cur.rowcount
        conn.commit()
        if deleted == 0:
            # Ya no quedan filas viejas; salimos del bucle.
            break
        total_deleted += deleted
        log.info(
            "chunk_deleted table=%s rows=%d total=%d elapsed_s=%.2f",
            table,
            deleted,
            total_deleted,
            time.monotonic() - t0,
        )
        # Checkpoint cada N chunks para que el WAL no se infle durante una
        # purga grande. Sin esto, borrar 14M filas hace que el WAL llegue a
        # varios GB y consuma toda la microSD antes de poder reclamar nada.
        # Cada 10 chunks ≈ 1M filas ≈ ~20 s de trabajo → cadencia razonable.
        if (total_deleted // DELETE_CHUNK_ROWS) % 10 == 0:
            _checkpoint_wal(conn, where=f"mid_purge_{table}")
    return total_deleted


def _cleanup_once(conn: sqlite3.Connection) -> None:
    """Una pasada de borrado + checkpoint del WAL."""
    cur = conn.cursor()
    # `datetime('now', '-N hours')` devuelve el cutoff como string UTC en el
    # mismo formato que escribe pi-service (`%Y-%m-%d %H:%M:%f`). La
    # comparación es lexicográfica pero correcta porque ambos usan ISO-8601.
    cutoff_sql = f"datetime('now', '-{RETENTION_HOURS} hours')"

    for table, ts_col in AGE_OUT_TABLES:
        # Conteo previo: barato con `idx_ble_samples_time`; útil para los
        # logs y para detectar tablas vacías sin abrir transacción.
        t0 = time.monotonic()
        n = cur.execute(
            f"SELECT COUNT(*) FROM {table} WHERE {ts_col} < {cutoff_sql}"
        ).fetchone()[0]
        log.info(
            "cleanup_count table=%s old_rows=%d elapsed_s=%.2f",
            table,
            n,
            time.monotonic() - t0,
        )
        if n == 0:
            continue
        deleted = _delete_old_chunked(conn, table, ts_col, cutoff_sql)
        log.info(
            "cleanup_deleted table=%s deleted_this_pass=%d remaining_after_pass≈%d",
            table,
            deleted,
            max(0, n - deleted),
        )

    _checkpoint_wal(conn, where="end_of_pass")


def _vacuum(conn: sqlite3.Connection) -> None:
    """Compacta el archivo principal. Sólo se llama al arrancar.

    `VACUUM` reescribe la DB entera y necesita lock exclusivo + ~2x el tamaño
    del archivo en espacio temporal. En estado estable no lo queremos: ya
    bastan los `DELETE` + `wal_checkpoint(TRUNCATE)`. Pero en el primer
    arranque, tras el `DELETE` masivo inicial, las páginas liberadas siguen
    asignadas al archivo (auto_vacuum=0); VACUUM las libera al FS.
    """
    log.info("vacuum_started")
    t0 = time.monotonic()
    # `VACUUM` no puede correr dentro de una transacción. El driver sqlite3
    # de Python abre una transacción implícita en cualquier statement; hay
    # que pasar a autocommit explícitamente.
    prev_isolation = conn.isolation_level
    conn.isolation_level = None
    try:
        conn.execute("VACUUM")
    finally:
        conn.isolation_level = prev_isolation
    log.info("vacuum_finished elapsed_s=%.1f", time.monotonic() - t0)


def main() -> int:
    log.info(
        "janitor_starting db_path=%s retention_hours=%.2f sleep_seconds=%d vacuum_on_start=%s",
        DB_PATH,
        RETENTION_HOURS,
        SLEEP_SECONDS,
        VACUUM_ON_START,
    )
    if not os.path.exists(DB_PATH):
        # No crashea: si el volumen no está montado o pi-service aún no ha
        # creado el archivo, esperamos y reintentamos. Es preferible a
        # entrar en CrashLoopBackOff.
        log.warning("db_missing_will_wait path=%s", DB_PATH)
        while not _stop and not os.path.exists(DB_PATH):
            time.sleep(5)
        if _stop:
            return 0

    # Pasada inicial inmediata: si la DB está llena al desplegar el janitor,
    # el operador no debería esperar `SLEEP_SECONDS` (300 s por defecto) a
    # ver actividad.
    try:
        conn = _connect()
        _cleanup_once(conn)
        if VACUUM_ON_START:
            _vacuum(conn)
        conn.close()
    except sqlite3.Error:
        # `exception` incluye traceback; el siguiente tick reintenta.
        log.exception("startup_cleanup_failed")

    while not _stop:
        # Sleep en rebanadas de 1 s para que SIGTERM se sienta inmediato.
        for _ in range(SLEEP_SECONDS):
            if _stop:
                break
            time.sleep(1)
        if _stop:
            break
        try:
            conn = _connect()
            _cleanup_once(conn)
            conn.close()
        except sqlite3.Error:
            log.exception("cleanup_failed")

    log.info("janitor_stopped")
    return 0


if __name__ == "__main__":
    sys.exit(main())
