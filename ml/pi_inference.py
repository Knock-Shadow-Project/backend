import argparse
import asyncio
import datetime
import os
import sqlite3
import sys
from collections import deque

import numpy as np
import pandas as pd
import torch

from logging_config import configure_logging, get_logger
from model_loader import ModelNotFoundError, load_model as _load_model
from pipeline import (
    HIT_THRESHOLD_G,
    create_windows,
    detect_hits,
    merge_sensors,
)

log = get_logger(__name__)

POLL_INTERVAL = 1.0
BUFFER_SECONDS = 5.0
PROCESSED_TTL = 10.0

DB_PATH = os.getenv("DB_PATH", "pi_data.db")
SENSOR_MAC_1 = os.getenv("SENSOR_MAC_1", "DF:65:81:D0:D7:E5")
SENSOR_MAC_2 = os.getenv("SENSOR_MAC_2", "CB:01:10:3E:0D:61")

# Shared with pi-service (Rust) via Docker volume. pi-service normally initializes
# the file with journal_mode=WAL, pero NO podemos depender de eso: si pi-inference
# arranca antes que pi-service (orden de docker-compose, reboot) o si pi-service
# falla al inicializar, el archivo queda en modo rollback-journal y los writers
# concurrentes se serializan inmediatamente.
# Por eso configuramos WAL + busy_timeout localmente también: las PRAGMA son
# persistentes a nivel de archivo (WAL) o de conexión (busy_timeout), aplicarlas
# desde aquí es idempotente.
SQLITE_TIMEOUT = 5.0
SQLITE_BUSY_TIMEOUT_MS = 5000


def _connect_sqlite() -> sqlite3.Connection:
    conn = sqlite3.connect(DB_PATH, timeout=SQLITE_TIMEOUT)
    # journal_mode=WAL es persistente: una vez aplicado al archivo se mantiene
    # entre conexiones. Forzarlo aquí garantiza la propiedad incluso si
    # pi-inference se conecta antes que pi-service.
    conn.execute("PRAGMA journal_mode=WAL")
    conn.execute(f"PRAGMA busy_timeout={SQLITE_BUSY_TIMEOUT_MS}")
    # synchronous=NORMAL es el recomendado con WAL: durabilidad similar a FULL
    # pero sin coste extra de fsync en cada commit. Igual setup que pi-service/db.rs.
    conn.execute("PRAGMA synchronous=NORMAL")
    return conn


def load_model():
    """Wrapper local de `model_loader.load_model` con logging del proyecto.

    Mantiene la firma legacy `(model, class_names, mean, std)` esperada por el
    AsyncInferenceEngine. El loader compartido devuelve un `LoadedModel`
    (dataclass); aquí lo desempaquetamos para no romper el resto del archivo.
    """
    try:
        bundle = _load_model()
    except ModelNotFoundError as e:
        log.error("model_not_found", error=str(e))
        sys.exit(1)

    log.info(
        "model_loaded",
        device=str(bundle.device),
        classes=bundle.class_names.tolist(),
        num_classes=bundle.num_classes,
    )
    return bundle.model, bundle.class_names, bundle.mean, bundle.std


def predict_windows(model, windows, class_names, mean, std):
    if len(windows) == 0:
        return []

    device = next(model.parameters()).device
    X_norm = (windows - mean) / std
    X_t = torch.from_numpy(X_norm).float().to(device)

    with torch.no_grad():
        logits = model(X_t)
        probs = torch.softmax(logits, dim=1)
        top_probs, top_indices = torch.topk(probs, k=min(3, len(class_names)), dim=1)

    results = []
    for i in range(len(windows)):
        preds = [
            {"clase": str(class_names[idx]), "prob": float(prob)}
            for idx, prob in zip(top_indices[i].cpu().numpy(), top_probs[i].cpu().numpy())
        ]
        results.append(preds)

    return results


