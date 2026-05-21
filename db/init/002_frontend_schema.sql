-- Crear la tabla USUARIO
CREATE TABLE IF NOT EXISTS USUARIO (
    ID_USUARIO SERIAL PRIMARY KEY,
    NOMBRE VARCHAR(100) NOT NULL,
    APELLIDO VARCHAR(100) NOT NULL,
    CORREO VARCHAR(150) UNIQUE NOT NULL,
    CONTRASENA VARCHAR(255) NOT NULL,
    TELEFONO VARCHAR(20),
    EDAD INTEGER,
    PESO DECIMAL(5, 2),
    ESTATURA INTEGER,
    PAIS VARCHAR(50),
    CIUDAD VARCHAR(50),
    DIRECCION VARCHAR(255),
    LATERALIDAD VARCHAR(20), -- Ej: Izquierdo, Derecho
    NIVEL VARCHAR(50) -- Ej: Principiante, Pro
);

-- Crear la tabla GOLPE
-- Catálogo declarado antes que RUTINA porque RUTINA referencia IDs de GOLPE
-- en su columna SECUENCIA_GOLPES (aunque sea un array sin FK declarada,
-- mantener el orden lógico evita sorpresas si en el futuro se añade un check).
CREATE TABLE IF NOT EXISTS GOLPE (
    ID_GOLPE SERIAL PRIMARY KEY,
    NOMBRE VARCHAR(50) NOT NULL, -- Ej: Jab, Cross, Upper, Gancho
    EXTREMIDAD VARCHAR(50) NOT NULL, -- Ej: Derecha, Izquierda
    POSICION VARCHAR(50) NOT NULL -- Ej: Cabeza, Cuerpo
);

-- Crear la tabla RUTINA
-- Debe declararse antes que ENTRENAMIENTO porque éste tiene FK hacia RUTINA.
CREATE TABLE IF NOT EXISTS RUTINA (
    ID_RUTINA SERIAL PRIMARY KEY,
    NOMBRE VARCHAR(100) NOT NULL,
    NIVEL_RECOMENDADO VARCHAR(50),
    SECUENCIA_GOLPES INTEGER[]       -- Array de IDs de GOLPE para la secuencia rítmica
);

-- Crear la tabla ENTRENAMIENTO
-- La relación (1:N) con USUARIO se resuelve agregando el ID_USUARIO aquí
CREATE TABLE IF NOT EXISTS ENTRENAMIENTO (
    ID_ENTRENAMIENTO SERIAL PRIMARY KEY,
    ID_USUARIO INTEGER NOT NULL,
    ID_RUTINA INTEGER, -- Puede ser NULL si el entrenamiento es 'Libre'
    HORA_INICIO TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    HORA_FIN TIMESTAMP,
    TIPO VARCHAR(50), -- 'Guiado' o 'Libre'
    CALORIAS INTEGER DEFAULT 0,
    PASO_ACTUAL INTEGER DEFAULT 0,
    ESTADO VARCHAR(20) DEFAULT 'ACTIVO',
    CONSTRAINT fk_usuario_entrenamiento FOREIGN KEY (ID_USUARIO) REFERENCES USUARIO (ID_USUARIO) ON DELETE CASCADE,
    CONSTRAINT fk_rutina_entrenamiento FOREIGN KEY (ID_RUTINA) REFERENCES RUTINA (ID_RUTINA) ON DELETE SET NULL
);

-- Crear la tabla HISTORIAL (Relación N:M entre ENTRENAMIENTO y GOLPE)
-- Esta tabla incluye el atributo propio 'POTENCIA'
CREATE TABLE IF NOT EXISTS HISTORIAL (
    ID_HISTORIAL SERIAL PRIMARY KEY,
    ID_ENTRENAMIENTO INTEGER NOT NULL,
    ID_GOLPE_LANZADO INTEGER NOT NULL,
    ID_GOLPE_ESPERADO INTEGER, -- Puede ser NULL si el entrenamiento fue 'Libre'
    POTENCIA DECIMAL(10, 2),
    ES_CORRECTO BOOLEAN DEFAULT TRUE,
    FECHA_IMPACTO TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT fk_hist_entrenamiento FOREIGN KEY (ID_ENTRENAMIENTO) REFERENCES ENTRENAMIENTO (ID_ENTRENAMIENTO) ON DELETE CASCADE,
    CONSTRAINT fk_hist_lanzado FOREIGN KEY (ID_GOLPE_LANZADO) REFERENCES GOLPE (ID_GOLPE),
    CONSTRAINT fk_hist_esperado FOREIGN KEY (ID_GOLPE_ESPERADO) REFERENCES GOLPE (ID_GOLPE)
);

