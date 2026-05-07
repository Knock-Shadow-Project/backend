"""Cliente para interactuar con la API de KnockShadow."""

import asyncio
import json
import os

import requests
import websockets

API_BASE_URL = os.getenv("API_BASE_URL", "http://localhost:3000")
WS_URL = os.getenv("WS_URL", "ws://localhost:3000/ws")


class ApiClient:
    """Cliente síncrono/asíncrono para la API REST y WebSocket."""

    def __init__(
        self,
        base_url: str = API_BASE_URL,
        ws_url: str = WS_URL,
        correo: str = "",
        contrasena: str = "",
    ):
        self.base_url = base_url.rstrip("/")
        self.ws_url = ws_url
        self.ws = None
        self.token: str | None = None
        self.golpe_map = {}  # (nombre, extremidad, posicion) -> id_golpe
        self._correo = correo
        self._contrasena = contrasena

    def _headers(self) -> dict:
        h = {"Content-Type": "application/json"}
        if self.token:
            h["Authorization"] = f"Bearer {self.token}"
        return h

    def login(self, correo: str | None = None, contrasena: str | None = None) -> dict:
        """Autentica y almacena el token JWT."""
        c = correo or self._correo
        p = contrasena or self._contrasena
        if not c or not p:
            raise ValueError("Se requiere correo y contrasena para login")

        resp = requests.post(
            f"{self.base_url}/login",
            json={"correo": c, "contrasena": p},
            timeout=10,
        )
        resp.raise_for_status()
        data = resp.json()
        self.token = data["token"]
        return data

    def fetch_golpes(self) -> dict:
        """Descarga la tabla GOLPE y construye un mapa de búsqueda."""
        resp = requests.get(
            f"{self.base_url}/golpes",
            headers=self._headers(),
            timeout=10,
        )
        resp.raise_for_status()
        golpes = resp.json()
        self.golpe_map = {}
        for g in golpes:
            key = (
                g["nombre"].lower(),
                (g["extremidad"] or "").lower(),
                (g["posicion"] or "").lower(),
            )
            self.golpe_map[key] = g["id_golpe"]
        return self.golpe_map

    @staticmethod
    def _parse_label(label: str) -> tuple[str, str, str]:
        """Convierte etiqueta ML en (nombre, extremidad, posicion)."""
        parts = label.lower().split("_")
        if len(parts) < 2:
            return label.capitalize(), "Derecha", "Cabeza"

        punch_type = parts[0]
        if punch_type == "jab":
            nombre = "Jab"
        elif punch_type == "cross":
            nombre = "Cross"
        elif punch_type == "hook":
            nombre = "Gancho"
        elif punch_type == "uppercut":
            nombre = "Upper"
        else:
            nombre = punch_type.capitalize()

        pos_str = "_".join(parts[1:])
        if "izquierda" in pos_str:
            extremidad = "Izquierda"
        elif "derecha" in pos_str:
            extremidad = "Derecha"
        else:
            extremidad = "Derecha"

        if "arriba" in pos_str:
            posicion = "Cabeza"
        else:
            posicion = "Cuerpo"

        return nombre, extremidad, posicion

    def map_prediction_to_golpe(self, pred_label: str) -> int | None:
        """Devuelve el id_golpe para una etiqueta ML, o None."""
        nombre, extremidad, posicion = self._parse_label(pred_label)
        key = (nombre.lower(), extremidad.lower(), posicion.lower())
        return self.golpe_map.get(key)

    def create_entrenamiento(
        self, id_usuario: int = 1, tipo: str = "Estandar"
    ) -> dict:
        """Crea un nuevo entrenamiento."""
        payload = {
            "hora_inicio": datetime.datetime.now(datetime.timezone.utc).isoformat(),
            "hora_fin": None,
            "tipo": tipo,
            "calorias": None,
            "id_usuario": id_usuario,
        }
        resp = requests.post(
            f"{self.base_url}/entrenamientos",
            json=payload,
            headers=self._headers(),
            timeout=10,
        )
        resp.raise_for_status()
        return resp.json()

    def finish_entrenamiento(self, id_entrenamiento: int) -> dict:
        """Marca la hora_fin de un entrenamiento."""
        payload = {
            "hora_fin": datetime.datetime.now(datetime.timezone.utc).isoformat()
        }
        resp = requests.put(
            f"{self.base_url}/entrenamientos/{id_entrenamiento}",
            json=payload,
            headers=self._headers(),
            timeout=10,
        )
        resp.raise_for_status()
        return resp.json()

    def create_historial(
        self, id_entrenamiento: int, id_golpe: int, potencia: float | None = None
    ) -> dict:
        """Registra un golpe en el historial."""
        payload = {
            "id_entrenamiento": id_entrenamiento,
            "id_golpe": id_golpe,
            "potencia": potencia,
        }
        resp = requests.post(
            f"{self.base_url}/historial",
            json=payload,
            headers=self._headers(),
            timeout=10,
        )
        resp.raise_for_status()
        return resp.json()

    async def connect_ws(self):
        """Abre la conexión WebSocket con el token en el header."""
        extra_headers = {}
        if self.token:
            extra_headers["Authorization"] = f"Bearer {self.token}"
        self.ws = await websockets.connect(self.ws_url, extra_headers=extra_headers)

    async def send_ws(self, data: dict):
        """Envía un mensaje JSON por WebSocket."""
        if self.ws:
            await self.ws.send(json.dumps(data))

    async def close_ws(self):
        """Cierra la conexión WebSocket."""
        if self.ws:
            await self.ws.close()
            self.ws = None

    def __enter__(self):
        if not self.token:
            self.login()
        self.fetch_golpes()
        return self

    def __exit__(self, *args):
        if self.ws:
            asyncio.get_event_loop().run_until_complete(self.close_ws())