def load_data_sqlite(start_time, end_time):
    conn = _connect_sqlite()
    query = """
    SELECT received_at, device_mac, x, y, z
    FROM ble_samples
    WHERE received_at BETWEEN ? AND ?
      AND device_mac IN (?, ?)
    ORDER BY received_at ASC;
    """
    df = pd.read_sql_query(
        query, conn, params=(start_time, end_time, SENSOR_MAC_1, SENSOR_MAC_2)
    )
    conn.close()
    df["received_at"] = pd.to_datetime(df["received_at"], utc=True)
    return df


def save_detected_punch(user_id, local_training_id, class_name, limb, position, power, prob):
    conn = _connect_sqlite()
    cursor = conn.cursor()
    cursor.execute(
        """
        INSERT INTO detected_punches
        (user_id, local_training_id, class_name, limb, position, power, prob)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        """,
        (user_id, local_training_id, class_name, limb, position, power, prob),
    )
    conn.commit()
    conn.close()


def get_active_training():
    """Busca el entrenamiento activo (sin end_time) mas reciente en SQLite."""
    conn = _connect_sqlite()
    cursor = conn.cursor()
    cursor.execute(
        """
        SELECT id, user_id FROM local_trainings
        WHERE end_time IS NULL
        ORDER BY start_time DESC
        LIMIT 1
        """
    )
    row = cursor.fetchone()
    conn.close()
    if row:
        return int(row[0]), int(row[1])
    return None, None


def parse_label(label: str) -> tuple[str, str, str]:
    parts = label.lower().split("_")
    if len(parts) < 2:
        return label.capitalize(), "Derecha", "Cabeza"

    punch_type = parts[0]
    if punch_type == "jab":
        name = "Jab"
    elif punch_type == "cross":
        name = "Cross"
    elif punch_type == "hook":
        name = "Gancho"
    elif punch_type == "uppercut":
        name = "Upper"
    else:
        name = punch_type.capitalize()

    pos_str = "_".join(parts[1:])
    if "izquierda" in pos_str:
        limb = "Izquierda"
    elif "derecha" in pos_str:
        limb = "Derecha"
    else:
        limb = "Derecha"

    if "arriba" in pos_str:
        position = "Cabeza"
    else:
        position = "Cuerpo"

    return name, limb, position


