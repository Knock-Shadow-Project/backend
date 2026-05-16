"""Modelos pydantic para validar las cargas útiles del cliente API.

Antes de Phase C.6 el cliente accedía directamente a dicts (`p["name"]`,
`data["token"]`, etc.) sin validación. Eso convertía errores tempranos
(faltó un campo, vino con tipo inesperado) en `KeyError`/`TypeError` lejos
del punto de origen, complicando el debugging.

Estos modelos son `BaseModel` simples — sin lógica de negocio — para:
    - Validar que la respuesta del backend tiene la forma esperada.
    - Validar los payloads salientes ANTES del POST, en vez de que el
      servidor devuelva 400.
    - Servir de documentación viva del contrato API (los nombres y tipos
      están en el código).
"""

from __future__ import annotations

import datetime as _dt

from pydantic import BaseModel, ConfigDict, Field, field_validator

# Etiquetas reconocidas por el clasificador (deben mantenerse en sync con
# el dataset etiquetado). Se valida que las predicciones del modelo
# pertenezcan a este set; si no, se loggea y se descarta antes de tocar la API.
KNOWN_PUNCH_TYPES = frozenset({"jab", "cross", "hook", "uppercut", "swing"})
KNOWN_LIMBS = frozenset({"izquierda", "derecha"})
KNOWN_POSITIONS = frozenset({"arriba", "abajo"})


class Punch(BaseModel):
    """Respuesta de `GET /punches`: catálogo de golpes registrados.

    El backend devuelve `name/limb/position` traducidos al español (`Jab`,
    `Derecha`, `Cabeza`). El cliente las usa como llave en `punch_map`.
    """

    model_config = ConfigDict(populate_by_name=True, extra="ignore")

    punch_id: int = Field(..., alias="punch_id")
    name: str
    limb: str | None = None
    position: str | None = None


class Training(BaseModel):
    """Respuesta de `POST /trainings`: entrenamiento creado en cloud."""

    model_config = ConfigDict(populate_by_name=True, extra="ignore")

    training_id: int
    user_id: int
    training_type: str | None = None
    start_time: _dt.datetime | None = None
    end_time: _dt.datetime | None = None


class CreateTrainingPayload(BaseModel):
    """Payload saliente para `POST /trainings`.

    Validamos en cliente para evitar el round-trip con un 400 si falta un
    campo o el tipo es incorrecto.
    """

    user_id: int = Field(..., gt=0)
    training_type: str = Field(default="Estandar", min_length=1, max_length=50)
    start_time: _dt.datetime
    end_time: _dt.datetime | None = None
    calories: int | None = None


class CreateHistoryPayload(BaseModel):
    """Payload saliente para `POST /history`: registra un golpe detectado."""

    training_id: int = Field(..., gt=0)
    punch_id: int = Field(..., gt=0)
    power: float | None = Field(default=None, ge=0)


class ParsedLabel(BaseModel):
    """Etiqueta del modelo descompuesta en (name, limb, position).

    Atomic data shape para evitar tuplas anónimas en el código consumidor:
    `parsed.limb` deja claro qué campo es qué.
    """

    name: str
    limb: str
    position: str

    @field_validator("name", "limb", "position")
    @classmethod
    def _non_empty(cls, v: str) -> str:
        if not v or not v.strip():
            raise ValueError("must be non-empty")
        return v
