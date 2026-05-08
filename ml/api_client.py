"""Cliente para interactuar con la API de KnockShadow."""

import asyncio
import datetime
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
        email: str = "",
        password: str = "",
    ):
        self.base_url = base_url.rstrip("/")
        self.ws_url = ws_url
        self.ws = None
        self.token: str | None = None
        self.punch_map = {}  # (name, limb, position) -> punch_id
        self._email = email
        self._password = password

    def _headers(self) -> dict:
        h = {"Content-Type": "application/json"}
        if self.token:
            h["Authorization"] = f"Bearer {self.token}"
        return h

    def login(self, email: str | None = None, password: str | None = None) -> dict:
        """Autentica y almacena el token JWT."""
        c = email or self._email
        p = password or self._password
        if not c or not p:
            raise ValueError("Se requiere email y password para login")

        resp = requests.post(
            f"{self.base_url}/login",
            json={"email": c, "password": p},
            timeout=10,
        )
        resp.raise_for_status()
        data = resp.json()
        self.token = data["token"]
        return data

    def fetch_punches(self) -> dict:
        """Descarga la tabla PUNCH y construye un mapa de búsqueda."""
        resp = requests.get(
            f"{self.base_url}/punches",
            headers=self._headers(),
            timeout=10,
        )
        resp.raise_for_status()
        punches = resp.json()
        self.punch_map = {}
        for p in punches:
            key = (
                p["name"].lower(),
                (p["limb"] or "").lower(),
                (p["position"] or "").lower(),
            )
            self.punch_map[key] = p["punch_id"]
        return self.punch_map

    @staticmethod
    def _parse_label(label: str) -> tuple[str, str, str]:
        """Convierte etiqueta ML en (name, limb, position)."""
        parts = label.lower().split("_")
        if len(parts) < 2:
            return label.capitalize(), "Derecha", "Cabeza"

        punch_type = parts[0]
        if punch_type == "jab":
            name = "Jab"
        elif punch_type == "cross":
            name = "Cross"
        elif punch_type == "hook":
            name = "Gancho"
        elif punch_type == "uppercut":
            name = "Upper"
        else:
            name = punch_type.capitalize()

        pos_str = "_".join(parts[1:])
        if "izquierda" in pos_str:
            limb = "Izquierda"
        elif "derecha" in pos_str:
            limb = "Derecha"
        else:
            limb = "Derecha"

        if "arriba" in pos_str:
            position = "Cabeza"
        else:
            position = "Cuerpo"

        return name, limb, position

    def map_prediction_to_punch(self, pred_label: str) -> int | None:
        """Devuelve el punch_id para una etiqueta ML, o None."""
        name, limb, position = self._parse_label(pred_label)
        key = (name.lower(), limb.lower(), position.lower())
        return self.punch_map.get(key)

    def create_training(
        self, user_id: int = 1, training_type: str = "Estandar"
    ) -> dict:
        """Crea un nuevo entrenamiento."""
        payload = {
            "start_time": datetime.datetime.now(datetime.timezone.utc).isoformat(),
            "end_time": None,
            "training_type": training_type,
            "calories": None,
            "user_id": user_id,
        }
        resp = requests.post(
            f"{self.base_url}/trainings",
            json=payload,
            headers=self._headers(),
            timeout=10,
        )
        resp.raise_for_status()
        return resp.json()

    def finish_training(self, training_id: int) -> dict:
        """Marca la end_time de un entrenamiento."""
        payload = {
            "end_time": datetime.datetime.now(datetime.timezone.utc).isoformat()
        }
        resp = requests.put(
            f"{self.base_url}/trainings/{training_id}",
            json=payload,
            headers=self._headers(),
            timeout=10,
        )
        resp.raise_for_status()
        return resp.json()

    def create_history(
        self, training_id: int, punch_id: int, power: float | None = None
    ) -> dict:
        """Registra un golpe en el historial."""
        payload = {
            "training_id": training_id,
            "punch_id": punch_id,
            "power": power,
        }
        resp = requests.post(
            f"{self.base_url}/history",
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
        self.fetch_punches()
        return self

    def __exit__(self, *args):
        if self.ws:
            asyncio.get_event_loop().run_until_complete(self.close_ws())
