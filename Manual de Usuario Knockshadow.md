---
title: "Manual de Usuario — KnockShadow"
subtitle: "Saco de Boxeo Inteligente"
date: "30/05/2026"
author:
  - Victor Galán Martinez
  - Cristhian Dávila Andrade
---

# MANUAL DE USUARIO

## KnockShadow — Saco de Boxeo Inteligente

**Versión 1.0 — 30/05/2026**

Integrantes:

Victor Galán Martinez.

Cristhian Dávila Andrade.

```{=openxml}
<w:p><w:r><w:br w:type="page"/></w:r></w:p>
```

# Índice

El índice se generará automáticamente desde Google Docs mediante Insertar → Tabla de contenidos.

```{=openxml}
<w:p><w:r><w:br w:type="page"/></w:r></w:p>
```

# 1. Introducción

KnockShadow es un saco de boxeo inteligente basado en tecnologías IoT e inteligencia artificial, diseñado para proporcionar retroalimentación en tiempo real sobre la ejecución de golpes durante el entrenamiento deportivo. El sistema combina sensores inerciales de alta precisión, comunicación inalámbrica Bluetooth Low Energy, procesamiento edge mediante una pasarela local y sincronización con infraestructura cloud, permitiendo registrar, analizar y mejorar progresivamente la técnica del usuario.

El presente manual tiene como objetivo guiar al usuario en todas las fases relacionadas con el uso del producto, desde la instalación física hasta el aprovechamiento avanzado de las funcionalidades disponibles en la aplicación móvil. El documento ha sido elaborado para cubrir tanto las necesidades de usuarios domésticos individuales como las de instaladores profesionales, gimnasios y centros de entrenamiento de boxeo.

KnockShadow integra una red neuronal entrenada para clasificar automáticamente los principales tipos de golpe del boxeo, alcanzando actualmente una precisión aproximada del 85% sobre golpes correctamente ejecutados. El sistema continúa mejorando progresivamente a medida que recibe nuevos datos de entrenamiento, gracias a su arquitectura de aprendizaje continuo.

Antes de comenzar el uso del producto, se recomienda leer íntegramente el presente manual, prestando especial atención al apartado de advertencias de seguridad y al procedimiento de instalación mural, debido a las cargas dinámicas elevadas a las que se somete la estructura durante el entrenamiento.

# 2. Advertencias de seguridad

El uso de KnockShadow implica impactos repetitivos de alta intensidad sobre una estructura anclada mediante soporte mural. Por este motivo, resulta imprescindible respetar las indicaciones de seguridad descritas a continuación con el fin de evitar lesiones personales, daños materiales y deterioro prematuro del producto.

**Anclaje mural obligatorio:** el producto debe instalarse exclusivamente sobre paredes estructurales de hormigón, ladrillo macizo o mampostería equivalente. Está terminantemente prohibida la instalación sobre tabiques de yeso laminado (pladur), paredes huecas o estructuras de madera ligera, ya que no soportan las cargas dinámicas generadas durante el entrenamiento.

**Verificación del soporte:** antes de cada sesión de entrenamiento, el usuario debe inspeccionar visualmente la estructura mural metálica y la tornillería de fijación, verificando la ausencia de holguras, deformaciones o signos de aflojamiento.

**Distancia de seguridad:** durante el uso del saco, debe mantenerse una zona libre de obstáculos de al menos un metro y medio alrededor del producto, evitando la presencia de otras personas, mascotas o mobiliario en dicho perímetro.

**Uso con equipamiento adecuado:** se recomienda encarecidamente el uso de guantes de boxeo, vendas y, en su caso, protector bucal durante el entrenamiento. KnockShadow no sustituye el equipamiento de protección personal del usuario.

**Restricciones de uso:** el producto no está diseñado para uso infantil sin supervisión. La edad mínima recomendada para uso autónomo es de 16 años. Personas con condiciones cardiovasculares, musculoesqueléticas o neurológicas deben consultar a un profesional sanitario antes de iniciar el entrenamiento.

**Condiciones ambientales:** KnockShadow está diseñado para uso en interiores, en entornos secos y a temperaturas comprendidas entre +5 °C y +35 °C. La exposición a humedad elevada, agua directa o radiación solar prolongada puede comprometer el funcionamiento de los componentes electrónicos.

**Manipulación de la electrónica:** queda prohibida la apertura del producto por parte del usuario. Cualquier intervención sobre los sensores, módulos de comunicación o cableado interno debe realizarse exclusivamente por personal técnico autorizado.

**Desconexión eléctrica:** antes de cualquier tarea de mantenimiento o limpieza, debe desconectarse la alimentación eléctrica del sistema.

