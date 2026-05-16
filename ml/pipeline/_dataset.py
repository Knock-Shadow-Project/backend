"""Persistencia del dataset etiquetado en disco (`dataset.npz`).

Diseño: el dataset vive en un único `.npz` con tres arrays paralelos
(X, y, ids). Cada operación (save/delete/relabel) carga el archivo entero,
modifica en memoria y reescribe — funciona bien para datasets de hasta
decenas de miles de muestras (uso típico). Si crece más, migrar a SQLite/
Parquet sin cambiar la API pública.
"""

from __future__ import annotations

import os
import uuid

import numpy as np

from ._constants import DEFAULT_DATASET, FEATURE_COLS, WINDOW_SIZE


def save_dataset(
    X: np.ndarray,
    y: np.ndarray,
    ids: np.ndarray | None = None,
    dataset_file: str = DEFAULT_DATASET,
) -> tuple[int, np.ndarray]:
    """Guarda muestras en el dataset. Si se pasan ids, se usan; si no, se generan UUIDs.
    Devuelve (total_muestras, ids_usados)."""
    os.makedirs(os.path.dirname(dataset_file), exist_ok=True)

    if ids is None:
        ids = np.array([str(uuid.uuid4()) for _ in range(len(y))], dtype=object)

    if os.path.exists(dataset_file):
        existing = np.load(dataset_file, allow_pickle=True)
        X = np.concatenate([existing["X"], X], axis=0)
        y = np.concatenate([existing["y"], y], axis=0)
        ids = np.concatenate([existing["ids"], ids], axis=0)

    np.savez(dataset_file, X=X, y=y, ids=ids)
    return int(len(y)), ids


def load_dataset(dataset_file: str = DEFAULT_DATASET):
    if not os.path.exists(dataset_file):
        empty_X = np.empty((0, WINDOW_SIZE, len(FEATURE_COLS)), dtype=np.float32)
        empty_y = np.array([], dtype=str)
        empty_ids = np.array([], dtype=str)
        return empty_X, empty_y, empty_ids
    data = np.load(dataset_file, allow_pickle=True)
    return data["X"], data["y"], data["ids"]


def delete_last_samples(n: int = 1, dataset_file: str = DEFAULT_DATASET) -> int:
    """Elimina las últimas n muestras del dataset. Devuelve cuántas quedan."""
    if not os.path.exists(dataset_file):
        return 0
    data = np.load(dataset_file, allow_pickle=True)
    X, y, ids = data["X"], data["y"], data["ids"]
    if len(y) == 0:
        return 0
    n = min(n, len(y))
    X = X[:-n]
    y = y[:-n]
    ids = ids[:-n]
    np.savez(dataset_file, X=X, y=y, ids=ids)
    return int(len(y))


def delete_samples_by_id(
    sample_ids: list[str], dataset_file: str = DEFAULT_DATASET
) -> int:
    """Elimina muestras específicas por su ID. Devuelve cuántas quedan."""
    if not os.path.exists(dataset_file):
        return 0
    data = np.load(dataset_file, allow_pickle=True)
    X, y, ids = data["X"], data["y"], data["ids"]
    if len(y) == 0:
        return 0
    mask = ~np.isin(ids, sample_ids)
    X = X[mask]
    y = y[mask]
    ids = ids[mask]
    np.savez(dataset_file, X=X, y=y, ids=ids)
    return int(len(y))


def delete_samples_by_label(
    label: str, dataset_file: str = DEFAULT_DATASET
) -> tuple[int, int]:
    """Elimina todas las muestras con una etiqueta dada.
    Devuelve (cuántas se eliminaron, cuántas quedan)."""
    if not os.path.exists(dataset_file):
        return 0, 0
    data = np.load(dataset_file, allow_pickle=True)
    X, y, ids = data["X"], data["y"], data["ids"]
    if len(y) == 0:
        return 0, 0
    mask = y != label
    removed = int((~mask).sum())
    X = X[mask]
    y = y[mask]
    ids = ids[mask]
    np.savez(dataset_file, X=X, y=y, ids=ids)
    return removed, int(len(y))


def relabel_samples_by_label(
    old_label: str, new_label: str, dataset_file: str = DEFAULT_DATASET
) -> tuple[int, int]:
    """Cambia la etiqueta de todas las muestras con old_label a new_label.
    Devuelve (cuántas se cambiaron, total muestras)."""
    if not os.path.exists(dataset_file):
        return 0, 0
    data = np.load(dataset_file, allow_pickle=True)
    X, y, ids = data["X"], data["y"], data["ids"]
    if len(y) == 0:
        return 0, 0
    mask = y == old_label
    changed = int(mask.sum())
    if changed > 0:
        y = y.copy()
        y[mask] = new_label
        np.savez(dataset_file, X=X, y=y, ids=ids)
    return changed, int(len(y))


def get_recent_samples(n: int = 10, dataset_file: str = DEFAULT_DATASET):
    """Devuelve las últimas n muestras del dataset como lista de dicts."""
    if not os.path.exists(dataset_file):
        return []
    data = np.load(dataset_file, allow_pickle=True)
    X, y, ids = data["X"], data["y"], data["ids"]
    if len(y) == 0:
        return []
    n = min(n, len(y))
    recent = []
    for i in range(1, n + 1):
        idx = len(y) - i
        recent.append(
            {
                "id": str(ids[idx]),
                "label": str(y[idx]),
                "index": idx,
            }
        )
    return recent
