# Documentación de la API — KnockShadow

Base URL: `http://localhost:3000`

---

## Autenticación

Todos los endpoints **excepto** `POST /login` y `POST /register` requieren un token JWT válido en la cabecera `Authorization`.

```
Authorization: Bearer <token>
```

Para obtener un token, usa el endpoint de login. El token expira tras **24 horas**.

---

## Índice

- [Autenticación](#autenticación)
- [Login](#login)
- [Registro](#registro)
- [WebSocket](#websocket)
- [Metrics](#metrics)
- [Users](#users)
- [Trainings](#trainings)
- [Punches](#punches)
- [History](#history)

---

## Login

### Autenticar y obtener un JWT

```
POST /login
Content-Type: application/json

{
  "email": "admin@example.com",
  "password": "2admin1"
}
```

**Respuesta de éxito (200):**
```json
{
  "token": "eyJhbGciOiJIUzI1NiIs...",
  "user_id": 1,
  "first_name": "Admin",
  "email": "admin@example.com"
}
```

**Respuestas de error:**
- `401 Unauthorized` — Correo o contraseña incorrectos
- `500 Internal Server Error` — Error de base de datos o codificación JWT

---

## Registro

### Crear un nuevo usuario y obtener un JWT

```
POST /register
Content-Type: application/json

{
  "first_name": "John",
  "last_name": "Doe",
  "email": "john@example.com",
  "password": "secret123",
  "phone": null,
  "age": null,
  "weight": null,
  "height": null,
  "country": null,
  "city": null,
  "address": null,
  "laterality": null,
  "level": null
}
```

**Respuesta de éxito (200):**
```json
{
  "token": "eyJhbGciOiJIUzI1NiIs...",
  "user_id": 42,
  "first_name": "John",
  "email": "john@example.com"
}
```

**Respuestas de error:**
- `500 Internal Server Error` — Error de base de datos o codificación JWT

---

## WebSocket

**Endpoint:** `ws://localhost:3000/ws`

> Requiere autenticación Bearer mediante la cabecera `Authorization` durante el handshake del WebSocket.

El servidor retransmite cualquier mensaje de texto recibido a todos los clientes conectados. Esto permite el streaming en tiempo real de los resultados de detección de golpes desde el pipeline de inferencia de ML.

**Ejemplo de mensaje (enviado por el cliente ML):**
```json
{
  "type": "punch_detected",
  "timestamp": "14:32:10.123",
  "predictions": [
    {"class": "jab_izquierda_arriba", "prob": 0.95},
    {"class": "cross_derecha_cabeza", "prob": 0.03}
  ]
}
```

**Conexión con `wscat`:**
```bash
npx wscat -c ws://localhost:3000/ws -H "Authorization: Bearer <token>"
```

---

## Metrics

### Métricas Prometheus

```
GET /metrics
```

Endpoint expuesto por `axum-prometheus`. Devuelve el formato estándar
text/plain con histogramas de latencia HTTP, contadores por status y por
ruta. **No requiere autenticación** — está intencionadamente fuera del
middleware `auth` para que el scraper de Prometheus pueda leerlo sin JWT.

Si el endpoint se publica en internet, restringirlo por IP o auth básico en
el reverse proxy (Nginx / Traefik / Caddy), nunca abriéndolo a todo el
mundo.

**Ejemplo de respuesta (truncada):**
```
# HELP axum_http_requests_duration_seconds Latency of HTTP requests
# TYPE axum_http_requests_duration_seconds histogram
axum_http_requests_duration_seconds_bucket{method="GET",path="/users",status="200",le="0.005"} 12
…
# HELP axum_http_requests_total Total number of HTTP requests
# TYPE axum_http_requests_total counter
axum_http_requests_total{method="POST",path="/login",status="200"} 4
```

Prometheus y Grafana ya vienen configurados en `docker-compose.yaml`
(servicios `prometheus` y `grafana`); los dashboards se aprovisionan desde
`grafana/provisioning/`.

---

## Modelos

### Usuario
```json
{
  "user_id": 1,
  "first_name": "Admin",
  "last_name": "Admin",
  "email": "admin@example.com",
  "phone": "+34123456789",
  "age": 30,
  "weight": "70.50",
  "height": 175,
  "country": "Spain",
  "city": "Madrid",
  "address": "Main Street 123",
  "laterality": "Right",
  "level": "Intermediate"
}
```

### Entrenamiento
```json
{
  "training_id": 1,
  "start_time": "2024-06-01T10:00:00",
  "end_time": "2024-06-01T11:00:00",
  "training_type": "Standard",
  "calories": 500,
  "user_id": 1
}
```

### Golpe
```json
{
  "punch_id": 1,
  "name": "Jab",
  "limb": "Right",
  "position": "Head"
}
```

### Historial
```json
{
  "training_id": 1,
  "punch_id": 1,
  "power": "75.50"
}
```

### HistorialDetail
Igual que `Historial` más:
```json
{
  "name": "Jab",
  "limb": "Right",
  "position": "Head"
}
```

---

## Users

Todos los endpoints de `USER` requieren autenticación Bearer.

### Listar todos los usuarios
```
GET /users
Authorization: Bearer <token>
```

### Get a user
```
GET /users/:id
Authorization: Bearer <token>
```

### Crear un usuario
```
POST /users
Authorization: Bearer <token>
Content-Type: application/json

{
  "first_name": "John",
  "last_name": "Doe",
  "email": "john@example.com",
  "password": "secret",
  "phone": null,
  "age": null,
  "weight": null,
  "height": null,
  "country": null,
  "city": null,
  "address": null,
  "laterality": null,
  "level": null
}
```

### Actualizar un usuario
```
PUT /users/:id
Authorization: Bearer <token>
Content-Type: application/json

{
  "first_name": "Jane"
}
```

### Eliminar un usuario
```
DELETE /users/:id
Authorization: Bearer <token>
```

---

## Trainings

Todos los endpoints de `TRAINING` requieren autenticación Bearer.

### Listar todos los entrenamientos
```
GET /trainings
Authorization: Bearer <token>
```

### Obtener un entrenamiento
```
GET /trainings/:id
Authorization: Bearer <token>
```

### Crear un entrenamiento
```
POST /trainings
Authorization: Bearer <token>
Content-Type: application/json

{
  "start_time": "2024-06-01T10:00:00",
  "end_time": null,
  "training_type": "Standard",
  "calories": null,
  "user_id": 1
}
```

### Actualizar un entrenamiento
```
PUT /trainings/:id
Authorization: Bearer <token>
Content-Type: application/json

{
  "end_time": "2024-06-01T11:00:00",
  "calories": 500
}
```

### Eliminar un entrenamiento
```
DELETE /trainings/:id
Authorization: Bearer <token>
```

### Listar entrenamientos por usuario
```
GET /users/:id/trainings
Authorization: Bearer <token>
```

---

## Punches

Todos los endpoints de `PUNCH` requieren autenticación Bearer.

### List all punches
```
GET /punches
Authorization: Bearer <token>
```

### Obtener un golpe
```
GET /punches/:id
Authorization: Bearer <token>
```

### Crear un golpe
```
POST /punches
Authorization: Bearer <token>
Content-Type: application/json

{
  "name": "Jab",
  "limb": "Right",
  "position": "Head"
}
```

### Actualizar un golpe
```
PUT /punches/:id
Authorization: Bearer <token>
Content-Type: application/json

{
  "position": "Body"
}
```

### Eliminar un golpe
```
DELETE /punches/:id
Authorization: Bearer <token>
```

---

## History

Todos los endpoints de `HISTORY` requieren autenticación Bearer.

### Listar todo el historial (con detalles del golpe)
```
GET /history
Authorization: Bearer <token>
```

### Obtener una entrada del historial
```
GET /history/:training_id/:punch_id
Authorization: Bearer <token>
```

### Crear una entrada del historial
```
POST /history
Authorization: Bearer <token>
Content-Type: application/json

{
  "training_id": 1,
  "punch_id": 1,
  "power": "75.50"
}
```

### Actualizar una entrada del historial
```
PUT /history/:training_id/:punch_id
Authorization: Bearer <token>
Content-Type: application/json

{
  "power": "80.00"
}
```

### Eliminar una entrada del historial
```
DELETE /history/:training_id/:punch_id
Authorization: Bearer <token>
```

### Listar historial por entrenamiento
```
GET /trainings/:id/history
Authorization: Bearer <token>
```

---

## Variables de entorno

| Variable | Por defecto | Descripción |
|----------|-------------|-------------|
| `DATABASE_URL` | `postgres://knockshadow:knockshadow@127.0.0.1:5432/knockshadow` | Cadena de conexión a PostgreSQL |
| `PORT` | `3000` | Puerto del servidor HTTP |
| `JWT_SECRET` | `knockshadow_default_secret_change_me` | Clave secreta para firmar JWT (**cambiar en producción**) |

---

## Códigos de respuesta

| Estado | Significado |
|--------|-------------|
| `200 OK` | Éxito |
| `201 Created` | Recurso creado |
| `204 No Content` | Recurso eliminado |
| `401 Unauthorized` | Falta token Bearer o credenciales inválidas |
| `404 Not Found` | Recurso no encontrado |
| `500 Internal Server Error` | Error de base de datos o del servidor |