# 3. Contenido del paquete

El embalaje de KnockShadow incluye los siguientes elementos. Se recomienda verificar la presencia de todos ellos antes de iniciar la instalación. En caso de detectar faltantes, contactar con el servicio técnico antes de proceder.

| Elemento | Cantidad | Descripción |
| :---- | :---- | :---- |
| Cuerpo principal del saco | 1 | Estructura con revestimiento de poliuretano y núcleo de espuma EVA de alta densidad |
| Soporte mural de acero estructural | 1 | Bastidor metálico de anclaje a pared con bastidor interno de madera técnica |
| Kit de tornillería reforzada | 1 | Tornillos de fijación, arandelas, tuercas autoblocantes y tacos químicos o mecánicos según superficie |
| Sensores inerciales preinstalados | [TODO: número exacto] | Módulos Bosch BMI160 integrados internamente en el cuerpo del saco |
| Microcontrolador ESP32 integrado | 1 | Módulo de comunicación BLE preinstalado y configurado de fábrica |
| Pasarela edge (Raspberry Pi 5) | 1 | Unidad de procesamiento local con sistema operativo preinstalado |
| Fuente de alimentación | 1 | Adaptador de corriente regulado con conector específico |
| Cable de red Ethernet | 1 | Cable opcional para conexión cableada de la pasarela |
| Llave de instalación | 1 | Llave de tipo Allen y/o llave fija incluida para el montaje de la tornillería |
| Plantilla de marcado mural | 1 | Plantilla de cartón rígido con la posición exacta de los puntos de anclaje |
| Tarjeta de inicio rápido | 1 | Documento físico con código QR de descarga de la aplicación móvil |
| Manual de usuario | 1 | El presente documento |

En caso de adquirir paquetes destinados a gimnasios o instalaciones profesionales, el contenido puede variar incluyendo unidades adicionales, soportes específicos o accesorios complementarios. La configuración exacta del paquete adquirido figura en el albarán de entrega.

# 4. Especificaciones técnicas

A continuación se resumen las características técnicas principales del sistema. Estos valores corresponden a la versión preindustrial actual del producto y pueden verse actualizados en versiones futuras mediante actualizaciones de firmware.

| Categoría | Característica | Valor |
| :---- | :---- | :---- |
| Sensor inercial | Modelo | Bosch BMI160 |
| Sensor inercial | Tipo | IMU de 6 ejes (acelerómetro + giroscopio) |
| Microcontrolador | Modelo | Espressif ESP32-WROOM-32 |
| Microcontrolador | Conectividad | Bluetooth Low Energy 4.2 / WiFi 2.4 GHz |
| Pasarela edge | Modelo | Raspberry Pi 5 |
| Pasarela edge | Función | Procesamiento local, sincronización cloud, ejecución de lógica de entrenamiento |
| Comunicación | Protocolo dispositivo–pasarela | Bluetooth Low Energy |
| Comunicación | Protocolo pasarela–nube | MQTT sobre TLS |
| Latencia | Impacto → respuesta en app | Inferior a 150 ms |
| Inteligencia artificial | Precisión actual de clasificación | Aproximadamente 85% |
| Estructura mecánica | Material absorbente | Espuma EVA de alta densidad |
| Estructura mecánica | Revestimiento exterior | Poliuretano sintético (PU) |
| Estructura mecánica | Bastidor interno | Madera técnica |
| Estructura mecánica | Soporte mural | Acero estructural |
| Vida útil estructural estimada | Impactos | Superior a 2.000.000 |
| Alimentación | Tensión de entrada | [TODO: voltaje fuente, p. ej. 100–240 V CA] |
| Alimentación | Tensión de salida | [TODO: voltaje regulado] |
| Condiciones de uso | Temperatura | +5 °C a +35 °C |
| Condiciones de uso | Humedad relativa | Inferior al 80% sin condensación |

# 5. Instalación física y anclaje mural

El proceso de instalación del soporte mural constituye el aspecto más crítico desde el punto de vista de seguridad. Una instalación incorrecta puede provocar el desprendimiento del producto durante el entrenamiento, con riesgo de lesiones graves para el usuario y daños materiales en la vivienda o instalación. Se recomienda encarecidamente que esta tarea sea realizada por personal cualificado, especialmente en el caso de instalaciones profesionales en gimnasios o centros deportivos.

## 5.1 Herramientas necesarias

Antes de iniciar la instalación, debe disponerse de las siguientes herramientas:

