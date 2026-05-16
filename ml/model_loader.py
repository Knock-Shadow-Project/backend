"""Carga compartida del modelo PyTorch entrenado.

Antes de Phase C.2 existían 2 copias casi idénticas de `load_model()` en
`main.py` y `pi_inference.py`. Cada divergencia (ej. añadir scale, soportar
arquitecturas distintas) había que reflejarla en N archivos, con riesgo de
desync entre el modo cloud y el modo Pi.

Esta función única es la fuente de verdad. Devuelve un namedtuple con todo
lo necesario para inferir; los entrypoints sólo se ocupan del flujo de
control y los logs.
"""

from __future__ import annotations

import os
from dataclasses import dataclass

import numpy as np
import torch

from model_def import PunchCNN
from pipeline import FEATURE_COLS

# Rutas estándar dentro de `ml/model/`. Si cambian, se cambia en un solo sitio.
MODEL_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "model")
MODEL_PATH = os.path.join(MODEL_DIR, "punch_classifier.pt")
CLASS_NAMES_PATH = os.path.join(MODEL_DIR, "class_names.npy")
NORM_MEAN_PATH = os.path.join(MODEL_DIR, "norm_mean.npy")
NORM_STD_PATH = os.path.join(MODEL_DIR, "norm_std.npy")


@dataclass(frozen=True)
class LoadedModel:
    """Bundle inmutable con todo lo necesario para `predict_windows`.

    Devolver una dataclass (vs. una tupla) hace el código consumidor
    auto-documentado: `bundle.model` vs `result[0]`.
    """

    model: torch.nn.Module
    class_names: np.ndarray
    mean: np.ndarray
    std: np.ndarray
    device: torch.device
    num_classes: int


class ModelNotFoundError(FileNotFoundError):
    """Levantada cuando el checkpoint no existe en disco.

    Se separa de `FileNotFoundError` genérico para que los entrypoints
    puedan distinguir "modelo no entrenado todavía" de otros I/O errors
    y dar un mensaje útil al usuario (ej. "Entrena con: python train.py").
    """


def _select_device() -> torch.device:
    """Elige CUDA si está disponible, CPU en caso contrario.

    Aislado en una función propia para testeo y para que un futuro Pi con
    GPU (Jetson) pueda overridar fácilmente vía monkeypatch sin tocar
    load_model().
    """
    return torch.device("cuda" if torch.cuda.is_available() else "cpu")


def load_model(
    model_path: str = MODEL_PATH,
    class_names_path: str = CLASS_NAMES_PATH,
    norm_mean_path: str = NORM_MEAN_PATH,
    norm_std_path: str = NORM_STD_PATH,
) -> LoadedModel:
    """Carga el modelo entrenado y los metadatos de normalización.

    Args:
        model_path: Path al checkpoint `.pt`. Default: `model/punch_classifier.pt`.
        class_names_path: Path al array de etiquetas. Default: `model/class_names.npy`.
        norm_mean_path: Path a la media de normalización. Default: `model/norm_mean.npy`.
        norm_std_path: Path a la desv. estándar. Default: `model/norm_std.npy`.

    Returns:
        `LoadedModel` listo para inferencia (model en modo eval, en el device elegido).

    Raises:
        ModelNotFoundError: si el checkpoint no existe.
    """
    if not os.path.exists(model_path):
        raise ModelNotFoundError(
            f"No se encontró el modelo en {model_path}. "
            "Entrena primero con: python train.py"
        )

    device = _select_device()
    checkpoint = torch.load(model_path, map_location=device)
    num_classes = int(checkpoint["num_classes"])
    in_channels = int(checkpoint.get("in_channels", len(FEATURE_COLS)))

    model = PunchCNN(num_classes=num_classes, in_channels=in_channels)
    model.load_state_dict(checkpoint["model_state"])
    model.to(device)
    model.eval()

    class_names = np.load(class_names_path, allow_pickle=True)
    mean = np.load(norm_mean_path)
    std = np.load(norm_std_path)

    return LoadedModel(
        model=model,
        class_names=class_names,
        mean=mean,
        std=std,
        device=device,
        num_classes=num_classes,
    )
