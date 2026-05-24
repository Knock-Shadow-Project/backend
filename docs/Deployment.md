# Deployment Guide — KnockShadow

Guía completa de despliegue: qué piezas van al **cloud** y qué piezas van a la **Raspberry Pi**.

---

## Arquitectura desplegada

```
┌─────────────────────────────────────────────────────────────────────────┐
│                              CLOUD / VPS                                │
│                   ┌──────────────┐  ┌──────────────┐                    │
│                   │  PostgreSQL  │  │   api-db     │                    │
│                   │   (DB)       │  │  (REST+WS)   │                    │
│                   │  :5432       │  │   :3000      │                    │
│                   └──────────────┘  └──────┬───────┘                    │
│                                            │                            │
│                    https://api.tudominio.com:3000                       │
└─────────────────────────────────────────────────────────────────────────┘
                                    ▲
                                    │ sync (cuando hay internet)
                                    │
┌─────────────────────────────────────────────────────────────────────────┐
│                        RED LOCAL / WIFI                                 │
│                                                                         │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │                    RASPBERRY PI 4                               │    │
│  │  ┌─────────────┐   BLE   ┌──────────────┐   MQTT    ┌──────┐   │    │
│  │  │  Sensores   │ ───────►│  pi-service  │◄─────────│pi_inf│   │    │
│  │  │   (2x)      │         │   (Rust)     │  nanomq  │(Py)  │   │    │
│  │  └─────────────┘         └──────┬───────┘          └──────┘   │    │
│  │                                 │ HTTP/WS                       │    │
│  │                                 ▼                               │    │
│  │                          ┌─────────────┐                        │    │
│  │                          │  App Móvil  │                        │    │
│  │                          │ (mDNS desc) │                        │    │
│  │                          └─────────────┘                        │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│                                                                         │
│   mDNS: _knockshadow._tcp.local.  →  knockshadow-pi.local.              │
│   API local: http://<ip-pi>:8080                                        │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## ¿Qué va dónde?

| Pieza | Destino | ¿Por qué? |
|-------|---------|-----------|
| **api-db** | Cloud / VPS | API REST central con PostgreSQL. Debe ser accesible desde internet. |
| **PostgreSQL** | Cloud / VPS | Base de datos central donde convergen los datos sincronizados desde la Pi. |
| **ml/app.py** | Local | Streamlit para etiquetar muestras de entrenamiento (usar para entrenar la CNN). |
| **pi-service** | Raspberry Pi 4 | Servicio offline-first: BLE → SQLite → API local + mDNS. |
| **pi_inference.py** | Raspberry Pi 4 | CNN offline que lee SQLite local y detecta golpes en tiempo real. |

> **Nota:** `ble-stream` y `ml/main.py` Se usaron en local para el dasarrollo.

> **Regla de oro:** La Raspberry Pi funciona **autónoma** sin internet. Solo sincroniza con el cloud cuando tiene conexión y `API_BASE_URL` está configurado.

---

## Despliegue Cloud

### Requisitos

- Servidor/VPS con Docker y Docker Compose instalados.

### Paso 1: Clonar y levantar

```bash
git clone <repo>
cd backend

# Configura variables
cp .env.example .env
# edita .env con tu JWT_SECRET, etc.

docker compose -f docker-compose.yaml up --build -d
```

### Paso 2: Verificar

```bash
# API
curl http://localhost:3000/

# PostgreSQL (desde otro contenedor)
docker exec -it knockshadow-db psql -U knockshadow -d knockshadow

# Streamlit (para etiquetar datos)
open http://localhost:8501
```

### Paso 3: Exponer a internet

- Usa un reverse proxy (Nginx, Traefik, Caddy) para HTTPS.
- Ejemplo con Caddy:
  ```
  api.tudominio.com {
      reverse_proxy localhost:3000
  }
  ```

---

## Despliegue Raspberry Pi

### Requisitos

- Raspberry Pi 4 con Raspberry Pi OS (64-bit recomendado).
- Docker y Docker Compose instalados.
- `bluetoothd` activo:
  ```bash
  sudo systemctl start bluetooth
  sudo systemctl enable bluetooth
  ```
- Sensores BLE emparejados (o al menos conocidas sus MACs).

### Paso 1: Configurar variables de entorno

Crea un archivo `.env.pi` en la Raspberry:

```bash
# Bluetooth
DEVICE_MAC_1=DF:65:81:D0:D7:E5
DEVICE_MAC_2=CB:01:10:3E:0D:61

# Identidad del usuario (se puede sobreescribir desde la app)
USER_ID=1

# Sync remoto (opcional)
API_BASE_URL=http://api.tudominio.com:3000

# mDNS
MDNS_HOSTNAME=knockshadow-pi
```

### Paso 2: Levantar el stack

```bash
cd backend

# Cargar variables
export $(cat .env.pi | xargs)

# Levantar
docker compose -f docker-compose.pi.yaml up --build -d
```

### Paso 3: Verificar

```bash
# Logs de pi-service
docker logs -f knockshadow-pi-service

# Logs de la CNN
docker logs -f knockshadow-pi-inference

# Probar API local desde otra máquina en la misma red
curl http://knockshadow-pi.local:8080/
curl http://knockshadow-pi.local:8080/training/active
```

### Paso 4: App móvil

1. Conecta el móvil a la **misma red WiFi** que la Raspberry.
2. La app descubre automáticamente `knockshadow-pi.local.` via mDNS.
3. Se conecta a `http://<ip>:8080`.
4. Inicia entrenamiento con `POST /training/start`.
5. Recibe golpes en tiempo real por `WS /live`.
6. Al finalizar: `POST /training/stop`.