- Taladro percutor con brocas adecuadas al tipo de pared (hormigón, ladrillo o mampostería).
- Nivel de burbuja o nivel láser.
- Lápiz o rotulador de marcaje.
- Cinta métrica.
- Llave dinamométrica (recomendada).
- Llaves fijas y/o Allen incluidas en el kit.
- Aspirador o escoba para retirar el polvo generado durante el taladrado.
- Equipo de protección individual: gafas de seguridad y guantes.

## 5.2 Selección del punto de instalación

La elección del punto de instalación debe cumplir los siguientes requisitos:

- **Tipo de pared:** la superficie debe ser exclusivamente hormigón armado, ladrillo macizo o mampostería estructural. Queda prohibida la instalación sobre tabiques de pladur, paredes huecas, paneles ligeros o estructuras de madera no portante.
- **Altura recomendada:** el punto medio del saco debe situarse aproximadamente a la altura del esternón del usuario principal. Para un usuario medio adulto, esta altura suele corresponder a entre 1,30 m y 1,50 m del suelo medidos desde la base del saco.
- **Espacio libre:** el producto debe disponer de un perímetro libre de obstáculos de al menos 1,5 metros en todas las direcciones, incluyendo zona superior, inferior y laterales.
- **Iluminación y ventilación:** se recomienda instalar el producto en una zona con buena iluminación y ventilación, evitando proximidad a fuentes directas de calor, humedad o radiación solar.
- **Cobertura inalámbrica:** la zona de instalación debe disponer de buena cobertura WiFi y proximidad razonable (inferior a 10 metros) respecto a la pasarela edge para garantizar la estabilidad de la conexión Bluetooth Low Energy.

## 5.3 Procedimiento de anclaje

El procedimiento de instalación se compone de los siguientes pasos:

1. **Marcado de puntos.** Colocar la plantilla de marcado mural incluida en el embalaje sobre la pared, verificando su correcta nivelación mediante el nivel de burbuja. Marcar con lápiz la posición exacta de cada uno de los puntos de anclaje.
2. **Perforación.** Utilizando el taladro percutor y la broca correspondiente al diámetro indicado en la plantilla, realizar los orificios respetando la profundidad mínima necesaria para los tacos suministrados. [TODO: diámetro broca y profundidad exactos según taco]
3. **Limpieza de orificios.** Retirar el polvo y los restos generados durante el taladrado, garantizando un correcto asentamiento de los tacos.
4. **Inserción de tacos.** Introducir los tacos químicos o mecánicos suministrados, comprobando que quedan al ras de la superficie de la pared.
5. **Fijación del soporte metálico.** Posicionar el soporte mural de acero estructural alineándolo con los orificios y atornillar la tornillería incluida en el kit. Se recomienda apretar los tornillos progresivamente y en diagonal, sin completar el par de apriete final hasta haber introducido todos los tornillos.
6. **Apriete final.** Aplicar el par de apriete recomendado utilizando una llave dinamométrica. [TODO: valor exacto del par de apriete, p. ej. 25 N·m]
7. **Acoplamiento del cuerpo del saco.** Encajar el cuerpo del saco sobre el soporte mural ya fijado, asegurando que todos los puntos de unión queden correctamente bloqueados.
8. **Verificación de cargas.** Ejercer una tracción manual progresiva sobre el cuerpo del saco para confirmar la ausencia de holguras o movimientos anómalos en el conjunto.

## 5.4 Verificación post-instalación

Una vez completada la instalación, debe realizarse una verificación funcional consistente en:

- Comprobación visual de la alineación general del producto.
- Verificación de la nivelación horizontal y vertical.
- Aplicación de cargas progresivas mediante golpes suaves para detectar posibles vibraciones excesivas, ruidos anómalos o desplazamientos del soporte.
- Inspección periódica del estado del anclaje durante las primeras semanas de uso intensivo.

Cualquier anomalía detectada durante esta verificación obliga a interrumpir el uso del producto hasta su corrección por parte del instalador o del servicio técnico.

# 6. Instalación y configuración de la aplicación móvil

La aplicación móvil KnockShadow constituye la interfaz principal de uso del sistema. A través de ella el usuario gestiona su cuenta, realiza el emparejamiento del dispositivo, accede a los modos de entrenamiento y consulta las métricas históricas registradas durante las sesiones.

## 6.1 Descarga e instalación

La aplicación se encuentra disponible para los principales sistemas operativos móviles. Para descargarla, el usuario dispone de las siguientes opciones:

- **Android:** descarga gratuita desde Google Play Store buscando «KnockShadow» o escaneando el código QR incluido en la tarjeta de inicio rápido. [TODO: URL exacta cuando esté publicada]
- **iOS:** descarga gratuita desde Apple App Store buscando «KnockShadow» o escaneando el código QR incluido en la tarjeta de inicio rápido. [TODO: URL exacta cuando esté publicada]

