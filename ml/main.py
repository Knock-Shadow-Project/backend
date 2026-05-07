import datetime
import time
import os
import sys
import numpy as np
import pandas as pd
import torch
import torch.nn as nn

from pipeline import (
    DB_URL,
    FEATURE_COLS,
    HIT_THRESHOLD_G,
    SENSOR_MAC_1,
    SENSOR_MAC_2,
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

    # Normalizar con los parámetros de entrenamiento
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


def run_inference_loop(
    model, class_names, mean, std, duration_sec=5.0, cooldown_sec=1.0
):
    """Bucle de inferencia en tiempo real."""
    print(f"\nBucle de inferencia iniciado (Ctrl+C para detener)")
    print(f"Ventana de análisis: {duration_sec}s | Cooldown: {cooldown_sec}s")
    print(f"Umbral de detección: {HIT_THRESHOLD_G} G\n")

    last_end = None

    try:
        while True:
            input(f"Presiona Enter para grabar {duration_sec}s de datos...")
            t_start = datetime.datetime.now(datetime.timezone.utc)
            t_end = t_start + datetime.timedelta(seconds=duration_sec)

            # Esperar a que pase el tiempo de grabación
            time.sleep(duration_sec)

            # Cargar datos del intervalo
            try:
                raw = load_data(t_start, t_end, db_url=DB_URL)
            except Exception as e:
                print(f"Error de BD: {e}")
                continue

            if raw.empty:
                continue

            merged = merge_sensors(raw)
            if merged.empty:
                continue

            peaks = detect_hits(merged, threshold=HIT_THRESHOLD_G)
            windows, valid_peaks = create_windows(
                merged, peaks, return_valid_peaks=True
            )

            if len(windows) == 0:
                continue

            predictions = predict_windows(model, windows, class_names, mean, std)

            # Mostrar resultados
            ts_str = t_start.strftime("%H:%M:%S")
            print(f"[{ts_str}] {len(windows)} golpe(s) detectado(s):")
            for i, preds in enumerate(predictions):
                peak_idx = valid_peaks[i]
                if "received_at" in merged.columns and peak_idx < len(merged):
                    ts = merged["received_at"].iloc[peak_idx]
                    time_str = ts.strftime("%H:%M:%S.%f")[:-3]
                else:
                    time_str = "?"

                top = preds[0]
                rest = ", ".join(
                    [f"{p['clase']} {p['prob'] * 100:.0f}%" for p in preds[1:]]
                )
                line = f"  #{i + 1} @ {time_str} → {top['clase']} ({top['prob'] * 100:.1f}%)"
                if rest:
                    line += f"  |  {rest}"
                print(line)
            print()

            # Cooldown
            time.sleep(cooldown_sec)

    except KeyboardInterrupt:
        print("\nInferencia detenida.")


def run_single(duration_sec=5.0):
    """Ejecuta una sola ventana de inferencia y muestra resultados."""
    model, class_names, mean, std = load_model()

    input(f"\nPresiona Enter para grabar {duration_sec}s de datos...")

    print(f"\nGrabando {duration_sec}s de datos...")
    t_start = datetime.datetime.now(datetime.timezone.utc)
    t_end = t_start + datetime.timedelta(seconds=duration_sec)
    time.sleep(duration_sec)

    raw = load_data(t_start, t_end, db_url=DB_URL)
    if raw.empty:
        print("No se recibieron datos. ¿Están los sensores activos?")
        return

    merged = merge_sensors(raw)
    if merged.empty:
        print("No se pudieron sincronizar los dos sensores.")
        return

    peaks = detect_hits(merged, threshold=HIT_THRESHOLD_G)
    windows, valid_peaks = create_windows(merged, peaks, return_valid_peaks=True)

    if len(windows) == 0:
        print(f"No se detectaron golpes (umbral={HIT_THRESHOLD_G}G).")
        return

    predictions = predict_windows(model, windows, class_names, mean, std)

    print(f"\n{len(windows)} golpe(s) detectado(s):\n")
    for i, preds in enumerate(predictions):
        peak_idx = valid_peaks[i]
        if "received_at" in merged.columns and peak_idx < len(merged):
            ts = merged["received_at"].iloc[peak_idx]
            time_str = ts.strftime("%H:%M:%S.%f")[:-3]
        else:
            time_str = "?"

        top = preds[0]
        print(f"  Golpe #{i + 1} @ {time_str}")
        print(f"    → Predicción: {top['clase']} ({top['prob'] * 100:.1f}%)")
        for p in preds[1:]:
            print(f"       {p['clase']}: {p['prob'] * 100:.1f}%")
        print()


def main():
    import argparse

    parser = argparse.ArgumentParser(
        description="Inferencia del clasificador de golpes"
    )
    parser.add_argument(
        "--loop",
        action="store_true",
        help="Bucle continuo de inferencia en tiempo real",
    )
    parser.add_argument(
        "--duration",
        type=float,
        default=5.0,
        help="Duración en segundos de cada ventana de análisis (default: 5.0)",
    )
    parser.add_argument(
        "--cooldown",
        type=float,
        default=1.0,
        help="Segundos de espera entre ventanas en modo loop (default: 1.0)",
    )
    args = parser.parse_args()

    model, class_names, mean, std = load_model()

    if args.loop:
        run_inference_loop(model, class_names, mean, std, args.duration, args.cooldown)
    else:
        run_single(args.duration)


if __name__ == "__main__":
    main()
