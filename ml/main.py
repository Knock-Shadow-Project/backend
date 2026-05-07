import argparse
import asyncio
import datetime
import os
import sys
from collections import deque

import numpy as np
import pandas as pd
import torch

from api_client import ApiClient
from pipeline import (
    DB_URL,
    FEATURE_COLS,
    HIT_THRESHOLD_G,
    WINDOW_SIZE,
    create_windows,
    detect_hits,
    load_data,
    merge_sensors,
)
from train import PunchCNN

MODEL_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "model")
MODEL_PATH = os.path.join(MODEL_DIR, "punch_classifier.pt")
CLASS_NAMES_PATH = os.path.join(MODEL_DIR, "class_names.npy")
NORM_MEAN_PATH = os.path.join(MODEL_DIR, "norm_mean.npy")
NORM_STD_PATH = os.path.join(MODEL_DIR, "norm_std.npy")

DEVICE = torch.device("cuda" if torch.cuda.is_available() else "cpu")

# Configuración del buffer circular
POLL_INTERVAL = 1.0          # segundos entre lecturas de BD
BUFFER_SECONDS = 5.0         # segundos de datos acumulados en memoria
PROCESSED_TTL = 10.0         # segundos para mantener timestamps procesados


def load_model():
    """Carga el modelo entrenado y los metadatos asociados."""
    if not os.path.exists(MODEL_PATH):
        print(f"ERROR: No se encontró el modelo en {MODEL_PATH}")
        print("Entrena primero con: python train.py")
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
    """Normaliza ventanas y devuelve predicciones con probabilidades."""
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
            {
                "clase": str(class_names[idx]),
                "prob": float(prob),
            }
            for idx, prob in zip(
                top_indices[i].cpu().numpy(), top_probs[i].cpu().numpy()
            )
        ]
        results.append(preds)

    return results