Los requisitos mínimos del dispositivo móvil son:

- Sistema operativo Android 9.0 o superior, o iOS 14.0 o superior. [TODO: confirmar versiones objetivo]
- Bluetooth Low Energy 4.2 o superior.
- Conexión a Internet activa para el registro inicial y la sincronización de métricas.
- Permisos de ubicación activados (requeridos por el sistema operativo para el escaneo de dispositivos BLE).

## 6.2 Registro de cuenta

Para acceder a las funcionalidades del sistema, el usuario debe disponer de una cuenta personal. El proceso de registro se realiza directamente desde la aplicación móvil mediante los siguientes pasos:

1. Abrir la aplicación y seleccionar la opción «Crear cuenta».
2. Introducir una dirección de correo electrónico válida.
3. Definir una contraseña segura conforme a las recomendaciones mostradas en pantalla.
4. Cumplimentar los datos personales requeridos: nombre, fecha de nacimiento, nivel deportivo aproximado y unidades de medida preferidas.
5. Aceptar los términos y condiciones de uso y la política de privacidad.
6. Pulsar «Registrarse» para completar el proceso.

## 6.3 Verificación de correo electrónico

Una vez completado el registro, el sistema envía automáticamente un correo electrónico de verificación a la dirección proporcionada. Este paso resulta obligatorio antes de poder utilizar la aplicación.

El proceso de verificación se completa de la siguiente forma:

1. Acceder a la bandeja de entrada de la cuenta de correo registrada.
2. Localizar el mensaje remitido por KnockShadow con asunto «Verifica tu cuenta».
3. Pulsar sobre el enlace de verificación incluido en el cuerpo del mensaje.
4. Esperar la confirmación del sistema, tras la cual el usuario será redirigido automáticamente a la aplicación.

En caso de no recibir el correo electrónico en un plazo razonable, se recomienda revisar la carpeta de correo no deseado o spam. La aplicación dispone igualmente de una opción para reenviar el correo de verificación.

# 7. Emparejamiento del dispositivo

El emparejamiento entre la aplicación móvil y el dispositivo KnockShadow se realiza mediante comunicación Bluetooth Low Energy. Este proceso debe completarse en la primera puesta en marcha del sistema y, salvo modificaciones de configuración, no es necesario repetirlo posteriormente.

El procedimiento de emparejamiento es el siguiente:

1. **Conexión eléctrica del sistema.** Conectar la fuente de alimentación de la pasarela edge a una toma eléctrica estándar y esperar a que se complete el arranque del sistema. La pasarela tarda aproximadamente entre treinta segundos y un minuto en estar operativa.
2. **Verificación del estado del dispositivo.** Comprobar que el indicador luminoso del cuerpo del saco se encuentra activo, indicando la disponibilidad del módulo Bluetooth para emparejamiento. [TODO: descripción exacta del comportamiento del LED]
3. **Activación del Bluetooth y la ubicación.** En el dispositivo móvil, activar el Bluetooth y los servicios de ubicación, ambos requeridos para el descubrimiento de dispositivos BLE.
4. **Acceso al menú de emparejamiento.** Abrir la aplicación KnockShadow e iniciar sesión con las credenciales registradas previamente. Acceder al menú «Configuración» y seleccionar «Emparejar nuevo dispositivo».
5. **Escaneo de dispositivos disponibles.** La aplicación iniciará automáticamente el escaneo de dispositivos KnockShadow detectables en el entorno. Tras unos segundos, el dispositivo correspondiente aparecerá en la lista identificado mediante un nombre único.
6. **Selección y confirmación.** Pulsar sobre el dispositivo identificado y, si la aplicación lo solicita, introducir el código de emparejamiento mostrado por el sistema. Confirmar la operación.
7. **Asociación con la cuenta de usuario.** Una vez completado el emparejamiento, la aplicación asociará el dispositivo con la cuenta del usuario, permitiendo la sincronización de datos y el acceso a los modos de entrenamiento.

En caso de no detectarse el dispositivo durante el escaneo, se recomienda reducir la distancia entre el móvil y la pasarela, comprobar la ausencia de interferencias inalámbricas próximas y verificar que el sistema se encuentra correctamente alimentado. En caso de persistir el problema, consultar el apartado de resolución de problemas del presente manual.

# 8. Primer uso y calibración inicial

Tras completar el emparejamiento del dispositivo, el sistema requiere una calibración inicial cuyo objetivo es ajustar los parámetros de los sensores inerciales al entorno específico de instalación y a las características del usuario principal.

La calibración se inicia automáticamente desde la aplicación móvil en el primer acceso posterior al emparejamiento y se compone de las siguientes fases:

