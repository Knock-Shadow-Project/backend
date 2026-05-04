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

-- Crear la tabla ENTRENAMIENTO
-- La relación (1:N) con USUARIO se resuelve agregando el ID_USUARIO aquí
CREATE TABLE IF NOT EXISTS ENTRENAMIENTO (
    ID_ENTRENAMIENTO SERIAL PRIMARY KEY,
    HORA_INICIO TIMESTAMP NOT NULL,
    HORA_FIN TIMESTAMP,
    TIPO VARCHAR(50),
    CALORIAS INTEGER,
    ID_USUARIO INTEGER NOT NULL,
    CONSTRAINT fk_usuario_entrenamiento FOREIGN KEY (ID_USUARIO) REFERENCES USUARIO (ID_USUARIO) ON DELETE CASCADE
);

-- Crear la tabla GOLPE
CREATE TABLE IF NOT EXISTS GOLPE (
    ID_GOLPE SERIAL PRIMARY KEY,
    NOMBRE VARCHAR(50) NOT NULL, -- Ej: Jab, Cross, Upper, Gancho
    EXTREMIDAD VARCHAR(50), -- Ej: Derecha, Izquierda
    POSICION VARCHAR(50) -- Ej: Cabeza, Cabeza, Cabeza, Cuerpo, Cuerpo, Cuerpo
);

-- Crear la tabla HISTORIAL (Relación N:M entre ENTRENAMIENTO y GOLPE)
-- Esta tabla incluye el atributo propio 'POTENCIA'
CREATE TABLE IF NOT EXISTS HISTORIAL (
    ID_ENTRENAMIENTO INTEGER NOT NULL,
    ID_GOLPE INTEGER NOT NULL,
    POTENCIA DECIMAL(10, 2), -- Atributo de la relación
    PRIMARY KEY (ID_ENTRENAMIENTO, ID_GOLPE),
    CONSTRAINT fk_historial_entrenamiento FOREIGN KEY (ID_ENTRENAMIENTO) REFERENCES ENTRENAMIENTO (ID_ENTRENAMIENTO) ON DELETE CASCADE,
    CONSTRAINT fk_historial_golpe FOREIGN KEY (ID_GOLPE) REFERENCES GOLPE (ID_GOLPE) ON DELETE CASCADE
);

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

-- Insertar datos de ejemplo en ENTRENAMIENTO
INSERT INTO
    ENTRENAMIENTO (
        HORA_INICIO,
        HORA_FIN,
        TIPO,
        CALORIAS,
        ID_USUARIO
    )
VALUES (
        '2024-06-01 10:00:00',
        '2024-06-01 11:00:00',
        'Estandar',
        500,
        1
    ),
    (
        '2024-06-02 15:00:00',
        '2024-06-02 16:00:00',
        'Fuerza',
        600,
        1
    );

-- Insertar datos de ejemplo en HISTORIAL
INSERT INTO
    HISTORIAL (
        ID_ENTRENAMIENTO,
        ID_GOLPE,
        POTENCIA
    )
VALUES (1, 1, 75.5), -- Jab Derecha Cabeza en el primer entrenamiento
    (1, 5, 80.0), -- Cross Derecha Cuerpo en el primer entrenamiento
    (2, 9, 85.0), -- Upper Derecha Cabeza en el segundo entrenamiento
    (2, 13, 90.0);
-- Gancho Derecha Cuerpo en el segundo entrenamiento