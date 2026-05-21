"""Configuración tipada para los servicios Python.

Por qué dataclasses (y no `pydantic-settings`):
    - `pydantic-settings` añade ~6 MB al image del Pi y un import muy pesado
      en cold start. Para el alcance actual (envs simples, validación trivial)
      las dataclasses de stdlib + un loader explícito cubren el caso sin
      transitive deps.
    - Si en Phase C.6 introducimos pydantic para validar payloads HTTP,
      podemos migrar este módulo trivialmente reusando los mismos nombres.

Uso:
    from config import load_inference_config
    cfg = load_inference_config()
    print(cfg.db_path, cfg.sensor_mac_1)

Cada dataclass declara explícitamente los campos *requeridos* (sin default,
falla al arranque si la env var no está) y los *opcionales* (con default).
Esto coincide con el patrón fail-fast aplicado en Rust (Phase A.1).
"""

from __future__ import annotations

import os
from dataclasses import dataclass, field
from typing import Callable, TypeVar

T = TypeVar("T")


def _required_str(name: str) -> str:
    """Lee una variable de entorno o levanta un error explícito.

    Centralizamos el mensaje de error para que `docker logs` muestre siempre
    el mismo formato y sea trivial diagnosticar configuraciones incompletas.
    """
    value = os.getenv(name)
    if value is None or value.strip() == "":
        raise RuntimeError(
            f"Environment variable {name!r} is required but unset/empty. "
            "See docker-compose.*.yaml or docs/Deployment.md for the full list."
        )
    return value


def _optional(name: str, default: T, *, cast: Callable[[str], T] = str) -> T:
    """Lee una env var opcional con default y casting tipado."""
    raw = os.getenv(name)
    if raw is None or raw == "":
        return default
    try:
        return cast(raw)
    except (TypeError, ValueError) as e:
        raise RuntimeError(
            f"Environment variable {name!r} has invalid value {raw!r}: {e}"
        ) from e


# ---------------- Inference (pi_inference.py + main.py) ---------------------


@dataclass(frozen=True)
class InferenceConfig:
    """Configuración del motor de inferencia (cloud + Pi)."""

    db_path: str | None
    db_url: str
    sensor_mac_1: str
    sensor_mac_2: str
    sample_rate: int
    window_size: int
    hit_threshold_g: float
    sensor_scale: float
    poll_interval_s: float
    buffer_seconds: float
    processed_ttl_s: float

    @property
    def is_pi_mode(self) -> bool:
        """True si DB_PATH está definida → modo Pi-offline (SQLite local)."""
        return self.db_path is not None


def load_inference_config() -> InferenceConfig:
    """Carga `InferenceConfig` desde el entorno con defaults seguros.

    Los MACs por defecto se mantienen aquí (vs. hardcoded por archivo) para
    que cambien en un solo lugar. En despliegues reales SIEMPRE se sobreescriben
    vía docker-compose.
    """
    db_path = os.getenv("DB_PATH")
    return InferenceConfig(
        db_path=db_path if db_path else None,
        db_url=_optional(
            "DATABASE_URL",
            "postgres://knockshadow:knockshadow@127.0.0.1:5432/knockshadow",
        ),
        sensor_mac_1=_optional("SENSOR_MAC_1", "DF:65:81:D0:D7:E5"),
        sensor_mac_2=_optional("SENSOR_MAC_2", "CB:01:10:3E:0D:61"),
        sample_rate=_optional("SAMPLE_RATE", 50, cast=int),
        window_size=_optional("WINDOW_SIZE", 64, cast=int),
        hit_threshold_g=_optional("HIT_THRESHOLD_G", 1.2, cast=float),
        sensor_scale=_optional("SENSOR_SCALE", 1000.0, cast=float),
        poll_interval_s=_optional("POLL_INTERVAL_S", 1.0, cast=float),
        buffer_seconds=_optional("BUFFER_SECONDS", 5.0, cast=float),
        processed_ttl_s=_optional("PROCESSED_TTL_S", 10.0, cast=float),
    )


# ---------------- API client (cloud REST + WebSocket) -----------------------


@dataclass(frozen=True)
class ApiClientConfig:
    """Configuración del cliente REST/WebSocket hacia la API cloud."""

    base_url: str
    ws_url: str
    timeout_s: float = field(default=10.0)


def load_api_client_config() -> ApiClientConfig:
    """Defaults compatibles con docker-compose para desarrollo local.

    En producción siempre se pasan `API_BASE_URL` y `WS_URL` vía env.
    """
    return ApiClientConfig(
        base_url=_optional("API_BASE_URL", "http://localhost:3000"),
        ws_url=_optional("WS_URL", "ws://localhost:3000/ws"),
        timeout_s=_optional("API_TIMEOUT_S", 10.0, cast=float),
    )


# ---------------- Sync (sync_ble_to_cloud.py) -------------------------------


@dataclass(frozen=True)
class SyncConfig:
    """Configuración del script de sincronización Pi → cloud."""

    sqlite_path: str
    pg_url: str
    batch_size: int = field(default=1000)


def load_sync_config() -> SyncConfig:
    """Carga la configuración de sync. `DATABASE_URL` es REQUERIDA aquí."""
    return SyncConfig(
        sqlite_path=_optional("DB_PATH", "pi_data.db"),
        pg_url=_required_str("DATABASE_URL"),
        batch_size=_optional("SYNC_BATCH_SIZE", 1000, cast=int),
    )
