"""Tests para los modelos pydantic del cliente API."""

from __future__ import annotations

import datetime as _dt

import pytest
from pydantic import ValidationError

from api_models import (
    CreateHistoryPayload,
    CreateTrainingPayload,
    ParsedLabel,
    Punch,
    Training,
)


class TestPunch:
    def test_minimal_payload(self):
        p = Punch.model_validate({"punch_id": 1, "name": "Jab"})
        assert p.punch_id == 1
        assert p.name == "Jab"
        assert p.limb is None
        assert p.position is None

    def test_full_payload(self):
        p = Punch.model_validate(
            {
                "punch_id": 2,
                "name": "Cross",
                "limb": "Izquierda",
                "position": "Cabeza",
            }
        )
        assert p.limb == "Izquierda"

    def test_extra_fields_are_ignored(self):
        # El backend puede añadir campos sin romper el cliente — el modelo
        # filtra extras silenciosamente.
        p = Punch.model_validate(
            {"punch_id": 1, "name": "Jab", "internal_audit_id": 42}
        )
        assert p.punch_id == 1

    def test_missing_required_field_raises(self):
        with pytest.raises(ValidationError):
            Punch.model_validate({"name": "Jab"})  # punch_id falta


class TestTraining:
    def test_basic(self):
        t = Training.model_validate(
            {"training_id": 10, "user_id": 5, "training_type": "Estandar"}
        )
        assert t.training_id == 10
        assert t.user_id == 5


class TestCreateTrainingPayload:
    def test_valid(self):
        p = CreateTrainingPayload(
            user_id=1,
            training_type="Estandar",
            start_time=_dt.datetime(2026, 5, 16, tzinfo=_dt.timezone.utc),
        )
        assert p.user_id == 1

    def test_user_id_must_be_positive(self):
        with pytest.raises(ValidationError):
            CreateTrainingPayload(
                user_id=0,
                start_time=_dt.datetime.now(_dt.timezone.utc),
            )

    def test_training_type_default(self):
        p = CreateTrainingPayload(
            user_id=1,
            start_time=_dt.datetime.now(_dt.timezone.utc),
        )
        assert p.training_type == "Estandar"


class TestCreateHistoryPayload:
    def test_valid(self):
        p = CreateHistoryPayload(training_id=1, punch_id=2, power=12.5)
        assert p.power == 12.5

    def test_negative_power_rejected(self):
        with pytest.raises(ValidationError):
            CreateHistoryPayload(training_id=1, punch_id=2, power=-1.0)

    def test_power_optional(self):
        p = CreateHistoryPayload(training_id=1, punch_id=2)
        assert p.power is None


class TestParsedLabel:
    def test_valid(self):
        p = ParsedLabel(name="Jab", limb="Derecha", position="Cabeza")
        assert p.name == "Jab"

    @pytest.mark.parametrize("field", ["name", "limb", "position"])
    def test_empty_string_rejected(self, field: str):
        kwargs = {"name": "Jab", "limb": "Derecha", "position": "Cabeza"}
        kwargs[field] = ""
        with pytest.raises(ValidationError):
            ParsedLabel(**kwargs)

    @pytest.mark.parametrize("field", ["name", "limb", "position"])
    def test_whitespace_only_rejected(self, field: str):
        kwargs = {"name": "Jab", "limb": "Derecha", "position": "Cabeza"}
        kwargs[field] = "   "
        with pytest.raises(ValidationError):
            ParsedLabel(**kwargs)