1. **Verificación de estabilidad estructural.** El sistema solicita al usuario que permanezca alejado del saco durante unos segundos con el fin de registrar la línea base de vibración ambiental y descartar oscilaciones residuales derivadas de la instalación.
2. **Calibración postural del usuario.** La aplicación solicita información complementaria al usuario, incluyendo altura, peso, lateralidad (diestro o zurdo) y nivel deportivo. Estos datos permiten ajustar las referencias de intensidad y velocidad propias del modelo de clasificación.
3. **Registro de golpes de referencia.** El sistema guía al usuario a través de la ejecución de una serie reducida de golpes básicos —jab, cross, hook y uppercut— a intensidad controlada. Cada golpe es identificado y registrado por la aplicación como referencia personalizada.
4. **Confirmación final.** Una vez completados los golpes de referencia, el sistema confirma la finalización de la calibración y habilita el acceso a los modos de entrenamiento.

La calibración inicial debe repetirse en caso de cambios significativos en la posición de instalación del producto, sustitución del usuario principal o variaciones notables en el peso o nivel deportivo del mismo. La aplicación dispone de la opción «Recalibrar dispositivo» dentro del menú de configuración para iniciar el proceso de forma manual.

# 9. Modos de entrenamiento

La aplicación KnockShadow ofrece distintos modos de entrenamiento orientados a usuarios con diferentes niveles de experiencia y objetivos deportivos. Cada modo dispone de su propio sistema de métricas, indicaciones visuales y, en su caso, audio guiado.

| Modo | Descripción | Indicado para |
| :---- | :---- | :---- |
| Entrenamiento libre | Sesión sin guion predefinido en la que el usuario golpea libremente y el sistema registra todas las métricas | Todos los niveles |
| Entrenamiento guiado | Sesión estructurada en rounds con indicaciones específicas de golpes y combinaciones | Principiantes e intermedios |
| Combinaciones técnicas | Reproducción de combinaciones predefinidas que el usuario debe ejecutar dentro de un margen de tiempo | Intermedios y avanzados |
| Modo competición | Sesiones con puntuación competitiva basada en precisión, intensidad y velocidad | Usuarios avanzados |
| Modo cardio | Sesiones de alta intensidad orientadas al gasto calórico y la mejora cardiovascular | Todos los niveles |
| Modo evaluación | Análisis técnico detallado de la ejecución de golpes con retroalimentación específica | Intermedios y avanzados |

Cada modo puede personalizarse mediante parámetros como la duración total de la sesión, el número de rounds, el tiempo de descanso entre ellos y el grado de exigencia deportiva. Los entrenamientos quedan registrados automáticamente en el historial del usuario, donde pueden consultarse posteriormente para analizar la evolución del rendimiento.

# 10. Lectura e interpretación de métricas

KnockShadow registra un conjunto amplio de métricas relacionadas con la ejecución de golpes y la dinámica del entrenamiento. La aplicación móvil presenta estas métricas mediante gráficas, indicadores numéricos y comparativas históricas, permitiendo al usuario evaluar su progreso de forma objetiva.

Las principales métricas disponibles son las siguientes:

- **Tipo de golpe:** clasificación automática del golpe ejecutado entre las categorías reconocidas por la red neuronal —jab, cross, hook, uppercut, etc.—. La precisión actual de clasificación se sitúa en torno al 85% sobre golpes correctamente ejecutados.
- **Intensidad del impacto:** valor estimado de la fuerza ejercida sobre el saco durante cada golpe, expresado en una escala normalizada propia del sistema.
- **Velocidad del golpe:** estimación de la velocidad del impacto basada en los datos del acelerómetro durante la fase previa al contacto.
- **Cadencia:** número de golpes ejecutados por unidad de tiempo durante la sesión, indicador clave del ritmo de entrenamiento.
- **Precisión técnica:** valoración global del grado de corrección técnica del golpe en función de los patrones aprendidos por el modelo de inteligencia artificial.
- **Distribución por tipo de golpe:** proporción de cada tipo de golpe dentro de la sesión, útil para identificar tendencias de entrenamiento.
- **Calorías estimadas:** estimación del gasto energético total de la sesión, calculada a partir de la intensidad y duración de la actividad, así como del perfil del usuario.
- **Histórico evolutivo:** evolución temporal de las métricas principales a lo largo de las sesiones, mostrando tendencias semanales, mensuales y por períodos definidos.

Es importante señalar que las métricas obtenidas por KnockShadow constituyen estimaciones derivadas de modelos de inteligencia artificial entrenados sobre datos deportivos. Aunque su precisión resulta elevada, deben interpretarse como referencias indicativas y no como mediciones biomecánicas absolutas.

