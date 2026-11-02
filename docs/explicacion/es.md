# NicheRS explicado desde cero

Este documento explica, sin dar nada por sabido, qué problema científico resuelve NicheRS, cómo
lo resuelve, cómo sabemos si el resultado es fiable y cómo hemos llegado hasta el punto actual
(20 de octubre de 2026, rama `feat/science-v1`, con el programa Science v1 completo, fases 1 a 9). Si
quieres ir al detalle técnico, cada apartado indica el documento de referencia.

Versiones: [español](es.md) · [English](en.md) · [français](fr.md).

**Cómo leerlo.** Si tienes un minuto: «En una página». Si tienes cinco: §1, el recorrido de §2.4, los resultados reales de §5.7 y las preguntas frecuentes de
§10. Si quieres entender la ciencia: §3 a §5. Si quieres saber en qué punto está el proyecto: §7 y
§8.

---

## En una página

- **Qué hace.** A partir de los sitios donde se ha visto una especie (GBIF) y del clima de cada
  kilómetro cuadrado (CHELSA), NicheRS dibuja un mapa de dónde se dan condiciones parecidas.
- **Qué no hace.** No dice dónde *está* la especie ni da una probabilidad: da una **idoneidad
  relativa** (0 a 1) y una **superficie de condiciones favorables**, en forma de horquilla.
- **Cómo se fía uno.** Cada resultado se examina escondiendo regiones enteras y comprobando si el
  modelo las acierta. El resultado se resume en un **nivel de evidencia** (Insuficiente,
  Exploratorio, Indicativo, Respaldado y, con datos de campo, Validado) con sus motivos, nunca en un porcentaje inventado.
- **El motor.** MaxEnt, el método de referencia, reescrito en Rust y comprobado contra el original:
  16 de 16 casos idénticos. Un estudio de una especie en toda Francia tarda segundos.
- **Con datos reales.** Las especies con un nicho climático claro (encina) salen *Respaldado*; las
  generalistas (erizo, víbora) salen *Exploratorio* cuando solo se usa el clima. Es la respuesta
  honesta, no un fallo.
- **Science v1 (terminado el 20 de octubre).** Se han añadido, como opciones de experto pendientes
  de revisión por ecólogos: corrección del sesgo de observación, pruebas contra el azar, relieve,
  vegetación y agua, un segundo método (Random Forest), proyecciones al clima futuro, datos de campo
  propios con validación independiente (nivel *Validado*) y salidas para decidir: dónde prospectar,
  qué toca un proyecto, zonas con muchas especies y comparación de estudios.
- **Lo que falta.** Ninguno de los métodos nuevos se ha usado todavía de principio a fin con una
  especie real para sacar una conclusión real. Antes de usarlos en un informe para un cliente hacen
  falta la revisión de los ecólogos de Natural Solutions y pruebas con especies que conozcan.

---

## 1. La pregunta en una frase

> **«¿En qué lugares de esta zona se dan las condiciones que le gustan a esta especie, y cuánto
> podemos fiarnos de la respuesta?»**

Un ejemplo real: una consultora ecológica tiene que evaluar el impacto de un parque eólico en
Bretaña. Necesita saber dónde podría haber lagarto verde (*Lacerta bilineata*), aunque nadie lo
haya buscado en cada parcela. No hay tiempo ni dinero para recorrer cada kilómetro cuadrado, así que
se usa un **modelo de distribución de especies** (en inglés *Species Distribution Model*, SDM): a
partir de los sitios donde *sí* se ha visto la especie y del clima de toda la zona, el modelo estima
qué otros sitios tienen condiciones parecidas.

NicheRS es una herramienta (una aplicación web) que hace eso de principio a fin, para personas que
no son especialistas en modelización.

---

## 2. Las ideas básicas

### 2.1 El nicho ecológico

Cada especie tolera un rango de condiciones: temperaturas, lluvia, estacionalidad, altitud, tipo de
vegetación… Una encina aguanta veranos secos y calurosos; un haya necesita humedad. Ese conjunto de
condiciones en las que una especie puede vivir se llama **nicho ecológico**. (De ahí el nombre:
*Niche* + *RS*, por el lenguaje de programación Rust.)

La idea central de los SDM es sencilla:

1. Miramos **qué condiciones** hay en los sitios donde se ha observado la especie.
2. Buscamos en el mapa **otros sitios con condiciones parecidas**.

### 2.2 Por qué no basta con «mirar el mapa de observaciones»

Porque las observaciones dicen dónde ha mirado la gente, no dónde vive la especie:

- Hay muchos más datos cerca de ciudades, carreteras y reservas naturales que en el monte.
- Una zona sin observaciones puede ser una zona sin especie… o una zona que nadie ha visitado.
- Hay errores: animales de zoo, plantas de jardín, fósiles, coordenadas mal puestas.

El modelo intenta separar «dónde le gustan las condiciones» de «dónde se ha mirado». No lo consigue
del todo, y por eso una parte grande del trabajo es **medir y comunicar honestamente** lo bien o
mal que lo consigue.

### 2.3 El mapa como una cuadrícula

Todo se trabaja sobre una **rejilla de celdas** de unos 1 km de lado (exactamente 30 segundos de
arco, la rejilla nativa de los datos de clima). Francia entera son unas 930 000 celdas. Para cada
celda conocemos sus condiciones (temperatura media, lluvia…) y al final el modelo le da una
puntuación.

### 2.4 El recorrido de un estudio, de un vistazo

```text
 1. Especie + zona          «Lagarto verde, en Bretaña»
        │
 2. Observaciones (GBIF)    miles de registros ──► limpieza ──► celdas con presencia
        │                                        (motivo de cada exclusión, visible)
 3. Ambiente (CHELSA…)      clima de cada celda (+ relieve, vegetación, agua: opcional)
        │                   máscara de tierra · filtro de variables redundantes
 4. Fondo                   10 000 celdas al azar con las que comparar
        │
 5. Modelo (MaxEnt)         25 configuraciones probadas, examen por bloques geográficos
        │                   ──► se elige una con reglas fijas ──► modelo final
 6. Evaluación              notas del examen, incertidumbre, extrapolación
        │
 7. Resultado               mapa de idoneidad relativa · superficie favorable (horquilla)
                            nivel de evidencia + motivos · informe · manifiesto
```

Cada flecha deja rastro: qué se quitó, qué se eligió y por qué. Ese rastro es lo que permite a un
ecólogo revisar el estudio y a cualquiera repetirlo.

---

## 3. Los datos que usamos

### 3.1 Dónde se ha visto la especie: GBIF