class AsyncInferenceEngine:
    """Motor de inferencia continua con buffer circular y procesamiento asíncrono."""

    def __init__(
        self,
        model,
        class_names,
        mean,
        std,
        use_api=False,
        use_ws=False,
        api_user_id=1,
    ):
        self.model = model
        self.class_names = class_names
        self.mean = mean
        self.std = std
        self.use_api = use_api
        self.use_ws = use_ws
        self.api_user_id = api_user_id

        self._buffer = deque()
        self._df = pd.DataFrame()
        self._processed_peaks: set[pd.Timestamp] = set()
        self._last_poll_end = datetime.datetime.now(datetime.timezone.utc)

        self.api: ApiClient | None = None
        self.id_entrenamiento: int | None = None

    async def setup(self):
        """Inicializa conexiones a API y WebSocket."""
        if self.use_api or self.use_ws:
            self.api = ApiClient()
            self.api.fetch_golpes()
            if self.use_api:
                ent = self.api.create_entrenamiento(
                    id_usuario=self.api_user_id, tipo="Estandar"
                )
                self.id_entrenamiento = ent["id_entrenamiento"]
                print(f"Entrenamiento creado: ID={self.id_entrenamiento}")
            if self.use_ws:
                await self.api.connect_ws()
                print("WebSocket conectado")
            print()

    async def run(self):
        """Arranca el productor y el consumidor concurrentemente."""
        print(f"Inferencia continua iniciada (Ctrl+C para detener)")
        print(f"Intervalo de lectura BD: {POLL_INTERVAL}s")
        print(f"Buffer en memoria: {BUFFER_SECONDS}s")
        print(f"Umbral de detección: {HIT_THRESHOLD_G} G")
        if self.use_api:
            print("Modo API: activado")
        if self.use_ws:
            print("Modo WebSocket: activado")
        print()

        producer = asyncio.create_task(self._producer())
        consumer = asyncio.create_task(self._consumer())

        try:
            await asyncio.gather(producer, consumer)
        except asyncio.CancelledError:
            pass
        finally:
            await self._shutdown()

    async def _producer(self):
        """Lee de PostgreSQL cada 1 segundo y alimenta el buffer."""
        self._last_poll_end = datetime.datetime.now(datetime.timezone.utc)
        while True:
            await asyncio.sleep(POLL_INTERVAL)

            t_start = self._last_poll_end
            t_end = datetime.datetime.now(datetime.timezone.utc)

            try:
                raw = await asyncio.to_thread(load_data, t_start, t_end, db_url=DB_URL)
            except Exception as e:
                print(f"  ⚠️ Error de BD: {e}")
                continue

            if not raw.empty:
                self._buffer.append(raw)

            self._last_poll_end = t_end

    async def _consumer(self):
        """Procesa continuamente el buffer acumulado."""
        while True:
            await asyncio.sleep(0.1)

            # Mover nuevos DataFrames del buffer al acumulado
            new_dfs = []
            while self._buffer:
                new_dfs.append(self._buffer.popleft())

            if new_dfs:
                self._df = pd.concat([self._df] + new_dfs, ignore_index=True)
                # Eliminar duplicados exactos
                self._df = self._df.drop_duplicates(
                    subset=["received_at", "device_mac", "x", "y", "z"]
                )

            # Recortar datos antiguos
            cutoff = datetime.datetime.now(datetime.timezone.utc) - datetime.timedelta(
                seconds=BUFFER_SECONDS
            )
            if not self._df.empty and "received_at" in self._df.columns:
                self._df = self._df[self._df["received_at"] > cutoff]

            if self._df.empty:
                continue

            # Fusionar sensores, detectar picos y clasificar
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

                # Deduplicación: evitar clasificar el mismo golpe dos veces
                if peak_ts in self._processed_peaks:
                    continue

                # Potencia = magnitud pico en G
                potencia = None
                if "mag" in merged.columns:
                    potencia = round(float(merged["mag"].iloc[peak_idx]), 2)

                top = preds[0]
                time_str = (
                    peak_ts.strftime("%H:%M:%S.%f")[:-3]
                    if hasattr(peak_ts, "strftime")
                    else str(peak_ts)
                )

                print(
                    f"  🥊 @ {time_str} → {top['clase']} ({top['prob'] * 100:.1f}%)"
                    f"  | Potencia: {potencia}G"
                )

                # Enviar por WebSocket
                if self.api and self.use_ws:
                    msg = {
                        "type": "punch_detected",
                        "timestamp": time_str,
                        "predictions": preds,
                        "potencia": potencia,
                    }
                    try:
                        await self.api.send_ws(msg)
                    except Exception as e:
                        print(f"  ⚠️ Error WS: {e}")

                # Subir a API REST
                if self.api and self.id_entrenamiento is not None:
                    id_golpe = self.api.map_prediction_to_golpe(top["clase"])
                    if id_golpe is not None:
                        try:
                            self.api.create_historial(
                                self.id_entrenamiento, id_golpe, potencia
                            )
                        except Exception as e:
                            print(f"  ⚠️ Error API: {e}")

                self._processed_peaks.add(peak_ts)

            # Limpiar timestamps procesados antiguos para evitar memory leak
            cutoff_processed = now - datetime.timedelta(seconds=PROCESSED_TTL)
            self._processed_peaks = {
                ts for ts in self._processed_peaks if ts > cutoff_processed
            }

    async def _shutdown(self):
        """Finaliza el entrenamiento y cierra conexiones."""
        if self.api and self.id_entrenamiento is not None:
            try:
                self.api.finish_entrenamiento(self.id_entrenamiento)
                print(f"Entrenamiento {self.id_entrenamiento} finalizado.")
            except Exception as e:
                print(f"Error finalizando entrenamiento: {e}")
        if self.api and self.use_ws:
            try:
                await self.api.close_ws()
            except Exception:
                pass


def main():
    parser = argparse.ArgumentParser(
        description="Inferencia continua del clasificador de golpes"
    )
    parser.add_argument(
        "--api",
        action="store_true",
        help="Sube los resultados a la API REST (crea entrenamiento + historial)",
    )
    parser.add_argument(
        "--ws",
        action="store_true",
        help="Strea los resultados en tiempo real por WebSocket",
    )
    parser.add_argument(
        "--api-user-id",
        type=int,
        default=1,
        help="ID de usuario para crear el entrenamiento (default: 1)",
    )
    args = parser.parse_args()

    model, class_names, mean, std = load_model()

    engine = AsyncInferenceEngine(
        model,
        class_names,
        mean,
        std,
        use_api=args.api,
        use_ws=args.ws,
        api_user_id=args.api_user_id,
    )

    try:
        asyncio.run(engine.run())
    except KeyboardInterrupt:
        print("\nInferencia detenida.")


if __name__ == "__main__":
    main()