# 11. Mantenimiento y limpieza

Para garantizar el correcto funcionamiento del sistema a lo largo del tiempo y prolongar su vida útil, se recomienda seguir las pautas de mantenimiento descritas en este apartado.

## 11.1 Mantenimiento periódico

- **Inspección estructural mensual:** revisar visualmente la integridad del soporte mural, la tornillería y el cuerpo del saco. Comprobar la ausencia de fisuras, deformaciones, oxidación o aflojamiento de fijaciones.
- **Reapriete trimestral de tornillería:** verificar el par de apriete de los tornillos del soporte mural cada tres meses, especialmente en instalaciones sometidas a uso intensivo. [TODO: par de apriete recomendado]
- **Revisión del revestimiento:** examinar el revestimiento de poliuretano en busca de desgaste superficial, cortes o erosión. En caso de detectar daños, contactar con el servicio técnico para evaluar la sustitución del componente afectado.
- **Verificación del núcleo absorbente:** la espuma EVA de alta densidad sufre fatiga progresiva por impactos repetitivos. En caso de detectar reducción significativa de la capacidad de absorción —percibida como mayor dureza al golpear—, contactar con el servicio técnico para evaluar la sustitución modular del núcleo.
- **Comprobación del estado de los sensores:** comprobar mensualmente desde la aplicación móvil la calibración y el estado de cada sensor inercial. En caso de detectar lecturas anómalas, ejecutar el proceso de recalibración descrito en el apartado correspondiente.

## 11.2 Limpieza del producto

- Limpiar el revestimiento de poliuretano con un paño suave humedecido en agua tibia y detergente neutro. No utilizar disolventes, alcoholes industriales, productos abrasivos ni dispositivos de limpieza a presión.
- Evitar la inmersión total del producto o la entrada de líquidos en zonas próximas a los componentes electrónicos.
- Secar inmediatamente con un paño limpio tras la limpieza, especialmente en zonas próximas a costuras y cremalleras.
- Tras sesiones de entrenamiento intensivo en las que el revestimiento entre en contacto con sudor abundante, realizar una limpieza superficial con paño húmedo y secar de inmediato.

## 11.3 Almacenamiento prolongado

En caso de no utilizar el producto durante períodos prolongados, se recomienda desconectar la alimentación eléctrica, mantener la zona seca y ventilada, y proteger el revestimiento con una funda transpirable. Antes de reanudar el uso, ejecutar una recalibración completa del sistema.

# 12. Resolución de problemas

En este apartado se describen las incidencias más frecuentes que pueden producirse durante el uso del sistema, junto con las acciones correctivas recomendadas. En caso de que el problema persista tras aplicar las medidas descritas, se recomienda contactar con el servicio técnico.

| Problema | Causa probable | Acción correctiva |
| :---- | :---- | :---- |
| El dispositivo no aparece durante el escaneo BLE | Distancia excesiva, interferencias o sistema no alimentado | Acercar el móvil al saco, comprobar la alimentación del sistema, alejar fuentes de interferencia inalámbrica |
| Pérdida intermitente de la conexión Bluetooth | Saturación inalámbrica en el entorno o batería del móvil en modo ahorro | Desactivar el modo de ahorro de energía del móvil, reducir el número de dispositivos Bluetooth próximos |
| Clasificación de golpes incorrecta o errática | Calibración desactualizada, ejecución técnica deficiente o vibraciones del soporte | Ejecutar una recalibración completa, verificar el estado de la instalación mural |
| La aplicación no sincroniza los datos con la nube | Conexión a Internet inestable o caída temporal del servicio cloud | Comprobar la conexión a Internet, esperar unos minutos y reintentar manualmente la sincronización |
| Latencia elevada entre el impacto y la respuesta visual | Saturación de procesamiento en la pasarela o exceso de dispositivos conectados | Reiniciar la pasarela edge y reducir el número de aplicaciones activas en el dispositivo móvil |
| El sistema no detecta impactos | Sensores descalibrados o pérdida de conexión interna | Realizar una recalibración, verificar el estado del sistema desde la aplicación y, si persiste, contactar con soporte técnico |
| La pasarela edge no arranca | Fallo de alimentación o desconexión del cable de red | Verificar la conexión eléctrica, esperar al ciclo completo de arranque y comprobar la integridad de los cables |
| El correo de verificación no se recibe | Filtro de spam, dirección errónea o retraso del servidor de correo | Comprobar la carpeta de correo no deseado y solicitar el reenvío desde la aplicación |
| La aplicación cierra inesperadamente | Versión desactualizada, incompatibilidad o memoria insuficiente | Actualizar la aplicación desde la tienda oficial y reiniciar el dispositivo móvil |
| Ruidos o vibraciones anómalas del soporte | Tornillería floja, anclaje incorrecto o desgaste estructural | Detener el uso inmediatamente, inspeccionar la instalación y, en caso necesario, contactar con un instalador cualificado |

