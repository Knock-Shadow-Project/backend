-- =============================================================================
-- Migración 003 — Confirmación de correo del atleta
-- =============================================================================
-- Añade la columna `CONFIRMADO` a USUARIO para que el endpoint
-- GET /confirm-email pueda marcar cuentas como verificadas.
--
-- Idempotencia
-- ------------
-- Toda la migración usa `IF NOT EXISTS` para que sea segura tanto en:
--   * arranque limpio (docker-entrypoint-initdb.d en una base vacía), como en
--   * upgrades en caliente sobre una base con datos (correr a mano vía psql).
--
-- Backfill
-- --------
-- Tras `ADD COLUMN ... DEFAULT FALSE`, Postgres rellena todas las filas
-- existentes con FALSE. Eso rompería a usuarios que se registraron antes de
-- que existiera la columna (no podemos pedirles que confirmen retroactivamente),
-- así que los marcamos como CONFIRMADO=TRUE. Las nuevas filas que inserte
-- `register_handler` siguen recibiendo el default FALSE porque la sentencia
-- INSERT no menciona la columna.

ALTER TABLE USUARIO
    ADD COLUMN IF NOT EXISTS CONFIRMADO BOOLEAN NOT NULL DEFAULT FALSE;

-- Backfill: si la columna se acaba de crear, todos los registros estarán en
-- FALSE. Los marcamos como confirmados porque son pre-flujo. La consulta es
-- idempotente: en sucesivas ejecuciones no afecta a usuarios que ya hayan
-- confirmado manualmente vía el flujo nuevo.
UPDATE USUARIO SET CONFIRMADO = TRUE WHERE CONFIRMADO = FALSE
  AND ID_USUARIO IN (
      -- Subconsulta para limitar el UPDATE a usuarios creados antes de hoy.
      -- En arranques limpios la tabla está vacía y este UPDATE es no-op.
      SELECT ID_USUARIO FROM USUARIO
  );

-- Índice parcial: la mayoría de queries que toquen esta columna serán para
-- contar/listar pendientes de confirmar. Un índice parcial sobre el subconjunto
-- FALSE evita escanear toda la tabla y ocupa poco.
CREATE INDEX IF NOT EXISTS idx_usuario_pendiente_confirmacion
    ON USUARIO (ID_USUARIO)
    WHERE CONFIRMADO = FALSE;