**GBIF** (*Global Biodiversity Information Facility*) es la gran base de datos mundial de
observaciones de especies: museos, programas científicos, aplicaciones de ciencia ciudadana como
iNaturalist u Observation.org… Cada registro dice «esta especie, en estas coordenadas, en esta
fecha». Son datos de **solo presencia**: dicen dónde se ha visto, nunca dónde se ha buscado sin
encontrarla.

Antes de usarlos, NicheRS los **limpia** con reglas fijas (decididas en ADR-0009), y siempre muestra
cuántos registros quita y por qué:

| Se excluye | Por qué |
|---|---|
| Fósiles, animales en cautividad, plantas cultivadas | No reflejan dónde vive la especie en libertad. |
| Registros con licencia «no comercial» (CC BY-NC) | No se pueden usar en un informe de consultoría que se cobra. |
| Registros sin fecha o anteriores a 1981 | El clima que usamos es el de 1981–2010; un dato de 1900 puede ser de otro clima. |
| Coordenadas con incertidumbre > 1 km | Si el punto puede estar a 5 km, no sabemos en qué celda cae. |
| Coordenadas «difuminadas» por el publicador sin precisión declarada (nuevo, ADR-0011) | Las especies sensibles (p. ej. aves rapaces nidificantes) se publican con la posición movida a propósito para protegerlas. Si no sabemos cuánto, el punto engaña al modelo. |
| Puntos en el mar o en grandes lagos | Trabajamos con especies terrestres. |
| Duplicados en la misma celda | Diez fotos del mismo lagarto en el mismo sitio no son diez lagartos. Se deja uno por celda. |

Una nota sobre el rango taxonómico: si el usuario elige un **género** en vez de una **especie**
(p. ej. «Quercus» en vez de «Quercus ilex»), se estarían mezclando varias especies con nichos
distintos. La herramienta lo avisa y limita el nivel de confianza (lo añadimos el 16 de octubre
después de ver que pasaba en un caso real).

### 3.2 Cómo es el ambiente: el clima CHELSA

Para describir las condiciones de cada celda usamos **CHELSA v2.1**, un conjunto de mapas
climáticos mundiales a 1 km. Por defecto se usan siete «variables bioclimáticas»:

- temperatura media anual (BIO1), su estacionalidad (BIO4), máxima del mes más cálido (BIO5),
  mínima del mes más frío (BIO6);
- lluvia anual (BIO12), lluvia del mes más seco (BIO14) y su estacionalidad (BIO15).

Es decir: calor, agua, cómo cambian a lo largo del año y sus extremos.

¿Por qué CHELSA y no WorldClim, que es el más conocido? Porque **WorldClim prohíbe el uso
comercial** sin permiso, y CHELSA es de dominio público (CC0). Natural Solutions es una empresa:
todos los datos por defecto tienen que poder usarse en trabajo pagado (ADR-0006).

Dos detalles que descubrimos por el camino:

- **CHELSA tiene valores en el mar** (por ejemplo, 13,65 °C en mitad del golfo de Vizcaya). Si no
  se tapa el mar, el modelo compararía la especie con condiciones marinas y parecería mejor de lo que
  es. Por eso aplicamos una **máscara de tierra** (Natural Earth) — ADR-0010.
- Algunas variables están muy **correlacionadas** entre sí (si hace calor en verano, suele hacer
  calor de media). Dar al modelo dos variables que dicen lo mismo lo confunde. Se aplica un filtro:
  si dos variables se parecen demasiado (correlación > 0,7), se queda la primera.

### 3.3 Más allá del clima: relieve, vegetación y agua (opcional)

A 1 km, el clima dice si una región es adecuada, pero no si dentro de ella hay bosque, campo o
ciudad. Por eso muchas especies generalistas salían *Exploratorias*. Desde la fase 4 de Science v1
(ADR-0015) se pueden añadir, en modo Experto, doce variables más, todas con licencia comercial:

| Grupo | Variables | Fuente |
|---|---|---|
| Relieve | altitud, pendiente, rugosidad | Copernicus DEM (90 m) |
| Cobertura del suelo | fracción de cada celda cubierta por árboles, matorral, pasto, cultivo, construido, suelo desnudo, agua, humedal | ESA WorldCover 2021 (10 m) |
| Agua | distancia al agua más cercana | ESA WorldCover 2021 |

Se usan **fracciones** («esta celda es 60 % bosque y 30 % cultivo») en vez de una sola clase («esta
celda es bosque»), porque a 1 km casi todas las celdas son mezclas. Todas se calculan sobre la misma
rejilla que el clima, sin deformarla.

Por defecto **no se selecciona ninguna**: un estudio normal sigue siendo solo clima. Y cada una lleva
sus avisos: la cobertura del suelo es una foto de 2021 mientras que las observaciones abarcan desde
1981; lo «construido» puede reflejar dónde vive la gente que observa más que dónde vive la especie;
el modelo de elevación mide la superficie (copas de árboles, tejados), no el suelo.

### 3.4 El «fondo»: con qué comparamos

Como solo sabemos dónde *está* la especie y no dónde *no está*, el modelo compara las celdas con
observaciones contra una muestra de **10 000 celdas al azar** de la zona de estudio, llamada
**fondo** (*background*). La pregunta que se hace el modelo no es «¿está aquí la especie?», sino
«¿en qué se diferencian los sitios donde se ha visto la especie de los sitios disponibles en
general?».

---

## 4. El modelo: MaxEnt

### 4.1 La idea, sin fórmulas

**MaxEnt** (*Maximum Entropy*, Phillips et al., 2006) es el método más usado en el mundo para este
problema. Su lógica:

> «De todas las maneras de repartir la especie por el mapa que son coherentes con lo que hemos
> observado (por ejemplo, que la temperatura media donde se ha visto es de unos 12 °C), escoge la
> más repartida, la que menos supone.»

Es una forma prudente de no inventarse preferencias que los datos no muestran. Para describir las
preferencias, MaxEnt puede usar formas simples (lineales: «cuanto más calor, mejor») o más complejas
(«le gusta entre 10 y 14 °C, ni más ni menos»). Cuanta más complejidad permitimos, más se ajusta a
los datos… y más riesgo de que memorice el ruido en vez de aprender la regla (**sobreajuste**). Un
parámetro llamado **regularización** (β) penaliza la complejidad.

### 4.2 Por qué hemos rehecho MaxEnt en Rust

El programa de referencia es `maxent.jar`, escrito en Java, gratuito y congelado desde 2020. Los
ecólogos confían en él. Pero exige Java, es difícil de integrar en un servicio web y es lento cuando
se encadenan cientos de ajustes.