Resulta importante remarcar que cualquier incidencia relacionada con la integridad estructural del producto o del anclaje mural debe motivar la interrupción inmediata del uso hasta su resolución, debido a los riesgos para la seguridad del usuario derivados del fallo mecánico del sistema.

# 13. Garantía y servicio técnico

KnockShadow se encuentra cubierto por la garantía legal aplicable en el marco de la normativa europea de protección al consumidor, así como por las condiciones específicas de garantía comercial establecidas por el fabricante.

**Período de garantía:** [TODO: indicar el período de garantía comercial, p. ej. 24 meses desde la fecha de compra].

La garantía cubre defectos de fabricación, fallos electrónicos no atribuibles al uso indebido del producto y averías mecánicas derivadas de defectos estructurales. Quedan expresamente excluidos de la cobertura los siguientes supuestos:

- Daños derivados de una instalación incorrecta del soporte mural, especialmente si la instalación no ha respetado las condiciones descritas en el presente manual.
- Desgaste habitual del revestimiento de poliuretano y la espuma EVA derivado del uso normal del producto, salvo que dicho desgaste resulte anormalmente prematuro respecto al perfil de uso declarado.
- Daños producidos por la apertura, manipulación o modificación no autorizada del producto.
- Daños derivados del uso del producto en condiciones ambientales no admitidas o en aplicaciones para las que no ha sido diseñado.

## 13.1 Procedimiento de reclamación

Para activar la garantía o solicitar asistencia técnica, el usuario debe seguir los siguientes pasos:

1. Reunir la información del producto: número de serie, fecha de compra y descripción detallada del fallo detectado.
2. Contactar con el servicio técnico oficial a través de los canales descritos en el apartado correspondiente del presente manual.
3. Seguir las instrucciones facilitadas por el equipo de soporte, que podrá solicitar diagnósticos remotos, fotografías o, en su caso, autorizar el envío del producto a las instalaciones del servicio técnico.

En caso de necesitar piezas de repuesto fuera del período de garantía, el fabricante mantendrá disponibilidad de componentes modulares —núcleo absorbente, revestimiento, soportes, electrónica— durante un período mínimo establecido conforme a la normativa europea aplicable.

# 14. Protección de datos personales (RGPD)

El uso de KnockShadow implica el tratamiento de datos personales y deportivos del usuario, almacenados en infraestructura cloud con el objetivo de proporcionar las funcionalidades descritas en el presente manual. Este tratamiento se realiza en cumplimiento del Reglamento General de Protección de Datos (RGPD) y de la normativa española aplicable.

**Datos tratados:** el sistema almacena información de identificación —correo electrónico, nombre, fecha de nacimiento—, características antropométricas básicas declaradas por el usuario y métricas deportivas derivadas del uso del producto.

**Finalidad del tratamiento:** los datos se utilizan exclusivamente con las siguientes finalidades:

- Gestión de la cuenta de usuario y autenticación segura.
- Personalización de la experiencia de entrenamiento.
- Cálculo y visualización de métricas deportivas.
- Mejora progresiva del modelo de inteligencia artificial mediante el uso de datos agregados y anonimizados.
- Cumplimiento de obligaciones legales aplicables.

**Base legal:** el tratamiento se fundamenta en el consentimiento expreso del usuario otorgado durante el proceso de registro y en la ejecución del contrato derivado de la adquisición del producto.

**Conservación de datos:** los datos personales se conservan durante el tiempo necesario para cumplir las finalidades descritas. El usuario puede solicitar en cualquier momento la cancelación de su cuenta y la eliminación de sus datos.

**Derechos del usuario:** el usuario dispone de los derechos de acceso, rectificación, supresión, limitación del tratamiento, portabilidad y oposición previstos en el RGPD. Estos derechos pueden ejercerse mediante solicitud dirigida a [TODO: dirección de contacto del responsable de tratamiento].

**Seguridad de la información:** la arquitectura del sistema implementa medidas técnicas y organizativas conformes al estado del arte, incluyendo autenticación segura, cifrado en tránsito y en reposo y control granular de accesos a los datos.

La política de privacidad completa se encuentra disponible en la aplicación móvil y en el sitio web oficial del fabricante. [TODO: URL exacta]

# 15. Información legal y de cumplimiento normativo

