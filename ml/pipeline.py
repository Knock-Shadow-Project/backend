import os
import uuid
import numpy as np
import pandas as pd
import psycopg2
from scipy.signal import find_peaks, butter, filtfilt

DB_URL = os.getenv(
    "DATABASE_URL", "postgres://knockshadow:knockshadow@127.0.0.1:5432/knockshadow"
)
SENSOR_MAC_1 = os.getenv("SENSOR_MAC_1", "DF:65:81:D0:D7:E5")
SENSOR_MAC_2 = os.getenv("SENSOR_MAC_2", "CB:01:10:3E:0D:61")
SAMPLE_RATE = int(os.getenv("SAMPLE_RATE", "50"))
WINDOW_SIZE = int(os.getenv("WINDOW_SIZE", "64"))
HIT_THRESHOLD_G = float(os.getenv("HIT_THRESHOLD_G", "1.5"))

_DATA_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "data")
DEFAULT_DATASET = os.path.join(_DATA_DIR, "dataset.npz")

FEATURE_COLS = ["x1", "y1", "z1", "x2", "y2", "z2"]
SENSOR_SCALE = float(os.getenv("SENSOR_SCALE", "1000.0"))  # raw units per G


def load_data(start_time, end_time, db_url: str = DB_URL) -> pd.DataFrame:
    conn = psycopg2.connect(db_url)
    query = """
    SELECT received_at, device_mac, x, y, z
    FROM ble_samples
    WHERE received_at BETWEEN %s AND %s
      AND device_mac IN (%s, %s)
    ORDER BY received_at ASC;
    """
    df = pd.read_sql(
        query, conn, params=(start_time, end_time, SENSOR_MAC_1, SENSOR_MAC_2)
    )
    conn.close()
    df["received_at"] = pd.to_datetime(df["received_at"], utc=True)
    return df


def lowpass_filter(
    signal: np.ndarray, cutoff: float = 10.0, fs: int = SAMPLE_RATE
) -> np.ndarray:
    # filtfilt needs at least padlen = 3 * max(len(a), len(b)) - 1 = 9 samples for order-3 filter
    if len(signal) <= 9:
        return signal
    b, a = butter(3, cutoff / (0.5 * fs), btype="low")
    return filtfilt(b, a, signal)


def merge_sensors(
    df: pd.DataFrame,
    mac1: str = SENSOR_MAC_1,
    mac2: str = SENSOR_MAC_2,
) -> pd.DataFrame:
    df1 = df[df["device_mac"] == mac1][["received_at", "x", "y", "z"]].copy()
    df2 = df[df["device_mac"] == mac2][["received_at", "x", "y", "z"]].copy()

    df1 = df1.sort_values("received_at").rename(
        columns={"x": "x1", "y": "y1", "z": "z1"}
    )
    df2 = df2.sort_values("received_at").rename(
        columns={"x": "x2", "y": "y2", "z": "z2"}
    )

    if df1.empty or df2.empty:
        return pd.DataFrame()

    # Nearest-neighbor merge with 100ms tolerance (~6 samples at 60Hz)
    merged = pd.merge_asof(
        df1,
        df2,
        on="received_at",
        direction="nearest",
        tolerance=pd.Timedelta("100ms"),
    ).dropna()

    if merged.empty:
        return pd.DataFrame()

    for col in FEATURE_COLS:
        merged[col] = lowpass_filter(merged[col].values / SENSOR_SCALE)

    merged["mag1"] = np.sqrt(merged["x1"] ** 2 + merged["y1"] ** 2 + merged["z1"] ** 2)
    merged["mag2"] = np.sqrt(merged["x2"] ** 2 + merged["y2"] ** 2 + merged["z2"] ** 2)
    merged["mag"] = (merged["mag1"] + merged["mag2"]) / 2

    return merged.reset_index(drop=True)


def detect_hits(
    df: pd.DataFrame,
    threshold: float = HIT_THRESHOLD_G,
    distance: int = 15,
) -> np.ndarray:
    if df.empty or "mag" not in df.columns:
        return np.array([], dtype=int)
    peaks, _ = find_peaks(df["mag"].values, height=threshold, distance=distance)
    return peaks


def create_windows(
    df: pd.DataFrame,
    peaks: np.ndarray,
    window_size: int = WINDOW_SIZE,
    return_valid_peaks: bool = False,
):
    windows = []
    valid_peaks = []
    half = window_size // 2
    n = len(df)

    for p in peaks:
        start = p - half
        end = p + half
        if start < 0 or end > n:
            continue
        window = df.iloc[start:end][FEATURE_COLS].values
        if len(window) == window_size:
            windows.append(window)
            valid_peaks.append(int(p))

    if not windows:
        empty = np.empty((0, window_size, len(FEATURE_COLS)), dtype=np.float32)
        if return_valid_peaks:
            return empty, np.array([], dtype=int)
        return empty

    result = np.array(windows, dtype=np.float32)
    if return_valid_peaks:
        return result, np.array(valid_peaks, dtype=int)
    return result


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


def delete_samples_by_id(sample_ids: list[str], dataset_file: str = DEFAULT_DATASET) -> int:
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


def delete_samples_by_label(label: str, dataset_file: str = DEFAULT_DATASET) -> tuple[int, int]:
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
        recent.append({
            "id": str(ids[idx]),
            "label": str(y[idx]),
            "index": idx,
        })
    return recent
