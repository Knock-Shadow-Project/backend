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

from pipeline import (
    FEATURE_COLS,
    HIT_THRESHOLD_G,
    WINDOW_SIZE,
    create_windows,
    detect_hits,
    merge_sensors,
)
from train import PunchCNN

MODEL_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "model")
MODEL_PATH = os.path.join(MODEL_DIR, "punch_classifier.pt")
CLASS_NAMES_PATH = os.path.join(MODEL_DIR, "class_names.npy")
NORM_MEAN_PATH = os.path.join(MODEL_DIR, "norm_mean.npy")
NORM_STD_PATH = os.path.join(MODEL_DIR, "norm_std.npy")

DEVICE = torch.device("cuda" if torch.cuda.is_available() else "cpu")

POLL_INTERVAL = 1.0
BUFFER_SECONDS = 5.0
PROCESSED_TTL = 10.0

DB_PATH = os.getenv("DB_PATH", "pi_data.db")
SENSOR_MAC_1 = os.getenv("SENSOR_MAC_1", "DF:65:81:D0:D7:E5")
SENSOR_MAC_2 = os.getenv("SENSOR_MAC_2", "CB:01:10:3E:0D:61")


def load_model():
    if not os.path.exists(MODEL_PATH):
        print(f"ERROR: No se encontro el modelo en {MODEL_PATH}")
        sys.exit(1)

    checkpoint = torch.load(MODEL_PATH, map_location=DEVICE)
    num_classes = checkpoint["num_classes"]
    in_channels = checkpoint.get("in_channels", len(FEATURE_COLS))

    model = PunchCNN(num_classes=num_classes, in_channels=in_channels)
    model.load_state_dict(checkpoint["model_state"])
    model.to(DEVICE)
    model.eval()

    class_names = np.load(CLASS_NAMES_PATH, allow_pickle=True)
    mean = np.load(NORM_MEAN_PATH)
    std = np.load(NORM_STD_PATH)

    print(f"Modelo cargado desde {MODEL_PATH}")
    print(f"Dispositivo: {DEVICE}")
    print(f"Clases: {class_names.tolist()}")
    return model, class_names, mean, std


def predict_windows(model, windows, class_names, mean, std):
    if len(windows) == 0:
        return []

    X_norm = (windows - mean) / std
    X_t = torch.from_numpy(X_norm).float().to(DEVICE)

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
    conn = sqlite3.connect(DB_PATH)
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
    conn = sqlite3.connect(DB_PATH)
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
    conn = sqlite3.connect(DB_PATH)
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
        print(f"Inferencia continua iniciada (Ctrl+C para detener)")
        print(f"Intervalo de lectura SQLite: {POLL_INTERVAL}s")
        print(f"Buffer en memoria: {BUFFER_SECONDS}s")
        print(f"Umbral de deteccion: {HIT_THRESHOLD_G} G")

        # Autodescubrir entrenamiento activo si no se proporciono
        if self.local_training_id is None:
            tid, uid = get_active_training()
            if tid is not None:
                self.local_training_id = tid
                self.user_id = uid
                print(f"Entrenamiento activo detectado: id={tid}, user={uid}")

        print(f"Usuario: {self.user_id} | Entrenamiento local: {self.local_training_id}")
        print()

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
            except Exception as e:
                print(f"  ⚠️ Error SQLite: {e}")
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
                    print(f"[Auto-detect] Entrenamiento activo: id={tid}, user={uid}")

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

                print(
                    f"  🥊 @ {time_str} → {top['clase']} ({top['prob'] * 100:.1f}%)"
                    f"  | Potencia: {potencia}G"
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
                    except Exception as e:
                        print(f"  ⚠️ Error guardando en SQLite: {e}")

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
        print("\nInferencia detenida.")


if __name__ == "__main__":
    main()