Decidimos **reescribir (portar) MaxEnt en Rust**, pero con una condición estricta: tiene que dar
**exactamente los mismos resultados** que `maxent.jar`. No «parecidos»: los mismos. Eso obligó a
copiar incluso las manías internas del programa original (el orden en que suma, cómo desempata,
cálculos que reutiliza aunque estén algo desfasados), porque MaxEnt se detiene antes de llegar al
óptimo matemático, y solo copiando su camino exacto se llega al mismo sitio (ADR-0001).

Resultado medido: **16 de 16 casos de prueba idénticos** a `maxent.jar` 3.4.4, incluidas 5 especies
reales, con diferencias del orden de 0,00000000005. Cada cambio en el motor se vuelve a comprobar
contra el original (`scripts/compare_maxent.py`); si no da 100 %, no entra.

### 4.3 Ajuste automático: elegir la mejor configuración

No hay una configuración de MaxEnt buena para todas las especies. NicheRS prueba **25
configuraciones** (5 niveles de complejidad × 5 niveles de regularización) y elige una con criterios
fijos, de forma transparente:

1. la que **menos se equivoca con datos que no ha visto** (tasa de omisión, ver §5);
2. en caso de empate, la que mejor **ordena** los sitios (índice de Boyce);
3. en caso de empate, la más **sencilla**.

Este criterio favorece modelos prudentes y «suaves», que funcionan mejor cuando se aplican a zonas
nuevas, a costa de parecer algo menos precisos sobre los propios datos. Es una decisión consciente.

### 4.4 Qué significa el número del mapa

El modelo da a cada celda un valor entre 0 y 1. **No es una probabilidad de presencia.** Es una
**idoneidad relativa**: 0,8 quiere decir «condiciones más parecidas a las de las observaciones que
una celda con 0,3», no «80 % de probabilidad de encontrar la especie». Con datos de solo presencia
es imposible calcular una probabilidad real, porque no sabemos cuántas veces se buscó sin éxito.

Esta regla está escrita (ADR-0007) y es innegociable: en ningún texto, mapa o informe de NicheRS
aparece «probabilidad» ni un «porcentaje de confianza» inventado.

---

## 5. ¿Cómo sabemos si el modelo es bueno?

Esta es la parte científicamente más delicada, y donde más esfuerzo hemos puesto.

### 5.1 Examinar con preguntas que no ha visto: validación cruzada

Si evalúas a un alumno con las mismas preguntas que estudió, sacará un 10 aunque no haya entendido
nada. Con los modelos pasa igual. Por eso se **esconde una parte de los datos**, se entrena con el
resto y se mira si el modelo acierta en lo escondido. Se repite varias veces cambiando la parte
escondida: es la **validación cruzada**.

### 5.2 Esconder por zonas, no al azar: bloques espaciales

Si escondemos puntos al azar, cada punto escondido tiene un vecino casi idéntico en los datos de
entrenamiento (a 500 m, con el mismo clima), y el examen es demasiado fácil. Por eso se divide el
territorio en **4 bloques geográficos** (cortando por la mediana de latitud y de longitud) y se
esconde un bloque entero cada vez. Así se comprueba si el modelo sabe **trasladarse a otra zona**,
que es justo lo que se le pide en la práctica.

### 5.3 Las notas del examen

- **Tasa de omisión al 10 % (OR10).** Se fija un umbral que deja fuera el 10 % de las observaciones
  de entrenamiento con peor puntuación, y se mira qué parte de las observaciones *escondidas* queda
  por debajo. Si el modelo no está sobreajustado, debería ser ≈ 0,10. Si es 0,40, el modelo no
  reconoce sitios nuevos donde sí está la especie.
- **Índice de Boyce.** Mira si, en las zonas que el modelo puntúa alto, hay de verdad más
  observaciones de las que habría por azar. Va de −1 a 1: 0 es «como tirar una moneda», cerca de 1
  es «muy coherente».
- **AUC.** Es la métrica más famosa, pero con datos de solo presencia mide sobre todo **lo grande que
  es la zona de estudio**, no la calidad del modelo. La mostramos en el detalle, pero no la usamos
  para decidir nada.

### 5.4 Incertidumbre y extrapolación

- **Mapa de incertidumbre:** cuánto cambia la predicción de una celda entre los modelos de la
  validación cruzada. Si cambia mucho, ese resultado depende de qué datos se usaron.
- **Extrapolación (MESS):** zonas cuyo clima es **más extremo que cualquiera de los datos usados**
  (p. ej. cumbres más frías que cualquier punto observado). Ahí el modelo está adivinando. Se marcan
  con un rayado en el mapa, activado por defecto, y se informa del porcentaje de superficie afectada.

### 5.5 Umbrales y superficie favorable

Para decir «hay X km² de condiciones favorables» hay que fijar un corte: por encima, «favorable».
No hay un único corte correcto, así que damos una **horquilla con tres reglas** conocidas (la más
inclusiva, la del 10 % — la usada por defecto — y la que equilibra aciertos y errores). Las áreas se
calculan con la fórmula exacta de la superficie de cada celda sobre el elipsoide terrestre. Siempre
se dice «superficie de condiciones favorables», nunca «área ocupada por la especie».

### 5.6 El nivel de evidencia: el resumen honesto

En vez de un porcentaje de confianza, cada resultado lleva un **nivel de evidencia** calculado con
reglas escritas (ADR-0009), y la lista de motivos que lo justifican:

| Nivel | Qué significa | Condiciones (todas) |
|---|---|---|
| **Insuficiente** | No se ajusta modelo. | Menos de 10 celdas con presencia. |
| **Exploratorio** | Sirve para orientar, no para decidir. | Hay modelo, pero falla algún criterio de «Indicativo». |
| **Indicativo** | Utilizable con prudencia. | ≥ 25 celdas; Boyce ≥ 0,40; OR10 ≤ 0,25; ≤ 25 % de zona extrapolada. |
| **Respaldado** | Lo mejor que se puede decir con estos datos. | ≥ 50 celdas; Boyce ≥ 0,70; OR10 ≤ 0,15; ≤ 10 % extrapolada; validación por bloques espaciales. |
| **Validado** | El mapa acierta con datos de campo que nunca vio. | Todo lo de «Respaldado» y, además, datos de campo propios **apartados** del entrenamiento: ≥ 30 sitios de presencia y ≥ 30 de ausencia; AUC presencia–ausencia ≥ 0,75; TSS ≥ 0,40. |

El nivel **Validado** (fase 7, ADR-0108) es el único que no depende solo de los registros con los
que se entrenó el modelo: exige comparar el mapa con datos de campo **independientes** de la
organización (sitios donde se buscó la especie y se encontró o no se encontró), explicados en §8. Si
no se aportan esos datos, el resultado lo dice explícitamente: «sin datos de validación
independientes: la evidencia viene de la validación cruzada de los mismos registros, posiblemente
sesgados».

