"""Configuración compartida de logging estructurado para los servicios Python.

Por qué structlog (y no `logging` raw):
    1. Salida JSON nativa: consumible por Grafana/Loki sin parsers ad-hoc.
    2. Bind de contexto: `log = log.bind(training_id=42)` propaga campos a
       todos los eventos sin reescribir cada `log.info(...)`.
    3. Compatible con stdlib `logging`: el handler de structlog adopta los
       logs de librerías terceras (requests, torch, sqlite3) sin reconfigurar
       cada uno.

Espeja el setup de Rust (`tracing_subscriber::fmt()` con JSON layer en
api-db/pi-service) para que el stack completo emita eventos en el mismo
formato y se pueda correlacionar por `correlation_id` cuando lo añadamos
en Phase C.4.

Uso:
    from logging_config import configure_logging, get_logger
    configure_logging(service="pi_inference")
    log = get_logger(__name__)
    log.info("inference_started", user_id=1, training_id=42)
"""

from __future__ import annotations

import logging
import os
import sys
from typing import Any

import structlog

# Mapping de strings a niveles para configuración por env var.
_LEVELS = {
    "DEBUG": logging.DEBUG,
    "INFO": logging.INFO,
    "WARNING": logging.WARNING,
    "WARN": logging.WARNING,
    "ERROR": logging.ERROR,
    "CRITICAL": logging.CRITICAL,
}


def configure_logging(
    service: str,
    *,
    level: str | None = None,
    json_output: bool | None = None,
) -> None:
    """Configura logging estructurado para el proceso completo.

    Llamar una sola vez al inicio de cada entrypoint (`pi_inference.py`,
    `main.py`, `app.py`, etc.). Llamadas adicionales son idempotentes pero
    redundantes.

    Args:
        service: Identificador corto del servicio (`pi_inference`,
            `ml_app`, `sync`). Se añade como campo fijo a cada evento.
        level: Nivel mínimo. Si es None, se lee de `LOG_LEVEL`
            (default INFO).
        json_output: Si True fuerza JSON, si False fuerza output legible.
            Si None se decide por TTY: JSON cuando stdout no es TTY
            (producción/Docker), legible en terminal interactiva.
    """
    log_level = _LEVELS.get((level or os.getenv("LOG_LEVEL", "INFO")).upper(), logging.INFO)
    use_json = (
        json_output
        if json_output is not None
        else not sys.stdout.isatty() or os.getenv("LOG_JSON", "").lower() in {"1", "true", "yes"}
    )

    # Procesadores compartidos: contexto, timestamp, nivel, info de excepción.
    # Orden importa — `add_log_level` debe ir antes que cualquier renderer.
    shared_processors: list[Any] = [
        structlog.contextvars.merge_contextvars,
        structlog.processors.add_log_level,
        structlog.processors.StackInfoRenderer(),
        structlog.processors.TimeStamper(fmt="iso", utc=True),
        structlog.processors.format_exc_info,
    ]

    if use_json:
        renderer: Any = structlog.processors.JSONRenderer()
    else:
        # ConsoleRenderer con colores: lectura local más cómoda.
        renderer = structlog.dev.ConsoleRenderer(colors=True)

    structlog.configure(
        processors=[*shared_processors, renderer],
        wrapper_class=structlog.make_filtering_bound_logger(log_level),
        logger_factory=structlog.PrintLoggerFactory(),
        cache_logger_on_first_use=True,
    )

    # Redirige también stdlib logging (librerías terceras) por el mismo pipe.
    logging.basicConfig(
        format="%(message)s",
        stream=sys.stdout,
        level=log_level,
    )

    # Campo fijo del servicio para todos los eventos del proceso.
    structlog.contextvars.bind_contextvars(service=service)


def get_logger(name: str) -> structlog.stdlib.BoundLogger:
    """Devuelve un logger structlog con el módulo como `logger`.

    Args:
        name: Generalmente `__name__` para identificar el módulo emisor.
    """
    return structlog.get_logger(name)
