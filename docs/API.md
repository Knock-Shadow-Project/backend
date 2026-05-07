# Documentación de la API — KnockShadow

URL base: `http://localhost:3000`

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
- [USUARIO](#usuario)
- [ENTRENAMIENTO](#entrenamiento)
- [GOLPE](#golpe)
- [HISTORIAL](#historial)

---

## Login

### Autenticar y obtener un JWT

```
POST /login
Content-Type: application/json

{
  "correo": "admin@example.com",
  "contrasena": "2admin1"
}
```

**Respuesta de éxito (200):**
```json
{
  "token": "eyJhbGciOiJIUzI1NiIs...",
  "id_usuario": 1,
  "nombre": "Admin",
  "correo": "admin@example.com"
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
  "nombre": "Juan",
  "apellido": "Pérez",
  "correo": "juan@example.com",
  "contrasena": "secreto123",
  "telefono": null,
  "edad": null,
  "peso": null,
  "estatura": null,
  "pais": null,
  "ciudad": null,
  "direccion": null,
  "lateralidad": null,
  "nivel": null
}
```

**Respuesta de éxito (200):**
```json
{
  "token": "eyJhbGciOiJIUzI1NiIs...",
  "id_usuario": 42,
  "nombre": "Juan",
  "correo": "juan@example.com"
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
    {"clase": "jab_izquierda_arriba", "prob": 0.95},
    {"clase": "cross_derecha_cabeza", "prob": 0.03}
  ]
}
```

**Conexión con `wscat`:**
```bash
npx wscat -c ws://localhost:3000/ws -H "Authorization: Bearer <token>"
```

---

## Modelos

### Usuario
```json
{
  "id_usuario": 1,
  "nombre": "Admin",
  "apellido": "Admin",
  "correo": "admin@example.com",
  "telefono": "+34123456789",
  "edad": 30,
  "peso": "70.50",
  "estatura": 175,
  "pais": "España",
  "ciudad": "Madrid",
  "direccion": "Calle Principal 123",
  "lateralidad": "Derecho",
  "nivel": "Intermedio"
}
```

### Entrenamiento
```json
{
  "id_entrenamiento": 1,
  "hora_inicio": "2024-06-01T10:00:00",
  "hora_fin": "2024-06-01T11:00:00",
  "tipo": "Estandar",
  "calorias": 500,
  "id_usuario": 1
}
```

### Golpe
```json
{
  "id_golpe": 1,
  "nombre": "Jab",
  "extremidad": "Derecha",
  "posicion": "Cabeza"
}
```

### Historial
```json
{
  "id_entrenamiento": 1,
  "id_golpe": 1,
  "potencia": "75.50"
}
```

### HistorialDetail
Igual que `Historial` más:
```json
{
  "nombre": "Jab",
  "extremidad": "Derecha",
  "posicion": "Cabeza"
}
```

---

## USUARIO

Todos los endpoints de `USUARIO` requieren autenticación Bearer.

### Listar todos los usuarios
```
GET /usuarios
Authorization: Bearer <token>
```

### Obtener un usuario
```
GET /usuarios/:id
Authorization: Bearer <token>
```

### Crear un usuario
```
POST /usuarios
Authorization: Bearer <token>
Content-Type: application/json

{
  "nombre": "John",
  "apellido": "Doe",
  "correo": "john@example.com",
  "contrasena": "secret",
  "telefono": null,
  "edad": null,
  "peso": null,
  "estatura": null,
  "pais": null,
  "ciudad": null,
  "direccion": null,
  "lateralidad": null,
  "nivel": null
}
```

### Actualizar un usuario
```
PUT /usuarios/:id
Authorization: Bearer <token>
Content-Type: application/json

{
  "nombre": "Jane"
}
```

### Eliminar un usuario
```
DELETE /usuarios/:id
Authorization: Bearer <token>
```

---

## ENTRENAMIENTO

Todos los endpoints de `ENTRENAMIENTO` requieren autenticación Bearer.

### Listar todos los entrenamientos
```
GET /entrenamientos
Authorization: Bearer <token>
```

### Obtener un entrenamiento
```
GET /entrenamientos/:id
Authorization: Bearer <token>
```

### Crear un entrenamiento
```
POST /entrenamientos
Authorization: Bearer <token>
Content-Type: application/json

{
  "hora_inicio": "2024-06-01T10:00:00",
  "hora_fin": null,
  "tipo": "Estandar",
  "calorias": null,
  "id_usuario": 1
}
```

### Actualizar un entrenamiento
```
PUT /entrenamientos/:id
Authorization: Bearer <token>
Content-Type: application/json

{
  "hora_fin": "2024-06-01T11:00:00",
  "calorias": 500
}
```

### Eliminar un entrenamiento
```
DELETE /entrenamientos/:id
Authorization: Bearer <token>
```

### Listar entrenamientos por usuario
```
GET /usuarios/:id/entrenamientos
Authorization: Bearer <token>
```

---

## GOLPE

Todos los endpoints de `GOLPE` requieren autenticación Bearer.

### Listar todos los golpes
```
GET /golpes
Authorization: Bearer <token>
```

### Obtener un golpe
```
GET /golpes/:id
Authorization: Bearer <token>
```

### Crear un golpe
```
POST /golpes
Authorization: Bearer <token>
Content-Type: application/json

{
  "nombre": "Jab",
  "extremidad": "Derecha",
  "posicion": "Cabeza"
}
```

### Actualizar un golpe
```
PUT /golpes/:id
Authorization: Bearer <token>
Content-Type: application/json

{
  "posicion": "Cuerpo"
}
```

### Eliminar un golpe
```
DELETE /golpes/:id
Authorization: Bearer <token>
```

---

## HISTORIAL

Todos los endpoints de `HISTORIAL` requieren autenticación Bearer.

### Listar todo el historial (con detalles del golpe)
```
GET /historial
Authorization: Bearer <token>
```

### Obtener una entrada del historial
```
GET /historial/:id_entrenamiento/:id_golpe
Authorization: Bearer <token>
```

### Crear una entrada del historial
```
POST /historial
Authorization: Bearer <token>
Content-Type: application/json

{
  "id_entrenamiento": 1,
  "id_golpe": 1,
  "potencia": "75.50"
}
```

### Actualizar una entrada del historial
```
PUT /historial/:id_entrenamiento/:id_golpe
Authorization: Bearer <token>
Content-Type: application/json

{
  "potencia": "80.00"
}
```

### Eliminar una entrada del historial
```
DELETE /historial/:id_entrenamiento/:id_golpe
Authorization: Bearer <token>
```

### Listar historial por entrenamiento
```
GET /entrenamientos/:id/historial
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