Consecuencia esperada: muchas especies con datos oportunistas saldrán *Exploratorio* o
*Indicativo*. Por ejemplo, la encina (*Quercus ilex*) en Francia sale *Respaldado*; la víbora áspid
(*Vipera aspis*), una especie generalista, sale *Exploratorio*. Eso no es un fallo de la
herramienta: es la respuesta honesta.

### 5.7 Lo que nos enseñaron diez especies reales

El 15 de octubre se modelaron diez especies en toda Francia continental a 1 km, solo con clima y con
los ajustes por defecto (`docs/benchmarks/g6-france-batch.md`). Se usaron como máximo 20 000
registros por especie, tomados de forma repartida por el territorio:

| Especie | Nivel | Boyce | OR10 | Superficie favorable (umbral del 10 %) |
|---|---|---|---|---|
| Encina (*Quercus ilex*) | Respaldado | 0,74 | 0,12 | 110 166 km² |
| Ardilla roja (*Sciurus vulgaris*) | Respaldado | 0,84 | 0,11 | 372 868 km² |
| Ciervo volante (*Lucanus cervus*) | Respaldado | 0,73 | 0,14 | 378 774 km² |
| Haya (*Fagus sylvatica*) | Indicativo | 0,90 | 0,19 | 330 204 km² |
| Sapo espinoso (*Bufo spinosus*) | Indicativo | 0,62 | 0,06 | 289 513 km² |
| Rana ágil (*Rana dalmatina*) | Indicativo | 0,60 | 0,11 | 290 522 km² |
| Salamandra común (*Salamandra salamandra*) | Exploratorio | 0,32 | 0,09 | 428 067 km² |
| Erizo europeo (*Erinaceus europaeus*) | Exploratorio | 0,27 | 0,06 | 408 759 km² |
| Víbora áspid (*Vipera aspis*) | Exploratorio | 0,23 | 0,10 | 469 518 km² |
| Lagarto verde (*Lacerta bilineata*) | Exploratorio | 0,17 | 0,05 | 431 716 km² |

Tres lecciones:

1. **Las especies con un nicho climático claro salen bien; las generalistas, no.** La encina es
   mediterránea y el clima la explica. El erizo o la víbora viven en casi toda Francia: el clima a
   1 km no distingue dónde están, y la herramienta no lo disimula. Para ellas hacen falta otras
   variables (cobertura del suelo, §3.3) o datos de campo.
2. **Una sola buena nota no basta.** El haya tiene el mejor índice de Boyce (0,90), pero su OR10
   (0,19) muestra que, al esconder una región entera, el modelo deja fuera demasiadas observaciones:
   le cuesta trasladarse a climas que no ha visto. Por eso se queda en *Indicativo*.
3. **La cantidad de datos cambia el resultado.** Al pasar de 20 000 registros a todos los registros
   disponibles (con las descargas masivas de GBIF), el Boyce del erizo subió de 0,27 a 0,80 y el
   del lagarto verde de 0,17 a 0,70 (`docs/benchmarks/gbif-batch-downloads.md`). Más registros
   también arrastran más sesgo de observación, por eso los avisos de sesgo se mantienen.

Las superficies son de **condiciones favorables**, no de presencia: que el lagarto verde tenga
431 716 km² con condiciones adecuadas no significa que ocupe toda esa superficie.

---

## 6. Qué entrega NicheRS

Al terminar un estudio el usuario obtiene:

- el **mapa de idoneidad relativa**, con capas de incertidumbre y extrapolación;
- la **horquilla de superficie favorable**;
- el **nivel de evidencia** con sus motivos y todos los avisos (los avisos nunca se pueden ocultar);
- la **importancia de cada variable** y las **curvas de respuesta** («a partir de qué temperatura le
  gusta menos»);
- un **informe de métodos** imprimible, con referencias bibliográficas;
- exportaciones (GeoTIFF, GeoJSON, el modelo en formato `.lambdas` de MaxEnt);
- un **manifiesto de ejecución**: un fichero que registra exactamente qué datos (con sus sumas de
  verificación y el DOI de la descarga GBIF), qué versiones, qué semilla aleatoria y qué ajustes se
  usaron. Con él, cualquiera puede repetir el estudio y obtener el mismo resultado, bit a bit.

---

## 7. Cómo hemos llegado hasta aquí

### Fase 0 — Investigar antes de programar (15 oct 2026)

Antes de escribir código se hizo una revisión del estado del arte (14 documentos en
`docs/research/`): qué métodos funcionan, qué herramientas existen (MaxEnt, biomod2, ENMeval,
Wallace…), qué datos se pueden usar legalmente, qué lenguaje elegir. Conclusiones principales:

- MaxEnt sigue siendo una referencia sólida; el segundo motor debería ser un *Random Forest*
  equilibrado (ver fase 1b).
- Los riesgos principales **no son técnicos**: son el sesgo de los datos, que la gente lea la
  idoneidad como probabilidad, la extrapolación silenciosa y las licencias de los datos.
- Se pusieron «puertas» (*gates*) medibles: si el motor en Rust no reproducía `maxent.jar`, el plan
  B era usar el propio `maxent.jar`.

El mismo día se demostró el port de MaxEnt (11/11 casos sintéticos idénticos; después 16/16 con
especies reales).

### MVP — El producto mínimo completo (15–16 oct)

Se construyó toda la cadena: descarga de GBIF y CHELSA, limpieza, modelo, validación, mapas, informe,
lotes de muchas especies, trabajos que se reanudan si se corta la luz. Las decisiones científicas
por defecto (limpieza, selección, niveles de evidencia) se fijaron en el ADR-0009. Se cumplieron los
14 criterios de «terminado» del MVP (`docs/mvp-status.md`).

Medidas reales: un estudio completo de una especie en toda Francia a 1 km tarda **5–12 s** en un
portátil de 8 núcleos. Un lote de 10 especies pasó de **22,5 min** (API de búsqueda de GBIF) a
**3 min 10 s** en el servidor una vez preparados los datos de GBIF (la primera descarga depende de
la cola de GBIF). Para 100 especies solo hay una extrapolación (~25 min de modelado), **no una
medida**.

### Producto usable por otros (16–19 oct)

Lo que convirtió el prototipo en algo que un equipo puede usar, sin cambiar la ciencia:

- cuentas, organizaciones, proyectos y cuotas; consola de administración;
- interfaz y ayuda en **inglés, francés y español**;
- dos caminos para crear un estudio: **Rápido** (tres pasos, sin saber de modelización) y
  **Experto**; una lectura simplificada de los resultados para no especialistas;
