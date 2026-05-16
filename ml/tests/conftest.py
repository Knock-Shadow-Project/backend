"""Configuración global de pytest para ml/tests.

Añade el directorio padre (ml/) al `sys.path` para que los tests puedan
importar `pipeline`, `api_client`, etc. como módulos top-level — el mismo
patrón que usan los entrypoints en producción.
"""

from __future__ import annotations

import sys
from pathlib import Path

_ML_DIR = Path(__file__).resolve().parent.parent
if str(_ML_DIR) not in sys.path:
    sys.path.insert(0, str(_ML_DIR))