-- Índices útiles para los listados habituales del API
CREATE INDEX IF NOT EXISTS idx_entrenamiento_usuario ON ENTRENAMIENTO (ID_USUARIO);
CREATE INDEX IF NOT EXISTS idx_entrenamiento_rutina ON ENTRENAMIENTO (ID_RUTINA);
CREATE INDEX IF NOT EXISTS idx_historial_entrenamiento ON HISTORIAL (ID_ENTRENAMIENTO);
CREATE INDEX IF NOT EXISTS idx_historial_golpe_lanzado ON HISTORIAL (ID_GOLPE_LANZADO);

-- Insertar datos de ejemplo en GOLPE
INSERT INTO
    GOLPE (NOMBRE, EXTREMIDAD, POSICION)
VALUES ('Jab', 'Derecha', 'Cabeza'),
    ('Jab', 'Izquierda', 'Cabeza'),
    ('Jab', 'Derecha', 'Cuerpo'),
    ('Jab', 'Izquierda', 'Cuerpo'),
    ('Cross', 'Derecha', 'Cuerpo'),
    (
        'Cross',
        'Izquierda',
        'Cuerpo'
    ),
    ('Cross', 'Derecha', 'Cabeza'),
    (
        'Cross',
        'Izquierda',
        'Cabeza'
    ),
    ('Upper', 'Derecha', 'Cabeza'),
    (
        'Upper',
        'Izquierda',
        'Cabeza'
    ),
    ('Upper', 'Derecha', 'Cuerpo'),
    (
        'Upper',
        'Izquierda',
        'Cuerpo'
    ),
    ('Gancho', 'Derecha', 'Cuerpo'),
    (
        'Gancho',
        'Izquierda',
        'Cuerpo'
    ),
    ('Gancho', 'Derecha', 'Cabeza'),
    (
        'Gancho',
        'Izquierda',
        'Cabeza'
    );

-- Insertar datos de ejemplo en USUARIO
INSERT INTO
    USUARIO (
        NOMBRE,
        APELLIDO,
        CORREO,
        CONTRASENA,
        TELEFONO,
        EDAD,
        PESO,
        ESTATURA,
        PAIS,
        CIUDAD,
        DIRECCION,
        LATERALIDAD,
        NIVEL
    )
VALUES (
        'Admin',
        'Admin',
        'admin@example.com',
        '2admin1',
        '+34123456789',
        30,
        70.5,
        175,
        'España',
        'Madrid',
        'Calle Principal 123',
        'Derecho',
        'Intermedio'
    );

-- Insertar datos de ejemplo en RUTINA
INSERT INTO
    RUTINA (NOMBRE, NIVEL_RECOMENDADO, SECUENCIA_GOLPES)
VALUES (
        'Jab-Cross básico',
        'Principiante',
        ARRAY[1, 7]
    ),
    (
        'Combo 4 golpes',
        'Intermedio',
        ARRAY[1, 7, 13, 9]
    );

-- Insertar datos de ejemplo en ENTRENAMIENTO
INSERT INTO
    ENTRENAMIENTO (
        HORA_INICIO,
        HORA_FIN,
        TIPO,
        CALORIAS,
        ID_USUARIO,
        ID_RUTINA,
        PASO_ACTUAL,
        ESTADO
    )
VALUES (
        '2024-06-01 10:00:00',
        '2024-06-01 11:00:00',
        'Guiado',
        500,
        1,
        1,
        2,
        'FINALIZADO'
    ),
    (
        '2024-06-02 15:00:00',
        '2024-06-02 16:00:00',
        'Libre',
        600,
        1,
        NULL,
        0,
        'FINALIZADO'
    );

-- Insertar datos de ejemplo en HISTORIAL
INSERT INTO
    HISTORIAL (
        ID_ENTRENAMIENTO,
        ID_GOLPE_LANZADO,
        ID_GOLPE_ESPERADO,
        POTENCIA,
        ES_CORRECTO
    )
VALUES (1, 1, 1, 75.5, TRUE), -- Jab Derecha Cabeza esperado y lanzado en el primer entrenamiento
    (1, 7, 7, 80.0, TRUE), -- Cross Derecha Cabeza correcto en el primer entrenamiento
    (2, 9, NULL, 85.0, TRUE), -- Upper Derecha Cabeza en el segundo entrenamiento (Libre)
    (2, 13, NULL, 90.0, TRUE);
-- Gancho Derecha Cuerpo en el segundo entrenamiento (Libre)
