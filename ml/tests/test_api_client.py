"""Tests unitarios para `api_client._parse_label`.

`_parse_label` convierte etiquetas del modelo (`jab_derecha_arriba`,
`cross_izquierda_abajo`, etc.) en la tupla `(name, limb, position)` que
mapea contra la tabla `golpe`. Es la única validación de entrada en
`ApiClient` y por lo tanto un punto donde regresiones silenciosas se
traducen en `punch_map` mal alineados.
"""

from __future__ import annotations

import pytest

from api_client import ApiClient


class TestParseLabel:
    """Casos canónicos extraídos del catálogo `golpe` que usa la app."""

    @pytest.mark.parametrize(
        ("label", "expected"),
        [
            ("jab_derecha_arriba", ("Jab", "Derecha", "Cabeza")),
            ("cross_izquierda_arriba", ("Cross", "Izquierda", "Cabeza")),
            ("hook_derecha_abajo", ("Gancho", "Derecha", "Cuerpo")),
            ("uppercut_izquierda_abajo", ("Upper", "Izquierda", "Cuerpo")),
        ],
    )
    def test_canonical_labels(self, label: str, expected: tuple[str, str, str]):
        assert ApiClient._parse_label(label) == expected

    def test_unknown_punch_type_capitalizes_name(self):
        """Etiquetas con un tipo desconocido caen al `capitalize()` por defecto."""
        name, limb, position = ApiClient._parse_label("swing_derecha_arriba")
        assert name == "Swing"
        assert limb == "Derecha"
        assert position == "Cabeza"

    def test_single_token_label_returns_defaults(self):
        """Etiquetas sin sufijo posicional caen a los defaults documentados."""
        name, limb, position = ApiClient._parse_label("jab")
        assert name == "Jab"
        assert limb == "Derecha"
        assert position == "Cabeza"

    def test_missing_side_token_defaults_to_derecha(self):
        """Si no se menciona izquierda/derecha, asumimos Derecha (igual que la app)."""
        _, limb, _ = ApiClient._parse_label("jab_arriba")
        assert limb == "Derecha"

    def test_arriba_maps_to_cabeza(self):
        _, _, position = ApiClient._parse_label("jab_derecha_arriba")
        assert position == "Cabeza"

    def test_anything_not_arriba_maps_to_cuerpo(self):
        # "abajo" o cualquier otra palabra que no sea "arriba" → cuerpo.
        _, _, position = ApiClient._parse_label("jab_derecha_centro")
        assert position == "Cuerpo"

    def test_label_is_case_insensitive(self):
        # `_parse_label` hace `.lower()` antes de comparar, así que la API
        # debe tolerar etiquetas en mayúsculas/minúsculas mixtas.
        assert ApiClient._parse_label("JAB_DERECHA_ARRIBA") == (
            "Jab",
            "Derecha",
            "Cabeza",
        )
