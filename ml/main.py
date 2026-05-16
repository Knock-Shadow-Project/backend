import argparse
import asyncio
import datetime
import sys
from collections import deque

import pandas as pd
import torch

from api_client import ApiClient
from logging_config import configure_logging, get_logger
from model_loader import ModelNotFoundError, load_model as _load_model
from pipeline import (
    DB_URL,
    HIT_THRESHOLD_G,
    create_windows,
    detect_hits,
    load_data,
    merge_sensors,
)

log = get_logger(__name__)

# Configuración del buffer circular
POLL_INTERVAL = 1.0          # segundos entre lecturas de BD
BUFFER_SECONDS = 5.0         # segundos de datos acumulados en memoria
PROCESSED_TTL = 10.0         # segundos para mantener timestamps procesados


def load_model():
    """Wrapper local del loader compartido con logging del proyecto.

    Mantiene la firma legacy `(model, class_names, mean, std)` para no romper
    el AsyncInferenceEngine. La fuente de verdad es `model_loader.load_model`.
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
    """Normaliza ventanas y devuelve predicciones con probabilidades."""
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
                log.info("training_created", training_id=self.id_entrenamiento)
            if self.use_ws:
                await self.api.connect_ws()
                log.info("websocket_connected")

    async def run(self):
        """Arranca el productor y el consumidor concurrentemente."""
        log.info(
            "inference_started",
            poll_interval_s=POLL_INTERVAL,
            buffer_seconds=BUFFER_SECONDS,
            hit_threshold_g=HIT_THRESHOLD_G,
            use_api=self.use_api,
            use_ws=self.use_ws,
        )

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
                # `load_data` puede levantar psycopg2 / pandas errors; capturar
                # genérico con stack trace para no perder visibilidad y permitir
                # que el loop continúe.
                log.warning("db_read_failed", error=str(e), exc_info=True)
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

                log.info(
                    "punch_detected",
                    timestamp=time_str,
                    class_name=top["clase"],
                    probability=round(float(top["prob"]), 4),
                    power_g=potencia,
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
                        # WebSocket: errores de red, parsing, conexión cerrada.
                        # No interrumpe la inferencia, sólo se pierde un mensaje.
                        log.warning("websocket_send_failed", error=str(e), exc_info=True)

                # Subir a API REST
                if self.api and self.id_entrenamiento is not None:
                    id_golpe = self.api.map_prediction_to_golpe(top["clase"])
                    if id_golpe is not None:
                        try:
                            self.api.create_historial(
                                self.id_entrenamiento, id_golpe, potencia
                            )
                        except Exception as e:
                            # API REST: timeouts, 4xx/5xx, JSON malformado.
                            # Mismo principio que WS — registrar y seguir.
                            log.warning(
                                "api_create_history_failed",
                                error=str(e),
                                training_id=self.id_entrenamiento,
                                punch_id=id_golpe,
                                exc_info=True,
                            )

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
                log.info("training_finished", training_id=self.id_entrenamiento)
            except Exception as e:
                log.warning(
                    "training_finish_failed",
                    error=str(e),
                    training_id=self.id_entrenamiento,
                    exc_info=True,
                )
        if self.api and self.use_ws:
            try:
                await self.api.close_ws()
            except Exception as e:
                # Cierre del WS: aceptable fallar silenciosamente (proceso saliendo).
                log.debug("websocket_close_failed", error=str(e))


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

    configure_logging(service="ml_main")

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
        log.info("inference_stopped_by_user")


if __name__ == "__main__":
    main()