- zonas de estudio por nombre (municipios, departamentos, espacios protegidos oficiales);
- un **espejo regional de GBIF** para Francia y España: los registros llegan en segundos en vez de
  esperar la cola de GBIF (ADR-0114). Este espejo, además, guarda cuántas observaciones hay de cada
  grupo de especies en cada celda — algo que luego sirve para corregir el sesgo (§8);
- privacidad (sin nombres de observadores en caché), despliegues sin cortes, informes de errores.

### Revisión científica (20 oct)

El 20 de octubre se revisó el código y se constató que **el núcleo científico no había cambiado
desde el MVP**, mientras que sus debilidades para uso profesional eran precisamente científicas:

1. no se corrige el **sesgo de muestreo** (se modela en parte dónde mira la gente);
2. solo se usa **clima** (ni relieve, ni vegetación, ni agua);
3. no hay **validación independiente**;
4. la salida es un mapa por especie, no una respuesta a la **decisión del cliente** (¿dónde muestrear?
   ¿afecta este proyecto?).

De ahí nace el programa **Science v1** (`docs/science-v1-plan.md`), en el que estamos ahora.

---

## 8. Science v1: lo que se ha añadido

### Reglas del programa

- **Los valores por defecto no cambian.** Cada método nuevo es **opcional** (modo Experto). Un
  estudio con los ajustes por defecto da resultados **idénticos bit a bit** a los de antes; lo hemos
  comprobado comparando ficheros.
- **El motor MaxEnt no se toca.** La equivalencia con `maxent.jar` sigue al 100 %.
- Cada método nuevo tiene un ADR con estado **«Propuesto — pendiente de revisión por los ecólogos de
  Natural Solutions»**. Convertirlo en valor por defecto será una decisión posterior, tras esa
  revisión.

### Fases

| Fase | Qué es | Estado |
|---|---|---|
| 1a | Piezas de evaluación: aclareo por distancia, bloques en cuadrícula, autocorrelación espacial, área de aplicabilidad, modelos nulos, métricas de presencia–ausencia | Hecha |
| 1b | Segundo motor: *Random Forest* equilibrado | Hecha |
| 1c | Datos: marcas de coordenadas difuminadas, cuadrículas de esfuerzo de muestreo, grupo taxonómico, clima futuro CHELSA CMIP6 | Hecha |
| 1d | Predictores: relieve (Copernicus DEM), cobertura del suelo (ESA WorldCover), distancia al agua, cambios de proyección | Hecha |
| 2 | **Sesgo y limpieza**: coordenadas difuminadas, fondo de grupo objetivo, capa de esfuerzo, aclareo | Hecha (ADR-0011, ADR-0013) |
| 3 | **Evaluación**: modelos nulos, bloques según autocorrelación, área de aplicabilidad | Hecha (ADR-0014) |
| 4 | **Predictores**: relieve, cobertura del suelo y agua como variables seleccionables (§3.3) | Hecha (ADR-0015) |
| 5 | **Clima futuro**: proyecciones, mapas de cambio y acuerdo entre modelos climáticos | Hecha (ADR-0016) |
| 6 | **Segundo motor en la aplicación**: Random Forest y conjunto MaxEnt + RF | Hecha (ADR-0017) |
| 7 | **Datos de campo propios**: importación, validación independiente y nivel *Validado* | Hecha (ADR-0108) |
| 8 | **Salidas para decidir**: prioridad de prospección, huella de proyecto, varias especies, comparación de estudios | Hecha (ADR-0018) |
| 9 | Documentación: estado del MVP, resumen ejecutivo, hoja de ruta, nota en francés para los ecólogos | Hecha |

### Los métodos nuevos, explicados

**Modelos nulos (fase 3).** ¿Es el modelo mejor que el azar? Se entrena el mismo modelo, con los
mismos ajustes, sobre puntos colocados **al azar** (20, 50 o 100 veces) y se compara: si el modelo
real no supera a los aleatorios en al menos el 95 % de los casos, el nivel de evidencia baja a
*Exploratorio*. Es como comprobar que un vidente acierta más que alguien que adivina.

**Bloques según la autocorrelación (fase 3).** Los 4 bloques por la mediana tienen un tamaño
arbitrario. Lo correcto es que los bloques sean al menos tan grandes como la distancia a la que el
clima «se parece a sí mismo» (la **autocorrelación espacial**). Ahora el tamaño se puede calcular a
partir de los propios datos de clima (variograma) y se divide el territorio en una cuadrícula de
bloques de ese tamaño.

**Área de aplicabilidad (AOA, fase 3).** La extrapolación MESS mira cada variable por separado. El
AOA mira también **combinaciones nuevas**: un sitio puede tener una temperatura normal y una lluvia
normal, pero una combinación de las dos que no aparece en ningún dato (por ejemplo, calor con mucha
lluvia). Se muestra como otra capa del mapa.

**Coordenadas difuminadas (fase 2, ADR-0011).** Las especies sensibles se publican con la posición
movida a propósito (iNaturalist hasta ~27 km, PatriNat 5 km). Antes, si el registro no indicaba su
incertidumbre, se usaba como si fuera exacto. Ahora, si el publicador indica que lo difuminó y no
dice cuánto (o dice más de 1 km), se excluye. Limitación: solo la API de búsqueda de GBIF da esta
información; las descargas masivas y el espejo regional no la dan, y la herramienta lo avisa.
Probado con datos reales: en un estudio de la tortuga mediterránea (*Testudo hermanni*) en Provenza,
339 de 1 509 registros venían marcados como difuminados; la herramienta los contó, avisó y explicó
el motivo de cada exclusión, sin cambiar el número de presencias ni el nivel de evidencia de ese
caso.

**Fondo de grupo objetivo (fase 2, ADR-0013).** Es la corrección del sesgo de muestreo más
recomendada en la literatura (Phillips et al., 2009). En vez de comparar las observaciones con
celdas al azar, se comparan con celdas **donde se han observado otras especies del mismo grupo**
(p. ej. otras aves si estudiamos un ave). Así, presencias y fondo tienen el mismo sesgo («la gente
pasea cerca de las ciudades») y ese sesgo se anula en parte. Los datos salen de las cuadrículas de
esfuerzo del espejo regional. Si no hay datos suficientes, se vuelve al fondo aleatorio **con un
aviso**: un estudio nunca falla porque la corrección no esté disponible.

**Aclareo por distancia (fase 2).** Si hay 200 observaciones en un mismo parque y 5 en el resto de
la región, el modelo aprende «el parque». El aclareo deja solo observaciones separadas al menos una
distancia dada (p. ej. 5 km).

