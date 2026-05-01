import os
import numpy as np
import pandas as pd
import psycopg2
from scipy.signal import find_peaks, butter, filtfilt

DB_URL = os.getenv("DATABASE_URL", "postgres://knockshadow:knockshadow@127.0.0.1:5432/knockshadow")
SENSOR_MAC_1 = os.getenv("SENSOR_MAC_1", "DF:65:81:D0:D7:E5")
SENSOR_MAC_2 = os.getenv("SENSOR_MAC_2", "CB:01:10:3E:0D:61")
SAMPLE_RATE = int(os.getenv("SAMPLE_RATE", "60"))
WINDOW_SIZE = int(os.getenv("WINDOW_SIZE", "64"))
HIT_THRESHOLD_G = float(os.getenv("HIT_THRESHOLD_G", "8.0"))

_DATA_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "data")
DEFAULT_DATASET = os.path.join(_DATA_DIR, "dataset.npz")

FEATURE_COLS = ["x1", "y1", "z1", "x2", "y2", "z2"]


def load_data(start_time, end_time, db_url: str = DB_URL) -> pd.DataFrame:
    conn = psycopg2.connect(db_url)
    query = """
    SELECT received_at, device_mac, x, y, z
    FROM ble_samples
    WHERE received_at BETWEEN %s AND %s
      AND device_mac IN (%s, %s)
    ORDER BY received_at ASC;
    """
    df = pd.read_sql(query, conn, params=(start_time, end_time, SENSOR_MAC_1, SENSOR_MAC_2))
    conn.close()
    df["received_at"] = pd.to_datetime(df["received_at"], utc=True)
    return df


def lowpass_filter(signal: np.ndarray, cutoff: float = 10.0, fs: int = SAMPLE_RATE) -> np.ndarray:
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

    df1 = df1.sort_values("received_at").rename(columns={"x": "x1", "y": "y1", "z": "z1"})
    df2 = df2.sort_values("received_at").rename(columns={"x": "x2", "y": "y2", "z": "z2"})

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
        merged[col] = lowpass_filter(merged[col].values)

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
) -> np.ndarray:
    windows = []
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

    if not windows:
        return np.empty((0, window_size, len(FEATURE_COLS)), dtype=np.float32)
    return np.array(windows, dtype=np.float32)


def save_dataset(
    X: np.ndarray,
    y: np.ndarray,
    dataset_file: str = DEFAULT_DATASET,
) -> int:
    os.makedirs(os.path.dirname(dataset_file), exist_ok=True)

    if os.path.exists(dataset_file):
        existing = np.load(dataset_file, allow_pickle=True)
        X = np.concatenate([existing["X"], X], axis=0)
        y = np.concatenate([existing["y"], y], axis=0)

    np.savez(dataset_file, X=X, y=y)
    return int(len(y))


def load_dataset(dataset_file: str = DEFAULT_DATASET):
    if not os.path.exists(dataset_file):
        return (
            np.empty((0, WINDOW_SIZE, len(FEATURE_COLS)), dtype=np.float32),
            np.array([], dtype=str),
        )
    data = np.load(dataset_file, allow_pickle=True)
    return data["X"], data["y"]
