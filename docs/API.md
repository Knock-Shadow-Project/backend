# Documentación de la API — KnockShadow

Base URL: `http://localhost:3000`

Todos los modelos serializan en JSON con campos en **inglés** (`user_id`,
`training_id`…). Las columnas en PostgreSQL siguen en español
(`id_usuario`, `id_entrenamiento`…) y se mapean en el código vía
`#[sqlx(rename = "...")]`. Esta dualidad existe porque la app móvil
consume nombres en inglés mientras que el modelo entidad-relación del
proyecto académico está en castellano.

---

## Índice

- [Autenticación](#autenticación)
- [Convenciones comunes](#convenciones-comunes)
  - [Paginación](#paginación)
  - [Errores](#errores)
- [Login](#login)
- [Registro](#registro)
- [Reenvío de confirmación](#reenvío-de-confirmación)
- [WebSocket](#websocket)
- [Metrics](#metrics)
- [Modelos](#modelos)
- [Users](#users)
- [Trainings](#trainings)
- [Routines](#routines)
- [Punches](#punches)
- [History](#history)
- [Variables de entorno](#variables-de-entorno)
- [Códigos de respuesta](#códigos-de-respuesta)

---

## Autenticación

Todos los endpoints **excepto** `POST /login`, `POST /register` y
`GET /metrics` requieren un token JWT válido en la cabecera
`Authorization`:

```
Authorization: Bearer <token>
```

Para obtener un token, usa el endpoint de login o registro. El token:

- Está firmado con `HS256` y la clave de `JWT_SECRET` (mínimo 32 chars).
- Expira tras **24 horas** desde la emisión (`exp` en los claims).
- Incluye `sub` (`user_id`) y `email` en los claims.

> ⚠️ **Seed admin**: el usuario `admin@example.com` se inserta vía el SQL
> de init con la contraseña en texto plano (`'2admin1'`), por lo que el
> `POST /login` con esas credenciales falla — bcrypt no la valida. Para
> autenticar como admin, hay que volver a poner la contraseña pasando por
> el endpoint `PUT /users/1` (o `POST /register` con un usuario nuevo).
> En despliegues nuevos conviene re-hashear el seed.

---

## Convenciones comunes

### Paginación

Todos los endpoints `GET` que devuelven listas aceptan dos query params:

| Param   | Tipo  | Default | Rango       | Notas |
|---------|-------|---------|-------------|-------|
| `limit` | `i64` | `50`    | `[1, 200]`  | Se recorta al límite superior automáticamente. |
| `offset`| `i64` | `0`     | `>= 0`      | Valores negativos se elevan a 0. |

Ejemplo:

```
GET /history?limit=20&offset=40
```

El servidor no devuelve metadatos de paginación en la respuesta; los
clientes deben llevar el cursor `offset += len(items)` hasta recibir un
array vacío o más corto que `limit`.

### Errores

| Status | Cuándo se emite |
|--------|------------------|
| `200 OK` | Operación de lectura/escritura exitosa. |
| `201 Created` | Reservado por convención HTTP; la API actual devuelve `200` también en crear. |
| `204 No Content` | `DELETE` exitoso. |
| `400 Bad Request` | (No actualmente generado) — payloads malformados producen el error nativo de Axum 422. |
| `401 Unauthorized` | Falta `Authorization` Bearer, token inválido/expirado, o credenciales incorrectas en `/login`. |
| `404 Not Found` | El recurso no existe (para `GET/PUT/DELETE` por ID). |
| `422 Unprocessable Entity` | Axum no pudo deserializar el JSON (falta campo obligatorio, tipo inválido…). |
| `500 Internal Server Error` | Error de BD, hash bcrypt, codificación JWT, etc. |

El cuerpo de error suele ir vacío; los detalles van a los logs del
servidor (`tracing::error!`). Esto evita filtrar información sensible al
cliente.

---

## Login

### Autenticar y obtener un JWT

```
POST /login
Content-Type: application/json

{
  "email": "user@example.com",
  "password": "secret123"
}
```

**Respuesta de éxito (200):**
```json
{
  "token": "eyJhbGciOiJIUzI1NiIs...",
  "user_id": 1,
  "first_name": "Admin",
  "email": "user@example.com"
}
```

**Respuestas de error:**
- `401 Unauthorized` — Correo no existe o contraseña incorrecta. Los dos
  casos se devuelven con el mismo status para no filtrar qué emails
  están registrados.
- `500 Internal Server Error` — Error de base de datos o codificación JWT.

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

Solo `first_name`, `last_name`, `email` y `password` son obligatorios.

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
- `422 Unprocessable Entity` — Falta algún campo obligatorio del payload.
- `500 Internal Server Error` — Email ya registrado (violación de
  `UNIQUE`), error de BD o de hash bcrypt.

---

## Reenvío de confirmación

### Solicitar un nuevo email de confirmación

```
POST /resend-confirmation
Content-Type: application/json

{
  "email": "user@example.com"
}
```

Endpoint **público** (no requiere JWT). Pensado para la pantalla
`confirmEmail.tsx` de la app móvil, donde el atleta aún no se ha
autenticado pero quiere volver a recibir el email de verificación.

**Respuesta de éxito (200):**
```json
{
  "status": "queued",
  "message": "Si el correo está registrado, recibirás un nuevo email en breve."
}
```

El mensaje es **genérico a propósito**: se devuelve igual exista o no el
correo en la base de datos, para no permitir enumeración de cuentas
válidas desde el endpoint público.

**Respuestas de error:**
- `400 Bad Request` — Payload sin `email` o formato visiblemente inválido
  (vacío, sin `@`).
- `429 Too Many Requests` — Mismo email pidió un reenvío hace menos de
  60 segundos. El cliente debe mostrar un countdown y volver a probar
  cuando expire.
- `500 Internal Server Error` — Error de BD o de firma JWT.

### Envío real vía Resend

El envío se hace con el SDK oficial `resend-rs` (crate `resend-rs = "0.24"`)
contra el dominio verificado en Resend. El handler ejecuta dos pasos:

1. Genera un JWT con `purpose=email_confirm`, mismo `JWT_SECRET` y TTL 24h.
2. Si `EmailService` está disponible en el `AppState`, lanza un
   `tokio::spawn` con `EmailService::send_confirmation(email, token)`.
   La respuesta HTTP devuelve `200` inmediatamente; el envío se completa
   en background. Cualquier error de Resend queda en logs como
   `tracing::error!`, pero **no se filtra** al cliente.

El email tiene cuerpo HTML (estilos inline, compatible con Gmail/Outlook/
Apple Mail) y un fallback `text/plain` para scoring antispam.

#### Variables de entorno requeridas

| Variable | Default | Notas |
|----------|---------|-------|
| `RESEND_API_KEY` | _(unset)_ | Clave de Resend (https://resend.com/api-keys). Si está vacía, el envío cae al **modo stub** (loggea el enlace en vez de mandarlo). |
| `RESEND_FROM` | `KnockShadow <no-reply@knockshadow.site>` | Buzón "From". Debe pertenecer a un dominio verificado en Resend o el envío fallará con 403. |
| `APP_BASE_URL` | `https://api.knockshadow.site` | Base de la URL de confirmación. El enlace final es `{APP_BASE_URL}/confirm-email?token=<token>`. |

> 🔐 **No commitees `RESEND_API_KEY` al repo.** El `.gitignore` ya
> excluye `.env`. Para producción usa Vault / Doppler / AWS Secrets
> Manager / variables de entorno del orquestador.

#### Modo stub (sin API key)

Si arrancas `api-db` sin `RESEND_API_KEY`, el log de arranque indica:

```
INFO RESEND_API_KEY ausente — emails en modo STUB (sólo log, sin envío real)
```

Cada solicitud loggea:

```
INFO [EMAIL STUB] Reenviar confirmación a user@example.com — token=eyJhbGciOi...
```

Útil para CI/dev sin tener que pegar la API key real.

#### Pendiente: confirmar el token

El endpoint que verifique el JWT y marque la cuenta como confirmada
todavía no existe. Su esqueleto obvio es:

```
GET /confirm-email?token=<token>
```

Decodificar el JWT, validar `purpose == "email_confirm"` y marcar la
columna `usuario.confirmado` (requiere migración nueva) a `TRUE`.

### Rate-limit y multi-pod

El throttle (60 s por email) sale del módulo `rate_limit` con dos
backends conmutables al arranque:

- **In-memory** (default): `HashMap` proceso-local. Suficiente para 1 pod.
- **Valkey / Redis**: cuando `REDIS_URL` está definido. Atómico vía
  `SET key 1 NX EX <secs>`. Funciona contra Valkey (servidor BSD-3 que
  arrancamos en `docker-compose.yaml` como `valkey/valkey:8-alpine`) o
  contra cualquier instancia Redis/RESP-compatible.

Fail-open: si Valkey está caído, el handler permite la petición y loggea.
Mejor que bloquear a todo el mundo por un blip de infra.

Resend tiene su propio rate-limit (10 req/s por API key) que el SDK
respeta internamente (9 req/1.1 s con buffer).

---

## Confirmación de correo

### Marcar una cuenta como confirmada desde el enlace del email

```
GET /confirm-email?token=<jwt>
```

Endpoint **público** que abre el atleta desde su cliente de correo. El
`token` es el JWT con `purpose=email_confirm` generado por
`POST /resend-confirmation` (o el del email automático de registro, si se
activa más adelante).

**Respuesta de éxito (200 OK):** página HTML estilizada (HUD táctico)
que muestra "¡Listo, atleta!" y ofrece un deep-link
`knockshadowfront://login` para abrir la app móvil.

**Respuestas de error (todas devuelven HTML):**
- `400 Bad Request` — token vacío, mal formado, expirado, o con
  `purpose` distinto de `email_confirm`. La página invita a pedir un
  nuevo enlace.
- `404 Not Found` — el email del token ya no existe en `usuario`
  (cuenta borrada entre que se generó el token y el clic).
- `500 Internal Server Error` — fallo de BD.

**Idempotente**: si la cuenta ya estaba confirmada, devuelve `200` con la
misma página de éxito (no es un error volver a hacer clic).

**Modelo afectado**: la migración `db/init/003_email_confirmation.sql`
añade `USUARIO.CONFIRMADO BOOLEAN NOT NULL DEFAULT FALSE`. El campo se
expone en el JSON del modelo `Usuario` como `confirmed: bool`. **No** se
puede setear vía `POST /register` ni `PUT /users/:id` — sólo cambia
desde este endpoint.

---

## WebSocket

**Endpoint:** `ws://localhost:3000/ws`

> Requiere autenticación Bearer mediante la cabecera `Authorization`
> durante el handshake del WebSocket. Los navegadores no permiten
> añadir cabeceras al `WebSocket` nativo, así que desde el frontend se
> usa la opción `protocols` o un proxy que añada la cabecera; los
> clientes nativos (CLI, móvil) la mandan directa.

El servidor mantiene un canal `broadcast::Sender<String>` global y
retransmite cualquier mensaje de texto/binary recibido a **todos** los
clientes conectados. Esto permite el streaming en tiempo real de los
resultados de detección de golpes desde el pipeline de inferencia ML
hacia las apps conectadas.

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
# TYPE axum_http_requests_total counter
axum_http_requests_total{method="GET",status="200",endpoint="/trainings"} 12
axum_http_requests_total{method="POST",status="200",endpoint="/login"} 4
# TYPE axum_http_requests_duration_seconds histogram
axum_http_requests_duration_seconds_bucket{method="GET",endpoint="/users",status="200",le="0.005"} 9
…
```

Prometheus y Grafana ya vienen configurados en `docker-compose.yaml`
(servicios `prometheus` y `grafana`); los dashboards se aprovisionan desde
`grafana/provisioning/`.

---

## Modelos

> Los campos `DECIMAL` (`weight`, `power`) se serializan como **string**
> en JSON para preservar precisión. Los `TIMESTAMP` se serializan como
> ISO-8601 sin zona horaria (`"2024-06-01T10:00:00"`) porque la columna
> PostgreSQL es `TIMESTAMP` (sin TZ).

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
  "country": "España",
  "city": "Madrid",
  "address": "Calle Principal 123",
  "laterality": "Derecho",
  "level": "Intermedio"
}
```

> El campo `password` **nunca** se devuelve en respuestas: la columna
> `contrasena` almacena el hash bcrypt y queda excluida del `SELECT`.

### Entrenamiento
```json
{
  "training_id": 1,
  "user_id": 1,
  "routine_id": 1,
  "start_time": "2024-06-01T10:00:00",
  "end_time": "2024-06-01T11:00:00",
  "training_type": "Guiado",
  "calories": 500,
  "current_step": 2,
  "state": "ACTIVO"
}
```

Notas:
- `routine_id` es `null` cuando el entrenamiento es `Libre`.
- `current_step` es un cursor 0-based sobre `routine.punch_sequence`.
- `state` toma valores como `ACTIVO`, `PAUSADO`, `FINALIZADO`. Por defecto
  la BD inserta `ACTIVO`.
- `training_type` se espera `Guiado` o `Libre`, aunque la BD no aplica
  CHECK constraint y aceptaría cualquier `VARCHAR(50)`.

### Golpe
```json
{
  "punch_id": 1,
  "name": "Jab",
  "limb": "Derecha",
  "position": "Cabeza"
}
```

> `limb` y `position` son obligatorios al crear (NOT NULL en la BD desde
> la migración 002). Valores típicos: `limb ∈ {Derecha, Izquierda}`,
> `position ∈ {Cabeza, Cuerpo}`.

### Rutina
```json
{
  "routine_id": 1,
  "name": "Jab-Cross básico",
  "recommended_level": "Principiante",
  "punch_sequence": [1, 7]
}
```

`punch_sequence` es un array ordenado de `punch_id` que define la cadena
de golpes esperada en un entrenamiento `Guiado`. Se persiste como
`INTEGER[]` en PostgreSQL.

### Historial
```json
{
  "history_id": 1,
  "training_id": 1,
  "thrown_punch_id": 1,
  "expected_punch_id": 1,
  "power": "75.50",
  "is_correct": true,
  "impact_date": "2024-06-01T10:00:05"
}
```

Notas:
- `history_id` es la PK desde la migración 002. Antes la PK era el par
  `(training_id, punch_id)`, lo que impedía registrar el mismo golpe dos
  veces en un entrenamiento.
- `thrown_punch_id` = golpe realmente detectado por el sensor.
- `expected_punch_id` = golpe que la rutina esperaba en ese paso; `null`
  para entrenamientos `Libre`.
- `is_correct` por defecto `true`; refleja si `thrown == expected`.
- `impact_date` por defecto `CURRENT_TIMESTAMP` al insertar.

### HistorialDetail
Proyección enriquecida devuelta por los endpoints `GET /history` y
`GET /trainings/:id/history`. Incluye todo lo del modelo `Historial` más
los datos del catálogo `Golpe` (calculados con un JOIN):

```json
{
  "history_id": 1,
  "training_id": 1,
  "thrown_punch_id": 1,
  "expected_punch_id": 1,
  "power": "75.50",
  "is_correct": true,
  "impact_date": "2024-06-01T10:00:05",
  "name": "Jab",
  "limb": "Derecha",
  "position": "Cabeza"
}
```

---

## Users

Todos los endpoints de `USER` requieren autenticación Bearer.

### Listar todos los usuarios
```
GET /users?limit=50&offset=0
Authorization: Bearer <token>
```

**Respuesta (200):** `Usuario[]` (ordenado por `user_id` ASC).

### Obtener un usuario
```
GET /users/:id
Authorization: Bearer <token>
```

**Respuesta (200):** `Usuario`. **404** si no existe.

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

`password` se hashea con bcrypt (`DEFAULT_COST = 12`) antes de guardar.

**Respuesta (200):** `Usuario` recién creado (sin password).

### Actualizar un usuario
```
PUT /users/:id
Authorization: Bearer <token>
Content-Type: application/json

{
  "first_name": "Jane"
}
```

Solo se actualizan los campos presentes (parcial PATCH-like vía
`COALESCE($n, columna)`). Si se incluye `password`, se vuelve a hashear.

**Respuesta (200):** `Usuario` actualizado. **404** si no existe.

### Eliminar un usuario
```
DELETE /users/:id
Authorization: Bearer <token>
```

**Respuesta:** `204 No Content`. **404** si no existe.

> Borrar un usuario cascadea (`ON DELETE CASCADE`) sobre sus
> `entrenamientos`, y a su vez sobre el `historial` asociado.

---

## Trainings

Todos los endpoints de `TRAINING` requieren autenticación Bearer.

### Listar todos los entrenamientos
```
GET /trainings?limit=50&offset=0
Authorization: Bearer <token>
```

Ordenado por `hora_inicio DESC, id_entrenamiento DESC`.

**Respuesta (200):** `Entrenamiento[]`.

### Obtener un entrenamiento
```
GET /trainings/:id
Authorization: Bearer <token>
```

**Respuesta (200):** `Entrenamiento`. **404** si no existe.

### Crear un entrenamiento
```
POST /trainings
Authorization: Bearer <token>
Content-Type: application/json

{
  "user_id": 1,
  "routine_id": 1,
  "start_time": null,
  "end_time": null,
  "training_type": "Guiado",
  "calories": null,
  "current_step": null,
  "state": null
}
```

Solo `user_id` es obligatorio. Los demás campos delegan en los `DEFAULT`
de la BD cuando van como `null`:

| Campo          | Default BD            |
|----------------|-----------------------|
| `start_time`   | `CURRENT_TIMESTAMP`   |
| `calories`     | `0`                   |
| `current_step` | `0`                   |
| `state`        | `'ACTIVO'`            |

**Respuesta (200):** `Entrenamiento` recién creado.

### Actualizar un entrenamiento
```
PUT /trainings/:id
Authorization: Bearer <token>
Content-Type: application/json

{
  "end_time": "2024-06-01T11:00:00",
  "calories": 500,
  "current_step": 4,
  "state": "FINALIZADO"
}
```

Actualización parcial vía `COALESCE`. Para enviar `null` explícito a una
columna nullable, hoy no es posible sin un cambio en el handler
(`Option<Option<T>>`).

**Respuesta (200):** `Entrenamiento` actualizado. **404** si no existe.

### Eliminar un entrenamiento
```
DELETE /trainings/:id
Authorization: Bearer <token>
```

**Respuesta:** `204 No Content`. **404** si no existe. Cascadea sobre
`historial`.

### Listar entrenamientos por usuario
```
GET /users/:id/trainings?limit=50&offset=0
Authorization: Bearer <token>
```

Mismos params de paginación que `GET /trainings`, filtrado por
`id_usuario = :id`.

**Respuesta (200):** `Entrenamiento[]`.

---

## Routines

Todos los endpoints de `ROUTINE` requieren autenticación Bearer.

### Listar todas las rutinas
```
GET /routines?limit=50&offset=0
Authorization: Bearer <token>
```

Ordenado por `id_rutina ASC`.

**Respuesta (200):** `Rutina[]`.

### Obtener una rutina
```
GET /routines/:id
Authorization: Bearer <token>
```

**Respuesta (200):** `Rutina`. **404** si no existe.

### Crear una rutina
```
POST /routines
Authorization: Bearer <token>
Content-Type: application/json

{
  "name": "Combo 4 golpes",
  "recommended_level": "Intermedio",
  "punch_sequence": [1, 7, 13, 9]
}
```

Sólo `name` es obligatorio. `punch_sequence` puede ir vacío (`[]`) o
ausente; se persistirá como `NULL`.

**Respuesta (200):** `Rutina` recién creada (con `routine_id` asignado).

### Actualizar una rutina
```
PUT /routines/:id
Authorization: Bearer <token>
Content-Type: application/json

{
  "punch_sequence": [1, 5, 9]
}
```

Actualización parcial vía `COALESCE`.

**Respuesta (200):** `Rutina` actualizada. **404** si no existe.

### Eliminar una rutina
```
DELETE /routines/:id
Authorization: Bearer <token>
```

**Respuesta:** `204 No Content`. **404** si no existe.

> Si se borra una rutina referenciada por un entrenamiento, la FK aplica
> `ON DELETE SET NULL` y `entrenamiento.routine_id` queda en `null`
> (en vez de cascadear y borrar el entrenamiento).

---

## Punches

Todos los endpoints de `PUNCH` requieren autenticación Bearer.

### Listar todos los golpes
```
GET /punches?limit=50&offset=0
Authorization: Bearer <token>
```

Ordenado por `id_golpe ASC`.

**Respuesta (200):** `Golpe[]`. El catálogo viene precargado con 16
combinaciones (4 nombres × 2 extremidades × 2 posiciones).

### Obtener un golpe
```
GET /punches/:id
Authorization: Bearer <token>
```

**Respuesta (200):** `Golpe`. **404** si no existe.

### Crear un golpe
```
POST /punches
Authorization: Bearer <token>
Content-Type: application/json

{
  "name": "Jab",
  "limb": "Derecha",
  "position": "Cabeza"
}
```

> `limb` y `position` son obligatorios; un cuerpo con `null` será
> rechazado al deserializar (`422 Unprocessable Entity`).

**Respuesta (200):** `Golpe` recién creado.

### Actualizar un golpe
```
PUT /punches/:id
Authorization: Bearer <token>
Content-Type: application/json

{
  "position": "Cuerpo"
}
```

Actualización parcial vía `COALESCE`.

**Respuesta (200):** `Golpe` actualizado. **404** si no existe.

### Eliminar un golpe
```
DELETE /punches/:id
Authorization: Bearer <token>
```

**Respuesta:** `204 No Content`. **404** si no existe.

> Borrar un golpe puede fallar (`500`) si está referenciado por
> `historial` o por `rutina.secuencia_golpes` (sin ON DELETE; la FK
> RESTRICT lanza error). En ese caso conviene desreferenciarlo primero.

---

## History

Todos los endpoints de `HISTORY` requieren autenticación Bearer.

### Listar todo el historial (con detalles del golpe)
```
GET /history?limit=50&offset=0
Authorization: Bearer <token>
```

Ordenado por `fecha_impacto DESC, id_historial DESC`. El JOIN con
`golpe` se hace contra `id_golpe_lanzado`.

**Respuesta (200):** `HistorialDetail[]`.

### Obtener una entrada del historial
```
GET /history/:history_id
Authorization: Bearer <token>
```

> ⚠️ **Breaking change desde la migración 002:** antes el path era
> `/history/:training_id/:punch_id`. La nueva PK sintética
> `id_historial` requiere un único identificador.

**Respuesta (200):** `Historial` (sin los campos del catálogo). **404**
si no existe.

### Crear una entrada del historial
```
POST /history
Authorization: Bearer <token>
Content-Type: application/json

{
  "training_id": 1,
  "thrown_punch_id": 1,
  "expected_punch_id": 1,
  "power": "75.50",
  "is_correct": true,
  "impact_date": null
}
```

Sólo `training_id` y `thrown_punch_id` son obligatorios. `is_correct` y
`impact_date` aplican los `DEFAULT` de la BD (`TRUE` y
`CURRENT_TIMESTAMP`) cuando llegan en `null`.

**Respuesta (200):** `Historial` recién creado (con `history_id` y
`impact_date` resueltos por la BD).

### Actualizar una entrada del historial
```
PUT /history/:history_id
Authorization: Bearer <token>
Content-Type: application/json

{
  "power": "80.00",
  "is_correct": false
}
```

Actualización parcial vía `COALESCE`.

**Respuesta (200):** `Historial` actualizado. **404** si no existe.

### Eliminar una entrada del historial
```
DELETE /history/:history_id
Authorization: Bearer <token>
```

**Respuesta:** `204 No Content`. **404** si no existe.

### Listar historial por entrenamiento
```
GET /trainings/:id/history?limit=50&offset=0
Authorization: Bearer <token>
```

Filtrado por `id_entrenamiento = :id`, ordenado por
`fecha_impacto ASC, id_historial ASC` (cronológico).

**Respuesta (200):** `HistorialDetail[]`.

---

## Variables de entorno

| Variable | Por defecto | Descripción |
|----------|-------------|-------------|
| `DATABASE_URL` | (obligatorio) | Cadena de conexión a PostgreSQL, p. ej. `postgres://knockshadow:knockshadow@127.0.0.1:5432/knockshadow`. Sin este valor la API hace fail-fast al arrancar. |
| `PORT` | `3000` | Puerto del servidor HTTP (debe parsearse como `u16`). |
| `JWT_SECRET` | (obligatorio) | Clave secreta para firmar JWT. **Mínimo 32 caracteres**. Generar con `openssl rand -hex 32`. La API se niega a arrancar si falta o es muy corta. |
| `RESEND_API_KEY` | _(opcional)_ | Clave de Resend para envío de emails. Si está vacía, los endpoints de email caen al modo stub (loggean en vez de enviar). Crea/rota la clave en `https://resend.com/api-keys`. |
| `RESEND_FROM` | `KnockShadow <no-reply@knockshadow.site>` | Buzón "From" usado en los emails. Debe pertenecer a un dominio verificado en Resend. |
| `APP_BASE_URL` | `https://api.knockshadow.site` | Base de los enlaces que se incluyen en los emails de confirmación. |
| `REDIS_URL` | _(opcional)_ | URL del backend RESP usado para rate-limit distribuido (`redis://host:6379/`). Sin ella, el rate-limit es in-memory (válido para 1 pod). El stack docker-compose levanta un Valkey local; usar `redis://valkey:6379/`. |

---

## Códigos de respuesta

| Estado | Significado |
|--------|-------------|
| `200 OK` | Éxito (incluido `POST` y `PUT`; la API no usa `201` actualmente). |
| `204 No Content` | Recurso eliminado. |
| `401 Unauthorized` | Falta token Bearer, token inválido/expirado o credenciales incorrectas en `/login`. |
| `404 Not Found` | Recurso no encontrado por ID. |
| `422 Unprocessable Entity` | JSON malformado o falta un campo obligatorio del payload. |
| `500 Internal Server Error` | Error de base de datos, hash bcrypt, codificación JWT u otra excepción interna. |