**Random Forest equilibrado (fases 1b y 6, ADR-0017).** Un segundo método, muy distinto de MaxEnt: miles
de «árboles de decisión» («si la temperatura > 12 °C y la lluvia < 800 mm, entonces…») que votan. La
versión por defecto funciona mal con datos de solo presencia; la versión **equilibrada** (cada árbol
ve tantos puntos de fondo como de presencia) está entre los mejores métodos en las comparaciones
publicadas (Valavi et al., 2022). Como no existe un programa de referencia que se pueda copiar bit a
bit, se valida con especies sintéticas cuyo nicho real conocemos: el modelo recupera las variables
importantes y los óptimos correctos. Tener dos métodos distintos también permite ver **dónde
discrepan**, lo cual es información útil.

Desde la fase 6, en modo Experto se puede elegir el algoritmo: **MaxEnt** (recomendado, por
defecto), **Random Forest** o un **conjunto** de los dos, que promedia sus clasificaciones de las
celdas. Los tres pasan exactamente por el mismo examen (mismos bloques, mismas notas, modelos nulos,
extrapolación, nivel de evidencia), así que se pueden comparar. En lugar de curvas de respuesta, el
Random Forest muestra la *dependencia parcial*, que se lee igual («cómo cambia la idoneidad al
variar la temperatura»). En la demo sintética, con bloques espaciales, el índice de Boyce fue 0,53
con MaxEnt, 0,47 con Random Forest y 0,52 con el conjunto; el estudio tardó 5,0 s, 2,7 s y 7,9 s.
Es solo una prueba con datos sintéticos: **todavía no se ha comparado con especies reales**. Ya se
sabe que los árboles extrapolan peor que MaxEnt fuera de las condiciones observadas, y que el
resultado del Random Forest varía algo según la semilla aleatoria (Boyce 0,51 frente a 0,47 con dos
semillas en la demo), algo que la herramienta todavía no muestra al usuario.

**Relieve, vegetación y agua (fase 4, ADR-0015).** Explicado en §3.3. Probado en un recorte real de
Provenza (unos 1 000 km²): preparar las capas tarda unos 12 s la primera vez y centésimas de segundo
después, gracias a la caché. Lo que **aún no se ha medido** es cuánto mejoran los modelos de
especies reales al añadir la cobertura del suelo; es la siguiente comprobación pendiente.

**Clima futuro (fase 5, ADR-0016).** El modelo ya ajustado se aplica al clima futuro de CHELSA
CMIP6: tres periodos (2011–2040, 2041–2070, 2071–2100), tres escenarios de emisiones (de bajas a muy
altas) y hasta cinco modelos climáticos distintos; se recomienda usar los cinco, porque la diferencia
entre ellos es parte de la respuesta. Solo cambia el clima: relieve, vegetación y agua se dejan como
hoy, y la herramienta lo avisa. Funciona con los tres algoritmos. Para cada escenario se obtiene:

- un **mapa de cambio** con cuatro clases: favorable hoy y mañana, **nuevo favorable**, **deja de
  ser favorable** y desfavorable en ambos casos, con el mismo umbral que hoy (el del 10 %) para que
  la comparación sea justa. En el mapa, las zonas donde no está de acuerdo la mayoría de los modelos
  climáticos aparecen pálidas;
- **cuántos modelos climáticos coinciden** en cada celda, y la superficie en la que coinciden
  **todos**: si los cinco dicen que una zona se pierde, la conclusión es más sólida que si lo dice uno;
- la **dispersión entre modelos** y la **extrapolación futura** (rayada en el mapa): zonas cuyo
  clima futuro no se parece a nada de lo que el modelo vio. Si superan el 10 % de la zona, hay aviso;
- las superficies de cada clase, con el rango entre modelos climáticos, en una pestaña «Futuro» de
  los resultados y en el informe.

Coste medido: en la demo, proyectar un escenario con cinco modelos climáticos añade 0,2 s a un
estudio con MaxEnt y 2,2 s con el conjunto. Leer las capas futuras de CHELSA para un recorte real de
Provenza tardó 1,8 s. Lo que **aún no se ha hecho**: proyectar una especie real de principio a fin
(la prueba real usó una especie virtual sobre capas reales), un estudio a escala nacional, y
confirmar con CHELSA la licencia de sus capas futuras (la atribución se da igualmente). El nivel de
evidencia se refiere al modelo actual; **no cubre las proyecciones**.

**Datos de campo propios y validación independiente (fase 7, ADR-0108).** Las consultoras tienen sus
propios inventarios, a menudo en Lambert-93 y a menudo con **ausencias**: sitios visitados donde la
especie no apareció. Ahora se pueden importar en un fichero CSV. El navegador lo lee y reconoce
separadores, comas decimales, fechas y una lista cerrada de sistemas de coordenadas (WGS84,
Lambert-93, LAEA Europa y UTM). Los datos son de la organización: otra organización no puede verlos,
y ni el manifiesto ni los informes contienen sus coordenadas.

Para cada especie se elige cómo usarlos:

- **GBIF + presencias propias** para entrenar;
- **solo presencias propias**, sin consultar GBIF;
- **GBIF para entrenar y datos propios apartados para comprobar**. Es la opción que da acceso al
  nivel *Validado*.

Al comprobar, se compara el mapa con los sitios de campo: ¿puntúa más alto donde se encontró la
especie que donde no? Se calculan el **AUC presencia–ausencia** (aquí sí tiene sentido, porque hay
ausencias reales), la **sensibilidad** (qué parte de las presencias reconoce), la **especificidad**
(qué parte de las ausencias reconoce) y el **TSS**, que combina las dos. Los umbrales se fijan con
los datos de entrenamiento, nunca ajustándolos a los de comprobación. Una regla evita hacerse
trampas: si un sitio de comprobación cae en una celda que ya se usó para entrenar, se descarta,
porque el modelo ya conoce esa respuesta (pasa, por ejemplo, con observaciones del SINP que también
están publicadas en GBIF).

Probado con un inventario **sintético** de la especie de la demo, en Lambert-93: quedaron 105 sitios
de presencia y 193 de ausencia, y 2 se descartaron por la regla anterior. El resultado fue AUC 0,84 y
TSS 0,53, pero el nivel se quedó en *Indicativo*, porque el modelo no cumplía todos los criterios de
*Respaldado*: aportar datos de campo no regala el nivel. **No se ha probado aún con un inventario real
de un cliente** (no se pueden incluir en el repositorio). Tampoco se modela que una ausencia puede ser
una especie que estaba y no se vio.

**Apoyo a la decisión (fase 8, ADR-0018).** Un mapa por especie no responde a las preguntas del
cliente. Una pestaña nueva de los resultados, «Apoyo a la decisión», calcula a petición cuatro
salidas a partir de un estudio terminado, sin volver a ajustar ningún modelo ni tocar sus ficheros:

- **Prioridad de prospección**: zonas que merece la pena visitar porque son favorables y están poco
  muestreadas (según el esfuerzo del grupo objetivo o, si no lo hay, la distancia al registro
  conocido más cercano). Propone zonas candidatas ordenadas. No dice dónde está la especie.
- **Huella de proyecto**: se sube o se dibuja el contorno de un proyecto (un parque eólico, una
  carretera) y se calcula, con áreas exactas, cuánta superficie favorable toca para cada especie,
  qué parte de la huella es, y si esa zona está en extrapolación.
- **Varias especies**: cuántas especies encuentran condiciones favorables en cada celda y dónde se
  concentran (los «puntos calientes», el 10 % de celdas con más especies). Solo entran especies con
  nivel *Indicativo* o superior, y se avisa de que sumar mapas tiende a sobrestimar.
- **Comparar con…**: dos estudios de la misma especie y la misma zona (por ejemplo, con y sin
  corrección del sesgo), con un mapa de diferencias y medidas de parecido.

Todas llevan la advertencia de que **no sustituyen a un inventario de campo ni a un estudio de
impacto**, y de que una idoneidad baja no es prueba de ausencia. Cada análisis guarda su propio
registro de trazabilidad, ligado al del estudio. Probado con dos especies virtuales de la demo;
**no se ha probado con especies reales**.

### Qué viene después de Science v1

El programa está terminado, pero **ninguna opción nueva se ha validado**. Lo siguiente no es
programar más, sino:

1. **La revisión de los ecólogos de Natural Solutions** de cada ADR propuesto (y del ADR-0009, que
   tampoco se ha revisado): qué grupo de especies usar como referencia del sesgo, los umbrales de los
   niveles de evidencia, los criterios de *Validado*, cómo se calcula la prioridad de prospección…
   La nota en francés the design notes reúne esas preguntas.
2. **Probar con especies reales** que los ecólogos conozcan: todas las cifras de Science v1 vienen
   de la demo sintética, de pruebas sintéticas o de pequeños recortes de Provenza.
3. **Confirmar dos licencias**: las capas futuras de CHELSA y el modelo de elevación de Copernicus.
4. Solo entonces, convertir en valor por defecto lo que se apruebe, con un ADR nuevo.

---

## 9. Cómo trabajamos para no equivocarnos

Una herramienta así puede producir mapas bonitos y equivocados. Estas son las reglas que lo
impiden (`AGENTS.md`):

- **La corrección científica va antes que todo**, incluida la velocidad y la comodidad de uso.
- **Cada decisión científica se escribe** en un ADR, con su justificación bibliográfica y su coste
  conocido. Ningún valor por defecto cambia «sin decirlo».
- **Lo nuevo es opcional hasta que lo revisen ecólogos.** Y se comprueba que activar nada nuevo
  deja los resultados idénticos, fichero a fichero.
- **El motor se compara con el original** en cada cambio; nunca se relaja una tolerancia para que
  una prueba pase.
- **Ningún aviso se esconde** para simplificar la interfaz (calidad de datos, extrapolación, pocas
  muestras, sesgo).
- **Los datos de demostración son sintéticos y lo dicen**; nunca se presentan como reales.
- **No se afirman velocidades sin medirlas**, y cada fase termina con una lista de lo verificado y
  de lo **no** verificado.

---

## 10. Preguntas frecuentes

**¿El mapa dice dónde está la especie?** No. Dice dónde las condiciones se parecen a las de los
sitios donde se ha visto. La especie puede faltar de una zona favorable (no ha llegado, la caza, la
competencia) y aparecer ocasionalmente en una desfavorable.

**¿Por qué no da un porcentaje de fiabilidad?** Porque cualquier porcentaje sería inventado: con
datos de solo presencia no existe forma honesta de calcularlo. En su lugar, el nivel de evidencia
dice qué criterios cumple y cuáles no.

**Mi especie sale «Exploratorio». ¿Está mal el estudio?** No necesariamente. Suele significar pocos
datos, una especie generalista que el clima no explica bien, o mucha zona extrapolada. Los motivos
aparecen junto al nivel. Añadir cobertura del suelo, corregir el sesgo o aportar datos de campo
puede mejorarlo; forzar el resultado, no.

**¿Por qué no usar directamente `maxent.jar`?** Porque exige Java, no se integra bien en un servicio
web y es más lento cuando se encadenan cientos de ajustes. Lo seguimos usando como **juez**: el
motor de NicheRS tiene que dar sus mismos números.

**¿MaxEnt o Random Forest?** Por defecto, MaxEnt: es el método comprobado contra la referencia y
el que mejor se comporta en zonas nuevas. El Random Forest y el conjunto son opciones para expertos,
útiles para ver si dos métodos muy distintos llegan a la misma conclusión; si discrepan, es una
señal para ser prudente.

**¿El mapa futuro dice dónde vivirá la especie en 2070?** No. Dice dónde se darán condiciones
climáticas parecidas a las de hoy donde se la ve, según cada modelo climático. No sabe si la especie
podrá llegar hasta allí, ni cómo cambiarán los usos del suelo o las otras especies. Y donde el clima
futuro no se parece a nada conocido (zonas rayadas), el modelo está adivinando.

**¿Qué gano aportando mis propios datos de campo?** Dos cosas. Más presencias para entrenar, a
menudo de mejor calidad que las de GBIF. Y, sobre todo, si se apartan para comprobar, una prueba
independiente del mapa, que es lo único que puede llevar un resultado al nivel *Validado*. Las
ausencias (sitios visitados donde no se encontró la especie) son lo más valioso.

**¿Puedo usar ya las opciones nuevas en un informe para un cliente?** Todavía no. Están
implementadas y comprobadas técnicamente, pero no revisadas por ecólogos ni probadas con especies
reales. Los ajustes por defecto (los del MVP) sí son los que se pueden usar hoy.

**¿Puedo usar los resultados en un informe para un cliente?** Los datos por defecto tienen licencia
comercial y los registros «no comerciales» se excluyen. El informe incluye métodos, fuentes y
referencias. Para especies sensibles, hay que revisar el referencial regional antes de publicar
puntos (el informe lo recuerda).

**¿Por qué tantas opciones están «pendientes de revisión»?** Porque son métodos reconocidos en la
literatura, pero su ajuste concreto (umbrales, grupo de especies de referencia, distancias) debe
validarlo un ecólogo de Natural Solutions antes de convertirlo en valor por defecto.

---

## 11. Lo que NicheRS no hace (todavía) — límites honestos

- Solo especies **terrestres**; el clima es la media 1981–2010.
- Por defecto los modelos usan **solo clima**; relieve, vegetación y agua son opcionales y aún no
  se ha medido su efecto en especies reales.