class AsyncInferenceEngine:
    def __init__(
        self,
        model,
        class_names,
        mean,
        std,
        user_id: int = 1,
        local_training_id: int | None = None,
    ):
        self.model = model
        self.class_names = class_names
        self.mean = mean
        self.std = std
        self.user_id = user_id
        self.local_training_id = local_training_id

        self._buffer = deque()
        self._df = pd.DataFrame()
        self._processed_peaks: set[pd.Timestamp] = set()
        self._last_poll_end = datetime.datetime.now(datetime.timezone.utc)

    async def run(self):
        log.info(
            "inference_started",
            poll_interval_s=POLL_INTERVAL,
            buffer_seconds=BUFFER_SECONDS,
            hit_threshold_g=HIT_THRESHOLD_G,
        )

        # Autodescubrir entrenamiento activo si no se proporciono
        if self.local_training_id is None:
            tid, uid = get_active_training()
            if tid is not None:
                self.local_training_id = tid
                self.user_id = uid
                log.info("active_training_detected", training_id=tid, user_id=uid)

        log.info(
            "inference_context",
            user_id=self.user_id,
            local_training_id=self.local_training_id,
        )

        producer = asyncio.create_task(self._producer())
        consumer = asyncio.create_task(self._consumer())

        try:
            await asyncio.gather(producer, consumer)
        except asyncio.CancelledError:
            pass

    async def _producer(self):
        self._last_poll_end = datetime.datetime.now(datetime.timezone.utc)
        while True:
            await asyncio.sleep(POLL_INTERVAL)

            t_start = self._last_poll_end
            t_end = datetime.datetime.now(datetime.timezone.utc)

            try:
                raw = await asyncio.to_thread(load_data_sqlite, t_start, t_end)
            except sqlite3.Error as e:
                log.warning("sqlite_read_failed", error=str(e), exc_info=True)
                continue

            if not raw.empty:
                self._buffer.append(raw)

            self._last_poll_end = t_end

    async def _consumer(self):
        active_check_tick = 0
        while True:
            await asyncio.sleep(0.1)
            active_check_tick += 1

            # Re-verificar entrenamiento activo cada ~10 segundos si no tenemos uno
            if self.local_training_id is None and active_check_tick % 100 == 0:
                tid, uid = get_active_training()
                if tid is not None:
                    self.local_training_id = tid
                    self.user_id = uid
                    log.info(
                        "active_training_autodetected",
                        training_id=tid,
                        user_id=uid,
                    )

            new_dfs = []
            while self._buffer:
                new_dfs.append(self._buffer.popleft())

            if new_dfs:
                self._df = pd.concat([self._df] + new_dfs, ignore_index=True)
                self._df = self._df.drop_duplicates(
                    subset=["received_at", "device_mac", "x", "y", "z"]
                )

            cutoff = datetime.datetime.now(datetime.timezone.utc) - datetime.timedelta(
                seconds=BUFFER_SECONDS
            )
            if not self._df.empty and "received_at" in self._df.columns:
                self._df = self._df[self._df["received_at"] > cutoff]

            if self._df.empty:
                continue

            merged = merge_sensors(self._df)
            if merged.empty:
                continue

            peaks = detect_hits(merged, threshold=HIT_THRESHOLD_G)
            if len(peaks) == 0:
                continue

            windows, valid_peaks = create_windows(
                merged, peaks, return_valid_peaks=True
            )
            if len(windows) == 0:
                continue

            predictions = predict_windows(
                self.model, windows, self.class_names, self.mean, self.std
            )

            now = datetime.datetime.now(datetime.timezone.utc)

            for i, preds in enumerate(predictions):
                peak_idx = int(valid_peaks[i])
                if peak_idx >= len(merged):
                    continue

                peak_ts = merged["received_at"].iloc[peak_idx]

                if peak_ts in self._processed_peaks:
                    continue

                potencia = None
                if "mag" in merged.columns:
                    potencia = round(float(merged["mag"].iloc[peak_idx]), 2)

                top = preds[0]
                time_str = (
                    peak_ts.strftime("%H:%M:%S.%f")[:-3]
                    if hasattr(peak_ts, "strftime")
                    else str(peak_ts)
                )

                name, limb, position = parse_label(top["clase"])

                log.info(
                    "punch_detected",
                    timestamp=time_str,
                    class_name=top["clase"],
                    name=name,
                    limb=limb,
                    position=position,
                    probability=round(float(top["prob"]), 4),
                    power_g=potencia,
                )

                if self.local_training_id is not None:
                    try:
                        save_detected_punch(
                            self.user_id,
                            self.local_training_id,
                            name,
                            limb,
                            position,
                            potencia,
                            round(float(top["prob"]), 4),
                        )
                    except sqlite3.Error as e:
                        log.warning(
                            "sqlite_save_failed",
                            error=str(e),
                            training_id=self.local_training_id,
                            exc_info=True,
                        )

                self._processed_peaks.add(peak_ts)

            cutoff_processed = now - datetime.timedelta(seconds=PROCESSED_TTL)
            self._processed_peaks = {
                ts for ts in self._processed_peaks if ts > cutoff_processed
            }


def main():
    parser = argparse.ArgumentParser(
        description="Inferencia continua del clasificador de golpes (modo Raspberry Pi offline)"
    )
    parser.add_argument(
        "--user-id",
        type=int,
        default=1,
        help="ID de usuario para asociar los golpes detectados",
    )
    parser.add_argument(
        "--training-id",
        type=int,
        default=None,
        help="ID local de entrenamiento (se obtiene de pi-service via /training/start)",
    )
    args = parser.parse_args()

    configure_logging(service="pi_inference")

    model, class_names, mean, std = load_model()

    engine = AsyncInferenceEngine(
        model,
        class_names,
        mean,
        std,
        user_id=args.user_id,
        local_training_id=args.training_id,
    )

    try:
        asyncio.run(engine.run())
    except KeyboardInterrupt:
        log.info("inference_stopped_by_user")


if __name__ == "__main__":
    main()