KnockShadow cumple las normativas europeas aplicables a productos electrónicos comercializados en la Unión Europea. La conformidad del producto se acredita mediante el marcado CE, visible sobre el cuerpo del saco y sobre la pasarela edge.

El producto cumple las siguientes directivas y normas armonizadas:

- **Directiva 2014/53/UE (RED):** equipos radioeléctricos. Acredita la conformidad del módulo Bluetooth Low Energy en términos de emisión, estabilidad espectral y ausencia de interferencias.
- **Directiva 2014/30/UE (EMC):** compatibilidad electromagnética. Garantiza que el producto no genera ni resulta afectado por interferencias electromagnéticas dentro de los márgenes establecidos.
- **Norma EN IEC 62368-1:** seguridad eléctrica aplicable a equipos electrónicos y tecnologías de la información y comunicación.
- **Directiva 2011/65/UE (RoHS):** restricción del uso de sustancias peligrosas en componentes electrónicos.
- **Reglamento (CE) 1907/2006 (REACH):** control de sustancias químicas presentes en los materiales utilizados durante la fabricación.

Adicionalmente, la infraestructura cloud sobre la que opera el sistema cumple los requisitos del Reglamento General de Protección de Datos (RGPD) y aplica medidas de ciberseguridad acordes con el estado del arte del sector.

La declaración de conformidad CE completa se encuentra disponible en el sitio web oficial del fabricante. [TODO: URL exacta]

Conforme a la normativa europea sobre gestión de residuos de aparatos eléctricos y electrónicos (RAEE), el producto no debe desecharse junto con la basura doméstica al final de su vida útil. El usuario debe depositarlo en un punto limpio autorizado o entregarlo al distribuidor para su correcta gestión ambiental.

# 16. Contacto y soporte

El equipo de soporte técnico de KnockShadow se encuentra disponible para atender consultas relacionadas con la instalación, configuración, uso y mantenimiento del producto. Los canales de contacto habilitados son los siguientes:

- **Soporte por correo electrónico:** [TODO: dirección de soporte oficial]
- **Soporte telefónico:** [TODO: número de teléfono y horario de atención]
- **Soporte web:** [TODO: URL del portal de soporte]
- **Comunidad online de usuarios:** [TODO: URL del foro o red social oficial]
- **Redes sociales oficiales:** [TODO: cuentas oficiales en redes]

Se recomienda contactar con el servicio técnico aportando la siguiente información, con el fin de agilizar la resolución de la consulta:

- Número de serie del producto.
- Fecha de compra y canal de adquisición.
- Versión de la aplicación móvil instalada.
- Descripción detallada de la incidencia o consulta.
- Capturas de pantalla o vídeos breves que ilustren el problema, en caso de aplicar.

# 17. Glosario de términos

Con el fin de facilitar la comprensión del presente manual, se recogen a continuación los principales términos técnicos utilizados a lo largo del documento.

| Término | Definición |
| :---- | :---- |
| IoT | Internet de las Cosas. Conjunto de tecnologías que permiten la conexión de objetos físicos a redes de comunicación |
| IMU | Unidad de Medición Inercial. Sensor capaz de medir aceleraciones lineales y velocidades angulares |
| BLE | Bluetooth Low Energy. Estándar de comunicación inalámbrica de bajo consumo |
| MQTT | Message Queuing Telemetry Transport. Protocolo ligero de mensajería utilizado en sistemas IoT |
| Edge computing | Procesamiento de datos cerca del origen, sin depender exclusivamente de infraestructura cloud |
| Cloud computing | Infraestructura de computación basada en servicios remotos accesibles a través de Internet |
| Pasarela edge | Dispositivo que actúa como puente entre los sensores locales y la infraestructura cloud |
| Red neuronal | Modelo de inteligencia artificial inspirado en el funcionamiento del sistema nervioso, utilizado para tareas de clasificación |
| Aprendizaje continuo | Capacidad del modelo de IA para mejorar progresivamente mediante la incorporación de nuevos datos de entrenamiento |
| Calibración | Proceso de ajuste de los sensores y modelos del sistema a las condiciones específicas de instalación y al perfil del usuario |
| RGPD | Reglamento General de Protección de Datos. Normativa europea relativa al tratamiento de datos personales |
| Marcado CE | Certificación de conformidad obligatoria para productos comercializados en la Unión Europea |
| Espuma EVA | Etileno-vinil-acetato. Material polimérico flexible y absorbente utilizado en aplicaciones deportivas |
| Poliuretano (PU) | Material polimérico resistente al desgaste, utilizado como revestimiento exterior |
| RAEE | Residuos de Aparatos Eléctricos y Electrónicos. Categoría de residuos sometida a regulación específica de reciclaje |