- La corrección del sesgo por grupo objetivo solo se ha probado sin conexión (datos de prueba y
  demo sintética), no con el espejo regional real.
- El Random Forest y el conjunto solo se han probado con la demo sintética, no con especies reales.
- Las proyecciones al clima futuro no se han probado aún con una especie real de principio a fin, no incluyen dispersión ni cambios de uso del suelo, y la licencia de las capas futuras de CHELSA está pendiente de confirmar.
- **Ningún método nuevo de Science v1 se ha usado de principio a fin con una especie real.** La única excepción es la regla de coordenadas difuminadas.
- Las salidas para decidir solo se han probado con especies virtuales y sus avisos aparecen solo en inglés en la interfaz.
- La validación independiente y el nivel *Validado* solo se han probado con datos de campo sintéticos; la importación admite solo CSV (no exportaciones de GeoNature, Excel, GeoPackage ni Shapefile).
- Las correcciones de sesgo son opcionales y **están pendientes de revisión** por los ecólogos de
  Natural Solutions, igual que los cortes de los niveles de evidencia (0,40 / 0,70 para Boyce, etc.),
  que son decisiones razonadas pero discutibles.
- Las velocidades citadas son de máquinas concretas y no se extrapolan sin medir.
- El resultado es siempre **idoneidad relativa**: no dice dónde está la especie, sino dónde se
  parecen las condiciones a las de los sitios donde se ha visto.

---

## 12. Glosario

| Término | Significado |
|---|---|
| **SDM** | Modelo de distribución de especies: estima dónde hay condiciones favorables para una especie. |
| **Nicho ecológico** | Conjunto de condiciones ambientales en las que una especie puede vivir. |
| **GBIF** | Base de datos mundial de observaciones de especies. |
| **Presencia / solo presencia** | Observación de la especie en un lugar. «Solo presencia»: no sabemos dónde se buscó sin éxito. |
| **Presencia–ausencia** | Datos de campo donde se anota también si la especie *no* estaba. Permiten validación independiente. |
| **Sensibilidad / especificidad** | Qué parte de las presencias / de las ausencias de campo reconoce el mapa. |
| **TSS** | Sensibilidad + especificidad − 1. 0 es azar, 1 es perfecto. |
| **Validado** | Nivel de evidencia más alto: el mapa supera una comprobación con datos de campo independientes. |
| **Prioridad de prospección** | Zonas favorables y poco muestreadas, donde merece la pena buscar primero. |
| **Huella de proyecto** | Contorno de un proyecto (parque eólico, carretera…) que se cruza con los mapas. |
| **Punto caliente** | Zona donde muchas especies encuentran condiciones favorables a la vez. |
| **CHELSA** | Mapas climáticos mundiales a 1 km, de dominio público. |
| **Escenario de emisiones (SSP)** | Hipótesis sobre cuántos gases de efecto invernadero se emitirán; de bajas (SSP1-2.6) a muy altas (SSP5-8.5). |
| **Modelo climático (GCM)** | Simulación del clima de la Tierra; distintos modelos dan futuros algo distintos, y esa diferencia es parte de la incertidumbre. |
| **Copernicus DEM / ESA WorldCover** | Mapas europeos de relieve (90 m) y de cobertura del suelo (10 m), usables comercialmente. |
| **Cobertura fraccional** | Parte de cada celda ocupada por cada tipo de cubierta (bosque, cultivo…). |
| **Variable bioclimática (BIO1…)** | Resumen del clima útil para la biología (temperatura media, lluvia del mes más seco…). |
| **Fondo (background)** | Muestra de celdas de la zona con la que se comparan las presencias. |
| **MaxEnt** | Método de modelización de referencia para datos de solo presencia. |
| **Regularización (β)** | Penalización de la complejidad del modelo para evitar el sobreajuste. |
| **Sobreajuste** | El modelo memoriza los datos (incluido el ruido) en vez de aprender la regla general. |
| **Idoneidad relativa** | Puntuación 0–1 de lo favorables que son las condiciones. No es una probabilidad. |
| **Validación cruzada** | Esconder parte de los datos, entrenar con el resto y comprobar si acierta en lo escondido. |
| **Bloques espaciales** | Esconder zonas geográficas enteras, para que el examen no sea demasiado fácil. |
| **OR10** | Proporción de observaciones escondidas que el modelo deja fuera con el umbral del 10 %. Ideal ≈ 0,10. |
| **Índice de Boyce** | Mide si las zonas bien puntuadas tienen más observaciones de lo esperado por azar. De −1 a 1. |
| **AUC** | Métrica clásica; con solo presencia depende sobre todo del tamaño de la zona. Solo informativa. |
| **MESS** | Detecta zonas con valores de una variable fuera del rango de los datos (extrapolación). |
| **AOA** | Área de aplicabilidad: detecta también combinaciones de variables nunca vistas. |
| **Modelo nulo** | Modelo entrenado con puntos al azar, para comprobar que el modelo real es mejor que el azar. |
| **Autocorrelación espacial** | Que lo cercano se parece: dos sitios a 1 km tienen casi el mismo clima. |
| **Sesgo de muestreo** | Hay más datos donde la gente mira más, no donde hay más especie. |
| **Fondo de grupo objetivo** | Fondo tomado donde se han visto especies parecidas, para que tenga el mismo sesgo que las presencias. |
| **Aclareo (thinning)** | Quitar observaciones demasiado cercanas entre sí. |
| **Random Forest** | Método basado en muchos árboles de decisión que votan; segundo motor de NicheRS. |
| **Nivel de evidencia** | Resumen por reglas (Insuficiente / Exploratorio / Indicativo / Respaldado) de cuánto fiarse del resultado. |
| **Manifiesto de ejecución** | Registro completo de datos, versiones y ajustes para poder repetir el estudio. |
| **ADR** | *Architecture Decision Record*: documento que deja por escrito una decisión, su porqué y su coste. |

## Para saber más

- Visión general: `docs/executive-summary.md`
- Decisiones científicas por defecto: `docs/architecture/architecture-decisions/ADR-0009-ecological-defaults-and-evidence-levels.md`
- Cómo se comunica la confianza: `ADR-0007-output-semantics-and-confidence.md`
- Programa Science v1: `docs/science-v1-plan.md` (resumen al principio) y los ADR 0011, 0013, 0014, 0015, 0016, 0017, 0018 y 0108
- Nota para los ecólogos de Natural Solutions (en francés): the design notes
- Estado del arte y bibliografía: `docs/research/01-sdm-state-of-the-art.md`, `docs/research/14-references.md`
- Equivalencia con MaxEnt: `docs/benchmarks/phase3-results.md`