---

## Compilación cruzada (desde PC hacia Raspberry Pi)

Si tu PC es x86_64 y la Raspberry es ARM64, compila con Docker Buildx:

```bash
# Activar buildx (una sola vez)
docker buildx create --use

# Compilar pi-service
docker buildx build \
  --platform linux/arm64 \
  -f Dockerfile.pi-service \
  -t tu-registry/knockshadow/pi-service:latest \
  --push .

# Compilar pi-inference
docker buildx build \
  --platform linux/arm64 \
  -f Dockerfile.pi-inference \
  -t tu-registry/knockshadow/pi-inference:latest \
  --push .
```

Luego, en la Raspberry Pi:

```bash
docker pull tu-registry/knockshadow/pi-service:latest
docker pull tu-registry/knockshadow/pi-inference:latest

# Edita docker-compose.pi.yaml para usar las imágenes pre-compiladas
# en lugar de build: context...

docker compose -f docker-compose.pi.yaml up -d
```

---

## Variables de entorno por entorno

### Cloud (`docker-compose.yaml`)

| Variable | Servicio | Valor típico |
|----------|----------|--------------|
| `DATABASE_URL` | api-db, ml-app | `postgres://knockshadow:knockshadow@db:5432/knockshadow` |
| `JWT_SECRET` | api-db | Cambiar en producción |
| `PORT` | api-db | `3000` |

### Raspberry Pi (`docker-compose.pi.yaml`)

| Variable | Servicio | Valor típico |
|----------|----------|--------------|
| `DB_PATH` | pi-service, pi-inference | `/data/pi_data.db` |
| `PORT` | pi-service | `8080` |
| `DEVICE_MAC_1` | pi-service | MAC sensor izquierdo |
| `DEVICE_MAC_2` | pi-service | MAC sensor derecho |
| `SENSOR_MAC_1` | pi-inference | Igual que `DEVICE_MAC_1` |
| `SENSOR_MAC_2` | pi-inference | Igual que `DEVICE_MAC_2` |
| `MQTT_HOST` | pi-service, pi-inference | `127.0.0.1` (broker nanomq) |
| `MQTT_PORT` | pi-service, pi-inference | `1883` |
| `API_BASE_URL` | pi-service | `http://api.tudominio.com:3000` (opcional) |
| `MDNS_HOSTNAME` | pi-service | `knockshadow-pi` |
| `USER_ID` | pi-inference | `1` |

---

## Consejos de red

### mDNS no funciona en algunas redes

Si la app móvil no descubre `knockshadow-pi.local.`:

1. Asegúrate de que el móvil y la Raspberry están en la **misma red WiFi**.
2. Algunos routers bloquean mDNS (multicast). Prueba con la IP directa:
   ```bash
   # Descubre la IP de la Raspberry
   hostname -I
   ```
3. Alternativa: asigna una IP estática a la Raspberry y configúrala manualmente en la app.

### Sync remoto con HTTPS

Si tu API cloud usa HTTPS con certificado autofirmado, `reqwest` en `pi-service` podría rechazarlo. Opciones:

- Usa un certificado real (Let's Encrypt).
- O configura `reqwest` para aceptar certificados no verificados (no recomendado en producción).

### Firewall

En el cloud, abre los puertos necesarios:
- `3000/tcp` — API REST + WebSocket
- `5432/tcp` — PostgreSQL (solo si necesitas acceso externo; mejor dejarlo en red interna de Docker)
- `80/tcp` y `443/tcp` — Reverse proxy

En la Raspberry, no necesitas abrir puertos en el firewall local si usas `network_mode: host` y estás en la misma red WiFi.

---

## Troubleshooting

### `pi-service` no detecta los sensores BLE

```bash
# Verificar que bluetoothd está activo
sudo systemctl status bluetooth

# Escanear manualmente
bluetoothctl scan on

# Verificar MACs
bluetoothctl devices
```

### `pi-inference` no detecta entrenamiento activo

1. Asegúrate de que `pi-service` está corriendo y creó un entrenamiento vía `POST /training/start`.
2. Verifica que ambos contenedores comparten el mismo volumen `pi-data`.
3. Revisa los logs:
   ```bash
   docker logs knockshadow-pi-inference
   ```

### La app móvil no recibe datos por WebSocket

1. Verifica que `pi-inference` publica punches por MQTT:
   ```bash
   # Suscribir al topic de punches para ver mensajes en tiempo real
   docker exec knockshadow-nanomq nanomq_cli sub -t "knockshadow/punches"
   ```
2. Verifica que `pi-service` recibe los punches vía MQTT:
   ```bash
   # En los logs de pi-service deberías ver conexiones WS y MQTT started
   docker logs -f knockshadow-pi-service
   ```
3. Verifica punches persistidos en SQLite (para sync):
   ```bash
   docker exec knockshadow-pi-service sqlite3 /data/pi_data.db \
     "SELECT COUNT(*) FROM detected_punches;"
   ```

---

## Checklist de despliegue

### Cloud
- [ ] Servidor con Docker y Docker Compose
- [ ] Puerto 3000 accesible (o detrás de reverse proxy con HTTPS)
- [ ] `JWT_SECRET` cambiado de valor por defecto
- [ ] PostgreSQL con volumen persistente

### Raspberry Pi
- [ ] Raspberry Pi 4 con Raspberry Pi OS 64-bit
- [ ] Docker y Docker Compose instalados
- [ ] `bluetoothd` activo y funcionando
- [ ] MACs de los sensores conocidas
- [ ] Misma red WiFi que la app móvil
- [ ] Volumen `pi-data` persistente
- [ ] (Opcional) `API_BASE_URL` configurado para sync
