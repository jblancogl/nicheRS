// Ayuda en español. Estructura impuesta por `en.ts` (TranslationOf): mismos artículos y secciones.
// Vocabulario: «idoneidad relativa», «nivel de evidencia», «consultora». Los nombres de los niveles
// (Exploratory, Indicative…) se mantienen en inglés, como en la interfaz.

import type { EnContent } from "./en";
import type { TranslationOf } from "./types";

export const es: TranslationOf<EnContent> = {
  ui: {
    help: "Ayuda",
    helpCentre: "Centro de ayuda",
    openHelp: "Abrir la ayuda",
    close: "Cerrar la ayuda",
    language: "Idioma de la ayuda",
    search: "Buscar en la ayuda",
    searchPlaceholder: "Buscar en la ayuda…",
    noResults: "Ningún artículo coincide con la búsqueda.",
    articles: "Artículos",
    onThisPage: "En este artículo",
    helpOn: "Ayuda:",
    openPage: "Abrir como página",
    backToApp: "Volver a NicheRS",
    back: "Todos los artículos",
    previous: "Anterior",
    next: "Siguiente",
    reviewNote:
      "Contenido de ayuda pendiente de revisión por los ecólogos de Natural Solutions. Describe las reglas implementadas en esta versión de NicheRS (ADR-0009); para un estudio concreto, su informe y su manifiesto son la referencia.",
    languageNote:
      "La ayuda y la interfaz están disponibles en inglés, francés y español. El idioma se elige en el menú de usuario, a la derecha de la barra superior, y se aplica a toda la interfaz, ayuda incluida (en el servicio alojado, se guarda en su cuenta).",
    hintTitle: "¿Primera vez en NicheRS?",
    hintBody:
      "«Primeros pasos» explica en unos cinco minutos el recorrido, de la especie a los resultados, y cómo leer el mapa.",
    hintOpen: "Leer Primeros pasos",
    hintDismiss: "Descartar",
    demoHelp: "¿Qué es la demostración sintética?",
    readingHelp: "Cómo leer estos resultados",
    pathHelp: "¿Qué decide el modo rápido por mí?",
    decisionHelp: "A qué responden estos resultados y a qué no",
  },
  articles: {
    "getting-started": {
      title: "Primeros pasos",
      summary:
        "Qué hace NicheRS y los siete pasos, de una especie a unos resultados revisados.",
      sections: {
        overview: {
          title: "Qué hace NicheRS",
          blocks: [
            {
              p: "NicheRS ajusta un modelo de distribución de especies MaxEnt a partir de registros de presencia de GBIF y del clima de CHELSA, lo valida con validación cruzada espacial y cartografía la **idoneidad relativa del hábitat** en su área de estudio.",
            },
            {
              p: "Indica dónde las condiciones climáticas se parecen a las de los lugares donde se ha registrado la especie. No indica dónde está la especie: véase [Qué muestra el mapa](#map).",
            },
          ],
        },
        paths: {
          title: "Rápido o Experto",
          blocks: [
            {
              p: "«Nuevo estudio» pregunta cómo prepararlo; la elección se recuerda en este navegador y se cambia en cualquier momento con el selector **Rápido / Experto** encima de los pasos, sin perder lo elegido.",
            },
            {
              ul: [
                "**Rápido** — tres pasos: una especie, el área de estudio y **Comprobar y lanzar**. La limpieza de datos, las variables climáticas CHELSA, la resolución de la malla (la más fina que permite el área) y la calibración del modelo usan los ajustes recomendados ([Limpieza de datos](#data), [Variables y colinealidad](#variables), [Validación](#validation)).",
                "**Experto** — los seis pasos descritos abajo: varias especies a la vez, filtros y resolución de los datos, elección de variables, ajustes del modelo y de la validación.",
              ],
            },
            {
              note: "Los dos caminos usan el mismo modelo validado, muestran los mismos avisos sobre los datos y registran todos los ajustes en el informe y el manifiesto del estudio. Un estudio rápido no es un análisis rebajado: es el análisis recomendado, con menos preguntas.",
              tone: "info",
            },
          ],
        },
        workflow: {
          title: "El recorrido, paso a paso",
          blocks: [
            {
              ol: [
                "**Especies** — en el espacio de trabajo (la página de inicio, con sus proyectos y estudios), «Nuevo estudio» abre este paso, dentro del proyecto seleccionado («Continuar este estudio» reanuda un estudio en preparación en este navegador). Busque en el catálogo taxonómico de GBIF y elija una o varias especies (hasta 25 por estudio en el servicio alojado). Elija una especie, no un género: un rango superior mezcla varios nichos y limita el nivel de evidencia a *Exploratory*.",
                "**Área de estudio** — busque un **lugar** — uno o varios municipios, mancomunidades (EPCI, Francia), departamentos o provincias, regiones, países o espacios protegidos (parques, reservas, Natura 2000), cuya unión forma el área; o haga clic en el mapa para ver los lugares de ese punto (Francia y España, límites oficiales del IGN, el INPN y el MITECO alojados en el servidor) — o elija un área predefinida, dibuje un polígono en el mapa o importe un archivo GeoJSON en WGS84. El modelo se ajusta y se proyecta solo dentro de esta área, de la que se extraen los puntos de fondo.",
                "**Datos** — NicheRS descarga los registros de GBIF, los depura e indica cuántos se conservan y por qué se excluyen los demás. Revise en el mapa que no haya registros inverosímiles antes de continuar. Aquí se ajustan los filtros de año, de incertidumbre de las coordenadas y de licencia, y la resolución de la malla. Véase [Depuración de datos](#data).",
                "**Variables** — elija los predictores climáticos de CHELSA (hay un conjunto recomendado preseleccionado) y, de forma opcional, predictores de relieve, cobertura del suelo y distancia al agua ([Relieve, cobertura del suelo y agua](#predictors)). Las variables muy correlacionadas se descartan automáticamente durante el cálculo. Véase [Variables y colinealidad](#variables).",
                "**Modelo** — los ajustes recomendados vienen rellenados (calibración automática, validación cruzada por bloques espaciales). Cámbielos solo por un motivo documentado; el paso Revisión enumera todo valor distinto del predeterminado.",
                "**Lanzamiento** — «Lanzar el estudio» en el paso Revisión lanza el cálculo en el servidor. Puede salir de la página y volver desde **Estudios** en la barra superior; el avance y los avisos se muestran en directo. Un estudio espera primero a que GBIF prepare el archivo completo de registros (con DOI), normalmente de 2 a 15 minutos en la cola de GBIF: la página muestra la espera, el estado en GBIF y la última comprobación, con un enlace a la descarga en gbif.org. Si GBIF va excepcionalmente lento (30 minutos), puede continuar sin la descarga: los registros vienen entonces de la API de búsqueda de GBIF, sin DOI y con un máximo por especie.",
                "**Resultados** — para cada especie: **Resumen** (avisos, nivel de evidencia, superficie de condiciones favorables, indicadores clave), **Validación**, **Variables** y **Datos y exportación**.",
              ],
            },
          ],
        },
        reading: {
          title: "Leer los resultados en el orden correcto",
          blocks: [
            {
              ol: [
                "Lea primero los **avisos** (calidad de los datos, extrapolación, rango taxonómico, descarga truncada).",
                "Lea el **nivel de evidencia** y cada uno de los motivos que aparecen debajo. Véase [Niveles de evidencia](#evidence).",
                "Mire el mapa con el **rayado de extrapolación** activado y después pase a la capa de **incertidumbre**.",
                "Solo entonces use la **superficie de condiciones favorables**, siempre con su regla de umbral y su horquilla. Véase [Horquilla de superficie favorable](#area).",
              ],
            },
          ],
        },
        demo: {
          title: "La demostración sintética",
          blocks: [
            {
              note: "La demostración usa **datos sintéticos**. Sus resultados describen una especie virtual en un entorno artificial: nunca deben aparecer en un entregable ni presentarse como una predicción para una especie real.",
              tone: "warn",
            },
            {
              p: "«Probar la demostración sintética», en el espacio de trabajo mientras no tiene ningún estudio, o en el paso Especies, ejecuta todo el recorrido con datos artificiales: una especie virtual, *Virtualis demonstrandum*, con una respuesta conocida a tres predictores sintéticos (temperatura, precipitación, rugosidad), en una región fija del sureste de Francia (3–6° E, 43,8–45,8° N). No se usan registros de GBIF ni clima de CHELSA.",
            },
            {
              p: "Los resultados de la demostración se marcan como sintéticos: una etiqueta **SINTÉTICO** junto al nombre de la especie y un aviso en la página de resultados, la mención «(SYNTHETIC)», en inglés, en los nombres de la especie y del área de estudio, y `synthetic_data: true` en `report.md` y el manifiesto. Úsela para aprender la interfaz y a leer los resultados.",
            },
            {
              p: "En el servicio alojado, un estudio de demostración es un estudio normal: cuando se ajusta su modelo, cuenta como una especie modelizada en su cuota mensual. En una instalación local, la misma demostración está disponible en la línea de comandos (`niche demo`).",
            },
          ],
        },
        keyboard: {
          title: "Atajos de teclado",
          blocks: [
            {
              table: {
                caption: "Atajos de teclado",
                head: ["Teclas", "Acción"],
                rows: [
                  ["Flechas / + −", "Desplazar / ampliar el mapa activo"],
                  ["I", "Leer los valores en el centro del mapa (resultados)"],
                  ["0", "Ampliar al área de estudio"],
                  [
                    "Intro / Esc / Retroceso",
                    "Terminar / cancelar / deshacer al dibujar",
                  ],
                  ["Esc", "Cerrar este panel de ayuda"],
                ],
              },
            },
          ],
        },
      },
    },

    map: {
      title: "Qué muestra el mapa",
      summary:
        "La idoneidad relativa del hábitat, la transformación cloglog y lo que el mapa no es.",
      sections: {
        suitability: {
          title: "Idoneidad relativa del hábitat",
          blocks: [
            {
              p: "La capa principal es la **idoneidad relativa del hábitat**: una puntuación de 0 a 1 calculada por el modelo MaxEnt para cada celda de la malla (alrededor de 1 km) del área de estudio. Cuanto más alta, más se parecen las condiciones ambientales a las de las celdas donde se registró la especie, en relación con las condiciones disponibles en el área de estudio (el fondo).",
            },
            {
              ul: [
                "Las puntuaciones son relativas a esta área de estudio, a estos registros y a estas variables. No compare puntuaciones entre especies ni entre áreas de estudio.",
                "La rampa de colores es fija de 0 a 1 y nunca se estira: un mapa pálido significa realmente puntuaciones bajas.",
              ],
            },
          ],
        },
        cloglog: {
          title: "La transformación cloglog",
          blocks: [
            {
              p: "La salida bruta de MaxEnt se reescala a 0–1 con la transformación **cloglog** (Phillips et al. 2017). Conserva el orden de las celdas; no convierte la puntuación en una probabilidad calibrada. Con datos de solo presencia y prevalencia desconocida (lo frecuente que es la especie en la zona), ninguna transformación puede hacerlo.",
            },
          ],
        },
        not: {
          title: "Lo que el mapa no es",
          blocks: [
            {
              note: "El mapa **no es una probabilidad de presencia**, **no es el área ocupada** (ni el área de distribución) y **no es una prueba de presencia ni de ausencia**.",
              tone: "warn",
            },
            {
              p: "Una puntuación alta indica que las condiciones de una celda se parecen a las de los registros conocidos. La especie puede estar ausente igualmente (barreras a la dispersión, hábitat destruido, competencia, usos del suelo o estructura del hábitat que las variables climáticas no describen). Una puntuación baja tampoco prueba la ausencia, sobre todo en zonas poco prospectadas.",
            },
          ],
        },
        layers: {
          title: "Capas del mapa",
          blocks: [
            {
              ul: [
                "**Idoneidad (relativa, 0–1)** — idoneidad relativa del hábitat del modelo final (capa principal).",
                "**Incertidumbre (DE entre pliegues)** — dispersión de la idoneidad entre los modelos de validación cruzada. Véase [Incertidumbre y extrapolación](#uncertainty).",
                "**Extrapolación (MESS < 0)** — celdas fuera de las condiciones vistas en el entrenamiento. El **rayado de extrapolación** las muestra sobre las demás capas y está activado por defecto.",
                "**Favorable o no según el umbral** — celdas cuya idoneidad alcanza el umbral de la regla elegida en la pestaña Resumen. Véase [Horquilla de superficie favorable](#area).",
                "**Esfuerzo de muestreo (registros del grupo objetivo)** — solo en estudios con fondo por grupo objetivo: registros de GBIF del grupo objetivo por celda, en naranja con escala logarítmica. Es el esfuerzo observado, no una predicción. Véase [Fondo por grupo objetivo](#data/targetgroup).",
                "**Registros de presencia** — los registros, dibujados como marcadores, conservados para la modelización o excluidos por un filtro: son datos observados, no predichos.",
              ],
            },
            {
              p: "Haga clic en una celda (o pulse **I**) para leer sus valores locales.",
            },
          ],
        },
        replaces: {
          title: "Lo que no sustituye",
          blocks: [
            {
              ul: [
                "Los inventarios de campo y los protocolos de muestreo.",
                "Un estudio de impacto ambiental y sus análisis reglamentarios.",
                "El conocimiento experto del sitio: hábitats, usos del suelo, barreras, microhábitats.",
                "Los datos de presencia de las bases naturalistas y el estatus de protección.",
              ],
            },
            {
              p: "Úselo para priorizar el esfuerzo de muestreo, plantear preguntas y documentar una hipótesis razonada sobre dónde las condiciones son favorables.",
            },
          ],
        },
      },
    },

    evidence: {
      title: "Niveles de evidencia",
      summary:
        "Insufficient, Exploratory, Indicative y Supported: criterios exactos y qué hacer.",
      sections: {
        overview: {
          title: "Reglas explícitas, sin porcentajes",
          blocks: [
            {
              p: "Cada especie recibe uno de cuatro **niveles de evidencia**, calculado con reglas fijas a partir de los datos y de los resultados de validación (ADR-0009). No hay porcentaje de confianza: el nivel y los motivos que aparecen debajo (✓ cumplido, ✗ no cumplido) son la afirmación completa.",
            },
          ],
        },
        levels: {
          title: "Niveles y criterios",
          blocks: [
            {
              table: {
                caption: "Niveles de evidencia y sus criterios",
                head: [
                  "Nivel",
                  "Criterios (deben cumplirse todos)",
                  "Significado",
                ],
                rows: [
                  [
                    "*Insufficient* (insuficiente)",
                    "Menos de 10 celdas de presencia utilizables tras la depuración",
                    "No se ajusta ningún modelo; no se cartografía nada.",
                  ],
                  [
                    "*Exploratory* (exploratorio)",
                    "Se ajustó un modelo, pero al menos un criterio de *Indicative* no se cumple",
                    "Útil para explorar patrones; no lo bastante robusto por sí solo para decidir.",
                  ],
                  [
                    "*Indicative* (indicativo)",
                    "≥ 25 celdas de presencia; índice de Boyce continuo medio ≥ 0,40; OR10 medio ≤ 0,25; fracción extrapolada del área de estudio ≤ 25 %",
                    "Respaldo razonable; usar junto con el conocimiento experto y los datos de campo.",
                  ],
                  [
                    "*Supported* (respaldado)",
                    "≥ 50 celdas de presencia; Boyce medio ≥ 0,70; OR10 medio ≤ 0,15; fracción extrapolada ≤ 10 %; validación cruzada por bloques espaciales",
                    "La suficiencia de los datos y la validación cumplen los criterios documentados.",
                  ],
                ],
              },
            },
            {
              p: "Las medias se calculan sobre los pliegues de validación cruzada del modelo seleccionado. Si el índice de Boyce no puede calcularse, el criterio correspondiente se considera no cumplido. Las «celdas de presencia» son las celdas de la malla con al menos un registro conservado (uno por celda). La fracción extrapolada es la parte del área de estudio con MESS < 0.",
            },
            {
              p: "Por encima de *Supported*, el nivel *Validated* exige datos de campo de presencia–ausencia independientes, reservados del entrenamiento, y criterios estrictos (ADR-0108, propuesto): vea [Sus datos de campo](#field-data/validated). Sin esos datos, ningún nivel está por encima de *Supported*.",
            },
          ],
        },
        independent: {
          title: "Sin validación independiente",
          blocks: [
            {
              note: "Cada resultado indica: «No independent validation data: evidence comes from cross-validation of the same, possibly biased, records» (sin datos de validación independientes: la evidencia procede de la validación cruzada de los mismos registros, posiblemente sesgados). Ni siquiera *Supported* significa que el mapa se haya contrastado con inventarios de campo.",
              tone: "warn",
            },
          ],
        },
        taxon: {
          title: "Género o familia seleccionados",
          blocks: [
            {
              p: "Si el taxón de GBIF elegido es de rango superior a especie (género, familia…), se mezclan los registros de varias especies con nichos distintos. La revisión de datos y los resultados muestran un aviso, y el nivel queda limitado a *Exploratory* sean cuales sean los indicadores. Elija la especie (o subespecie).",
            },
          ],
        },
        insufficient: {
          title: "Qué hacer con una especie Insufficient",
          blocks: [
            {
              ul: [
                "Revise los motivos de exclusión (paso Datos o pestaña Datos y exportación): puede que muchos registros se hayan eliminado por los filtros de licencia, fecha o incertidumbre. Relajar un filtro es posible pero debe justificarse; queda registrado en el informe y el manifiesto, e incluir registros CC BY-NC limita los resultados a un uso no comercial.",
                "Compruebe que el área de estudio no deja fuera una gran parte de la distribución conocida.",
                "Una malla más gruesa no ayuda: los registros se reducen a uno por celda, así que celdas más grandes dan menos celdas de presencia.",
                "Si la especie está simplemente muy poco registrada, indique que no es modelizable con los datos disponibles y apóyese en el conocimiento experto y el trabajo de campo.",
                "Una especie *Insufficient* no cuenta en la cuota mensual.",
              ],
            },
          ],
        },
        exploratory: {
          title: "Qué hacer con una especie Exploratory",
          blocks: [
            {
              ul: [
                "Lea los motivos ✗: nombran el criterio no cumplido (tamaño de muestra, índice de Boyce, omisión, extrapolación, rango taxonómico).",
                "Pocas celdas de presencia: presente el mapa como una hipótesis que comprobar en campo.",
                "OR10 alto: el modelo puede estar sobreajustado o transferirse mal entre regiones; revise los valores por pliegue en la pestaña Validación.",
                "Extrapolación alta: el área de estudio contiene condiciones ausentes de los datos de entrenamiento; mantenga visible el rayado o restrinja el área de estudio a la región de interés, por motivos ecológicos.",
                "Índice de Boyce bajo: las predicciones ordenan mal los registros reservados; busque errores en el mapa de revisión de datos y cuestione el conjunto de variables.",
                "En un informe: cite el nivel y sus motivos literalmente, no derive superficies para decidir solo a partir del mapa y combínelo con datos de campo.",
              ],
            },
            {
              note: "No cambie los ajustes solo para alcanzar un nivel superior. Todo cambio debe tener una justificación ecológica; queda registrado en el manifiesto.",
              tone: "info",
            },
          ],
        },
      },
    },

    validation: {
      title: "Indicadores de validación",
      summary:
        "Validación cruzada por bloques espaciales, índice de Boyce continuo, OR10, ORmtp y AUC presencia-fondo.",
      sections: {
        cv: {
          title: "Validación cruzada por bloques espaciales",
          blocks: [
            {
              p: "Por defecto, las presencias se dividen por su latitud mediana y después cada mitad por su longitud mediana, lo que da **4 bloques espaciales** de tamaño similar; los puntos de fondo se asignan con las mismas líneas (Muscarella et al. 2014). Cada bloque se reserva por turno y se predice con un modelo entrenado con los otros tres. Así el modelo se evalúa en una región que no ha visto, lo que reduce el optimismo causado por la autocorrelación espacial.",
            },
            {
              p: "La validación aleatoria en k pliegues está disponible en los ajustes del modelo, pero es optimista cuando los registros están agrupados en el espacio (Roberts et al. 2017). Un estudio validado con pliegues aleatorios no puede alcanzar *Supported*.",
            },
            {
              p: "Los indicadores se muestran como media ± desviación típica entre pliegues.",
            },
          ],
        },
        grid: {
          title:
            "Bloques espaciales en cuadrícula dimensionados por la autocorrelación (Experto)",
          blocks: [
            {
              p: "Una alternativa a los 4 bloques medianos: el área de estudio se cubre con una cuadrícula de **bloques cuadrados**, y bloques enteros se reparten en k pliegues (5 por defecto) para que cada pliegue tenga un número parecido de presencias (blockCV, Valavi et al. 2019). Así las presencias de prueba quedan separadas de las de entrenamiento, lo que reduce el optimismo de la validación con autocorrelación espacial (Roberts et al. 2017).",
            },
            {
              p: "**Tamaño de bloque.** Déjelo vacío para la elección automática: NicheRS ajusta un variograma a cada predictor sobre los puntos de fondo y usa el **alcance mediano** —la distancia a partir de la cual los valores dejan de estar correlacionados—, acotado para conservar bloques suficientes para la validación cruzada. También puede indicar un tamaño en km. La pestaña Validación muestra el tamaño usado, cómo se eligió, el alcance de cada predictor y el número de pliegues obtenidos.",
            },
            {
              p: "Si muy pocos bloques contienen presencias, puede haber menos pliegues de los pedidos; un aviso lo indica. Ambos métodos espaciales cuentan como validación por bloques espaciales para el nivel de evidencia.",
            },
          ],
        },
        nulls: {
          title:
            "Modelos nulos: ¿mejor que modelos con localizaciones aleatorias? (Experto)",
          blocks: [
            {
              p: "Prueba opcional (Raes & ter Steege 2007; Bohl et al. 2019; ENMeval 2.0, Kass et al. 2021). NicheRS sortea N conjuntos de **localizaciones aleatorias** en el área de estudio, tantas como presencias tiene la especie, y ajusta cada uno con exactamente el ajuste seleccionado y la misma validación cruzada. El índice de Boyce y el OR10 en los pliegues reservados del modelo real se comparan con los de los modelos con localizaciones aleatorias.",
            },
            {
              p: "**Valor p**: la fracción de modelos con localizaciones aleatorias que lo hacen al menos tan bien como el modelo real, calculada como (1 + número igual de buenos) / (N + 1). Un p ≤ 0,05 para el índice de Boyce significa que el modelo es **mejor que modelos con localizaciones aleatorias**. Con N = 20, el p más pequeño posible es 1/21 ≈ 0,048 (se necesitan al menos 19 para que p pueda llegar a 0,05; no se aceptan menos); use de 50 a 100 para una prueba más firme.",
            },
            {
              p: "Con un [fondo por grupo objetivo](#data/targetgroup), las localizaciones aleatorias se toman en proporción a los registros del grupo objetivo por celda (modelos nulos **emparejados con el sesgo**): la comparación no queda inflada por el sesgo de los observadores.",
            },
            {
              note: "Un resultado significativo indica que el modelo supera al azar, no que sea ecológicamente correcto. Si no es mejor que los modelos con localizaciones aleatorias, el nivel de evidencia se limita a *Exploratory*. No es un nivel de confianza. Coste: N × pliegues ajustes adicionales (p. ej. 100 × 4 = 400), lo que puede multiplicar la duración del estudio.",
              tone: "warn",
            },
          ],
        },
        boyce: {
          title: "Índice de Boyce continuo (CBI)",
          blocks: [
            {
              p: "De −1 a 1 (Hirzel et al. 2006). Mide si las presencias reservadas caen en celdas de idoneidad alta con más frecuencia de lo esperado según el fondo. Por encima de 0: mejor que el azar; cerca de 1: predicciones coherentes con los lugares donde se registró la especie; 0 o menos: no mejor que el azar.",
            },
            {
              p: "Se calcula con los valores por defecto de ecospat (101 ventanas móviles). No se calcula para un pliegue con muy pocas presencias de prueba (se muestra —).",
            },
          ],
        },
        or10: {
          title: "OR10 — omisión en el percentil 10",
          blocks: [
            {
              p: "Fracción de presencias reservadas que quedan por debajo del umbral que excluye el 10 % de las presencias de entrenamiento. Se espera ≈ 10 % en un modelo no sobreajustado (Radosavljevic & Anderson 2014); valores claramente superiores indican sobreajuste o mala transferencia a zonas nuevas. Es el primer criterio de selección del modelo.",
            },
          ],
        },
        ormtp: {
          title: "ORmtp — omisión en la presencia mínima de entrenamiento",
          blocks: [
            {
              p: "Fracción de presencias reservadas con una idoneidad inferior a la más baja de cualquier presencia de entrenamiento. Se espera 0 %; valores mayores significan que hay registros reservados en condiciones que el modelo valora por debajo de todos los de entrenamiento.",
            },
          ],
        },
        auc: {
          title: "AUC presencia-fondo y sus límites",
          blocks: [
            {
              p: "Se muestra solo como indicador secundario. Compara las presencias con puntos de fondo, no con ausencias conocidas: su máximo es inferior a 1 y refleja sobre todo lo extensa y ambientalmente variada que es el área de estudio (Lobo et al. 2008; Jiménez-Valverde 2012). Un área de estudio mayor la infla.",
            },
            {
              note: "Compare el AUC solo entre modelos de la misma especie en la misma área de estudio. Nunca la compare entre especies o áreas ni la llame «precisión». No se usa para seleccionar el modelo.",
              tone: "warn",
            },
          ],
        },
        selection: {
          title: "Calibración y selección del modelo",
          blocks: [
            {
              p: "Con la calibración automática, NicheRS evalúa **25 ajustes**: clases de variables L, LQ, H, LQH, LQHP (L lineal, Q cuadrática, H bisagra, P producto) × multiplicador de regularización β 0,5, 1, 2, 3, 4 (un β mayor da respuestas más suaves). Selecciona, por orden: el menor OR10 medio, después el mayor índice de Boyce medio y después el menor número de coeficientes (Radosavljevic & Anderson 2014; Kass et al. 2021). El modelo final se reajusta con todas las presencias y todos los puntos de fondo con el ajuste seleccionado.",
            },
            {
              p: "Esta regla favorece modelos más suaves y transferibles frente a una discriminación aparente. La tabla completa está en Validación › Calibración.",
            },
          ],
        },
      },
    },

    area: {
      title: "Horquilla de superficie favorable",
      summary:
        "Umbrales MTP, P10 y máx. SSS, horquilla de superficies y áreas exactas de celdas WGS84.",
      sections: {
        band: {
          title: "Superficie de condiciones favorables",
          blocks: [
            {
              p: "La **superficie de condiciones favorables** que se muestra es la suma de las áreas de las celdas cuya idoneidad alcanza o supera un umbral. Como el umbral es una elección, NicheRS muestra siempre la **horquilla** (mínimo – máximo) obtenida con tres reglas junto al valor principal, y la fracción del área de estudio. Cambiar la regla en la pestaña Resumen actualiza también la capa «Favorable o no según el umbral».",
            },
          ],
        },
        rules: {
          title: "Reglas de umbral",
          blocks: [
            {
              table: {
                caption: "Reglas de umbral",
                head: ["Regla", "Definición", "Comportamiento"],
                rows: [
                  [
                    "**P10** — percentil 10 de las presencias de entrenamiento (por defecto)",
                    "Idoneidad por debajo de la cual queda el 10 % de las presencias de entrenamiento",
                    "Tolera el ~10 % de registros erróneos o en hábitat marginal de los datos oportunistas, sin dejar de ser prudente.",
                  ],
                  [
                    "**MTP** — presencia mínima de entrenamiento",
                    "Idoneidad más baja de cualquier presencia de entrenamiento",
                    "La más inclusiva; un único registro atípico puede ampliar mucho la superficie.",
                  ],
                  [
                    "**Máx. SSS** — máxima sensibilidad + especificidad",
                    "Maximiza sensibilidad + especificidad, presencias frente a fondo (Liu et al. 2013)",
                    "Usa los puntos de fondo como pseudoausencias.",
                  ],
                ],
              },
            },
            {
              p: "Los umbrales se calculan con las predicciones del modelo final en las celdas de presencia de entrenamiento (y en los puntos de fondo para máx. SSS).",
            },
          ],
        },
        cells: {
          title: "Áreas exactas de celdas WGS84",
          blocks: [
            {
              p: "El modelo trabaja sobre la malla geográfica nativa de CHELSA (30 segundos de arco, unos 0,93 × 0,6 km a 50° N), sin reproyección ni remuestreo. El área de cada celda se calcula de forma exacta como un cuadrilátero sobre el elipsoide WGS84 (Snyder 1987) y las áreas se suman. Las celdas se encogen hacia los polos: un número de celdas no es una superficie.",
            },
            {
              p: "La superficie del área de estudio es la superficie terrestre con datos de predictores: se excluyen el mar y los grandes lagos.",
            },
          ],
        },
        wording: {
          title: "Cómo expresarlo",
          blocks: [
            {
              note: "Escriba: «superficie de condiciones favorables (regla P10): X km², horquilla Y–Z km² según las reglas de umbral». Nunca «área de distribución», «superficie de hábitat» ni «área ocupada».",
              tone: "info",
            },
          ],
        },
      },
    },

    uncertainty: {
      title: "Incertidumbre y extrapolación",
      summary:
        "Dispersión entre los modelos de los pliegues (desviación típica) y novedad ambiental (MESS).",
      sections: {
        foldsd: {
          title: "Incertidumbre: dispersión entre los modelos de los pliegues",
          blocks: [
            {
              p: "Para cada celda, la capa **Incertidumbre** muestra la desviación típica de la idoneidad entre los modelos de validación cruzada del ajuste seleccionado (4 modelos con bloques espaciales). Mide cuánto depende la predicción de qué registros se usaron para entrenar. La leyenda va de 0 a ≥ 0,25; la pestaña Resumen muestra la media en el área de estudio.",
            },
            {
              p: "No es un intervalo de confianza y no incluye todas las fuentes de error (errores en los registros, variables ausentes, sesgo de muestreo).",
            },
          ],
        },
        mess: {
          title: "Extrapolación (MESS)",
          blocks: [
            {
              p: "La capa **Extrapolación** es la superficie de similitud ambiental multivariante (MESS, Elith et al. 2010), calculada respecto a los ambientes de las celdas de fondo y de presencia. MESS < 0 significa que al menos una variable está fuera del rango visto en el entrenamiento: la predicción allí es una extrapolación, guiada por los supuestos del modelo y no por los datos.",
            },
            {
              p: "Estas celdas aparecen **rayadas en el mapa por defecto**; mantenga el rayado en los mapas publicados. La fracción del área de estudio con MESS < 0 entra en el nivel de evidencia (≤ 25 % para *Indicative*, ≤ 10 % para *Supported*), y por encima del 10 % se muestra un aviso.",
            },
          ],
        },
        aoa: {
          title: "Área de aplicabilidad (AOA, Experto)",
          blocks: [
            {
              p: "Capa opcional (Meyer & Pebesma 2021, paquete de R CAST). Para cada celda, el **índice de disimilitud (DI)** es la distancia, en el espacio de los predictores, al punto de entrenamiento más cercano (presencia o fondo), dividida por la distancia media entre puntos de entrenamiento. Los predictores se estandarizan y se ponderan por su importancia por permutación en el modelo final, de modo que las diferencias en variables importantes pesan más.",
            },
            {
              p: "El **umbral** se obtiene de los datos de entrenamiento con los mismos pliegues de validación cruzada (el mayor DI de los puntos de entrenamiento que no es atípico). Las celdas por encima quedan **fuera del área de aplicabilidad** y se rayan en violeta cuando la capa está activada. Se muestra un aviso si más del 10 % del área de estudio queda fuera.",
            },
            {
              p: "**Diferencia con MESS.** MESS señala valores fuera del rango de cada variable tomada por separado; el AOA señala además *combinaciones* nuevas de valores dentro de esos rangos y pondera las variables por su importancia. Pueden discrepar: lea ambos.",
            },
            {
              note: "El AOA no cambia el nivel de evidencia: MESS sigue siendo el criterio de extrapolación (ADR-0009). Estar dentro del área de aplicabilidad no garantiza que la predicción sea correcta.",
              tone: "info",
            },
          ],
        },
        together: {
          title: "Leerlas juntas",
          blocks: [
            {
              ul: [
                "Idoneidad alta, poca dispersión, sin rayado: la parte más robusta del mapa.",
                "Dispersión alta: la predicción cambia con el subconjunto de entrenamiento; precaución.",
                "Rayado: fuera de las condiciones de entrenamiento; no interprete allí la puntuación, sea cual sea su valor.",
                "Ninguna de las dos capas muestra el sesgo de muestreo: con datos oportunistas, las zonas bien muestreadas parecen más favorables (sin corrección de sesgo por defecto; véase [Sesgo de muestreo](#data/bias)).",
              ],
            },
          ],
        },
      },
    },

    data: {
      title: "Depuración de datos y motivos de exclusión",
      summary:
        "Registros de GBIF, reglas de depuración por orden, licencias y cómo citar la descarga de GBIF.",
      sections: {
        source: {
          title: "De dónde vienen los registros",
          blocks: [
            {
              p: "Los registros proceden de GBIF (gbif.org), a través del catálogo taxonómico de GBIF. Cuando el servidor dispone de una cuenta de GBIF (el servicio alojado usa la de Natural Solutions), proceden de una **descarga de GBIF con DOI**.",
            },
            {
              p: "Si no, se usa la API de búsqueda de GBIF, limitada por defecto a 20 000 registros por especie; por encima de ese límite se toma una submuestra estratificada espacialmente y la revisión de datos lo indica. Para un informe final, use un estudio con descarga con DOI.",
            },
            {
              p: "Fondo: 10 000 celdas terrestres al azar dentro del área de estudio, extraídas con la semilla del estudio. No se aplica corrección del sesgo de muestreo por defecto; el modo Experto ofrece un [fondo por grupo objetivo](#data/targetgroup).",
            },
          ],
        },
        rules: {
          title: "Reglas de depuración, por orden",
          blocks: [
            {
              table: {
                caption: "Reglas de depuración de registros",
                head: [
                  "#",
                  "Motivo de exclusión mostrado",
                  "Por defecto",
                  "Por qué",
                ],
                rows: [
                  [
                    "1",
                    "Ejemplar fósil o cautivo/cultivado",
                    "Excluido",
                    "No son poblaciones silvestres en su nicho ambiental.",
                  ],
                  [
                    "2",
                    "Licencia no permitida en este proyecto",
                    "Solo CC0 y CC BY 4.0",
                    "CC BY-NC es no comercial (véase abajo).",
                  ],
                  [
                    "3",
                    "Registro sin fecha",
                    "Excluido",
                    "No puede asociarse al periodo climático de referencia.",
                  ],
                  [
                    "4",
                    "Registrado antes de 1981 (periodo de referencia climático)",
                    "Año ≥ 1981",
                    "La climatología de CHELSA cubre 1981–2010.",
                  ],
                  [
                    "5",
                    "Coordenadas generalizadas (difuminadas) por el publicador; precisión no indicada o > 1000 m",
                    "Excluido; se conservan los registros generalizados con precisión indicada ≤ 1000 m",
                    "Posición difuminada de precisión desconocida. Véase [Coordenadas generalizadas](#data/generalised).",
                  ],
                  [
                    "6",
                    "Incertidumbre de las coordenadas > 1000 m",
                    "≤ 1000 m; se conservan los registros sin este dato",
                    "Mayor que la celda de unos 1 km.",
                  ],
                  ["7", "Fuera del área de estudio", "Excluido", "—"],
                  [
                    "8",
                    "En el mar o en un gran lago (máscara de tierra)",
                    "Excluido",
                    "Nicho terrestre; los datos climáticos también existen sobre el mar.",
                  ],
                  [
                    "9",
                    "Faltan datos ambientales",
                    "Excluido",
                    "No hay valor de predictor en la celda.",
                  ],
                  [
                    "10",
                    "Duplicado en una misma celda de la cuadrícula",
                    "Un registro por celda",
                    "Reduce la pseudorreplicación.",
                  ],
                  [
                    "11",
                    "Filtrado espacial (a menos de X km de un registro conservado)",
                    "Desactivado (opción Experto)",
                    "Reduce las agrupaciones. Véase [Filtrado espacial por distancia](#data/thinning).",
                  ],
                ],
              },
            },
            {
              p: "Cada registro excluido se cuenta en la **primera** regla que no cumple, de modo que los recuentos se suman. Las etapas de depuración no pueden desactivarse; los filtros de año, incertidumbre y licencia se ajustan en el paso Datos y quedan registrados en el informe y el manifiesto.",
            },
            {
              p: "La máscara terrestre (Natural Earth 1:10m) tiene una precisión de la línea de costa de unos 0,5–1 km: algunos registros costeros pueden excluirse por error. Se listan con su motivo; revíselos en especies costeras. Las especies marinas quedan fuera del alcance.",
            },
          ],
        },
        generalised: {
          title: "Coordenadas generalizadas (difuminadas)",
          blocks: [
            {
              p: "**Los publicadores difuminan las coordenadas de las especies sensibles. NicheRS no usa puntos difuminados cuya precisión se desconoce.**",
            },
            {
              p: "Algunos publicadores de datos *generalizan* las coordenadas a propósito — redondeadas a una cuadrícula de 10 km o llevadas al centroide del municipio, por ejemplo — para proteger especies sensibles (rapaces nidificantes, orquídeas, refugios de murciélagos) o a petición del observador. GBIF lo indica en los campos Darwin Core `dataGeneralizations` e `informationWithheld`.",
            },
            {
              p: "Un registro generalizado solo se conserva si la incertidumbre de sus coordenadas está indicada y dentro del límite de incertidumbre (1000 m por defecto). Si la precisión no se indica, o es mayor, se excluye con el motivo **Coordenadas generalizadas por el publicador** (regla 5, justo antes de la regla de incertidumbre) y se dibuja en el mapa como un círculo magenta discontinuo. La pestaña Datos y exportación da los recuentos: generalizados, excluidos, conservados.",
            },
            {
              p: "En Francia, el SINP (Système d'information de l'inventaire du patrimoine naturel) define, región por región, qué especies tienen una localización sensible (référentiels régionaux de sensibilité); sus registros suelen publicarse con una resolución gruesa. Las presencias conservadas pueden entonces infrarrepresentar los sitios más sensibles. El aviso `generalised_records` da las cifras; pasa a ser un Aviso cuando al menos el 10 % de los registros del área están generalizados.",
            },
            {
              note: "La API de búsqueda de GBIF indica la generalización; las descargas de GBIF (formato SIMPLE_CSV) y la réplica regional de GBIF no. Los estudios que las usan muestran la nota *indicadores de generalización no disponibles*: no se pueden detectar los registros generalizados con incertidumbre pequeña o ausente; compruebe usted mismo las especies sensibles en el référentiel regional.",
              tone: "warn",
            },
            {
              p: "`occurrences.geojson` conserva las coordenadas tal como las publica GBIF. Algunas especies son sensibles según las normas del SINP: consulte el référentiel regional antes de reproducir puntos en un informe público.",
            },
          ],
        },
        thinning: {
          title: "Filtrado espacial por distancia (Experto)",
          blocks: [
            {
              p: "Opcional, desactivado por defecto, se ajusta en el paso Modelo (modo experto). Tras la depuración, las celdas de presencia conservadas se filtran para que ningún par de celdas restantes esté más cerca que la distancia elegida (más de 0 y como máximo 100 km). Reduce las agrupaciones de registros de lugares muy muestreados (ciudades, reservas, sitios conocidos), que de otro modo pesarían mucho en el modelo, y la autocorrelación espacial entre presencias.",
            },
            {
              p: "El método sigue a spThin (Aiello-Lammens et al. 2015): las celdas se eliminan en orden aleatorio, el sorteo se repite varias veces y se conserva el conjunto de celdas más grande. Los sorteos proceden de la semilla del estudio: la vista previa de los datos, la ejecución y cualquier repetición del mismo estudio dan el mismo resultado. Las celdas eliminadas se cuentan como **Filtrado espacial (a menos de X km de un registro conservado)**, después de la regla del duplicado por celda, y se dibujan en el mapa como pequeños puntos grises.",
            },
            {
              note: "El filtrado cuesta datos: cada celda eliminada es una presencia real. Con pocas presencias puede llevar una especie hacia *Exploratory* o por debajo del mínimo. Elija una distancia acorde con las agrupaciones que quiere eliminar (por ejemplo unos pocos km alrededor de puntos calientes de observación), no mayor de lo necesario.",
              tone: "warn",
            },
            {
              p: "La distancia queda registrada en la especificación (`filters.thin_km`), el informe y el manifiesto.",
            },
          ],
        },
        bias: {
          title: "Sesgo de muestreo: los registros muestran dónde se mira",
          blocks: [
            {
              p: "Los registros de GBIF son sobre todo oportunistas: proceden de los lugares adonde va la gente — cerca de ciudades, carreteras, senderos, reservas y sitios conocidos — tanto como de los lugares donde vive la especie. Un modelo ajustado con ellos aprende el hábitat **y** los hábitos de los observadores: las condiciones bien muestreadas parecen más favorables de lo que son, y las zonas remotas, menos.",
            },
            {
              p: "Por defecto, NicheRS no corrige este sesgo y lo indica con el aviso `sampling_bias`. Lea el mapa teniéndolo en cuenta: una puntuación baja en una zona poco muestreada puede significar solo que nadie buscó allí.",
            },
            {
              ul: [
                "**Fondo por grupo objetivo** (Experto): toma los puntos de fondo donde se registraron organismos similares, para que presencias y fondo compartan el sesgo. Véase [Fondo por grupo objetivo](#data/targetgroup).",
                "**Filtrado espacial por distancia** (Experto): elimina presencias agrupadas. Véase [Filtrado espacial por distancia](#data/thinning).",
                "La validación cruzada por bloques espaciales no elimina el sesgo: los pliegues de prueba proceden de los mismos registros sesgados.",
              ],
            },
          ],
        },
        targetgroup: {
          title: "Fondo por grupo objetivo (Experto)",
          blocks: [
            {
              p: "En lugar de celdas aleatorias, los puntos de fondo se toman entre las celdas del área de estudio con al menos un registro de GBIF de un **grupo objetivo** — organismos muestreados del mismo modo que la especie, por defecto su propia clase de GBIF (Aves para un ave, Insecta para un escarabajo, Magnoliopsida para una planta con flores). Presencias y fondo comparten entonces el sesgo de los observadores, que se compensa en gran parte en el modelo (Phillips et al. 2009). Las celdas se toman de forma uniforme, una por celda, con la semilla del estudio.",
            },
            {
              p: "Necesita las **cuadrículas de esfuerzo de muestreo de la réplica regional de GBIF** (registros de cada clase por celda). Si ninguna réplica cubre el área, si la réplica aún no tiene cuadrículas, si la clase no tiene registros o si muy pocas celdas los tienen, el fondo vuelve a ser aleatorio y el aviso `target_group_unavailable` da el motivo: no se aplicó ninguna corrección del sesgo. Si menos celdas de las pedidas tienen registros, el fondo es más pequeño (nota `target_group_background_smaller`).",
            },
            {
              p: "**Mapa de esfuerzo.** Estos estudios tienen una capa *Esfuerzo de muestreo*: registros del grupo objetivo por celda, en naranja con escala logarítmica (1 registro → el máximo en una celda; las celdas sin registros son transparentes). Los recuentos son registros, no visitas ni observadores. En la demostración, la superficie de esfuerzo es sintética y se indica así.",
            },
            {
              p: "**Diagnóstico.** La pestaña Datos y exportación indica la proporción de celdas de presencia situadas en las celdas más muestreadas que cubren ≈ 10 % del área de estudio. Sin sesgo se esperaría en torno al 10 %; una proporción mucho mayor significa que las presencias siguen el esfuerzo de observación. Es solo descriptivo y no cambia el nivel de evidencia.",
            },
            {
              p: "**Modelos nulos emparejados con el sesgo.** Si se piden modelos nulos, sus localizaciones aleatorias se toman en proporción a los registros del grupo objetivo por celda en lugar de uniformemente: el modelo se compara con modelos de localizaciones aleatorias que tienen el mismo sesgo.",
            },
            {
              note: "Solo corrige la parte del sesgo que el grupo objetivo comparte con la especie. Las especies muestreadas de otro modo que su clase (un ave rara buscada por especialistas, una especie registrada sobre todo por un programa) conservan su propio sesgo, y un grupo objetivo con muy pocos registros da un fondo deficiente.",
              tone: "warn",
            },
          ],
        },
        licences: {
          title: "Licencias: por qué se excluye CC BY-NC",
          blocks: [
            {
              p: "Por defecto solo se usan registros **CC0** y **CC BY 4.0**. Los registros **CC BY-NC** se excluyen porque no pueden usarse en entregables comerciales de una consultora sin una lectura jurídica de la licencia (ADR-0006).",
            },
            {
              note: "Puede incluir registros CC BY-NC, pero entonces los resultados quedan restringidos a un uso no comercial: no los incluya en un entregable facturado.",
              tone: "warn",
            },
            {
              p: "Los registros CC BY exigen atribución: cite la descarga de GBIF (abajo).",
            },
          ],
        },
        citation: {
          title: "Citar la descarga de GBIF (DOI)",
          blocks: [
            {
              p: "El DOI aparece en la pestaña Datos y exportación (Fuente), en `report.md` y en el manifiesto (`occurrence_source.citation`). Cítelo en la forma recomendada por GBIF: *GBIF.org (fecha) GBIF Occurrence Download https://doi.org/10.15468/dl.xxxxxx*.",
            },
            {
              p: "Una descarga con DOI es el registro permanente y citable de los registros exactos utilizados. Un estudio con la API de búsqueda no tiene DOI.",
            },
          ],
        },
        review: {
          title: "Qué revisar en el paso Datos",
          blocks: [
            {
              ul: [
                "Registros en lugares inverosímiles: jardines, museos, coordenadas invertidas, centroides de país o región.",
                "Una gran proporción excluida por una sola regla merece una revisión.",
                "Agrupaciones alrededor de ciudades, reservas o carreteras: sesgo de muestreo, no corregido por defecto (véase [Sesgo de muestreo](#data/bias)).",
                "Los registros generalizados o filtrados (círculos magenta discontinuos, puntos grises) y sus recuentos en la tabla de exclusiones.",
                "Los avisos: taxón de rango superior a especie, descarga truncada o submuestreada.",
                "La resolución de la malla: ×1 equivale a unos 1 km; una malla más gruesa da menos celdas de presencia por la regla de un registro por celda.",
              ],
            },
          ],
        },
      },
    },

    variables: {
      title: "Variables y colinealidad",
      summary:
        "Clima CHELSA v2.1 1981–2010, el conjunto por defecto y el filtro |r| > 0,7.",
      sections: {
        chelsa: {
          title: "Clima CHELSA v2.1",
          blocks: [
            {
              p: "Los predictores son las 19 variables bioclimáticas de **CHELSA v2.1**, climatología 1981–2010, a 30 segundos de arco (unos 1 km), licencia CC0 (Karger et al. 2017).",
            },
            {
              note: "Describen **solo el clima**. Se pueden añadir relieve, cobertura del suelo y distancia al agua, que no están seleccionados por defecto ([Relieve, cobertura del suelo y agua](#predictors)); no se ofrecen suelos. Celdas con un clima favorable pueden no tener ningún hábitat favorable. Las proyecciones con clima futuro son una opción Experta ([Proyecciones con clima futuro](#future)).",
              tone: "warn",
            },
            {
              p: "No se usa WorldClim porque su licencia no permite el uso comercial.",
            },
          ],
        },
        defaults: {
          title: "Conjunto de variables por defecto",
          blocks: [
            {
              p: "Preseleccionadas: BIO1 (temperatura media anual), BIO4 (estacionalidad de la temperatura), BIO5 (temperatura máxima del mes más cálido), BIO6 (temperatura mínima del mes más frío), BIO12 (precipitación anual), BIO14 (precipitación del mes más seco), BIO15 (estacionalidad de la precipitación): energía, agua, su estacionalidad y sus extremos.",
            },
            {
              p: "BIO8, BIO9, BIO18 y BIO19 están señaladas y no se seleccionan por defecto: combinan temperatura y precipitación de trimestres concretos y presentan artefactos espaciales (Booth 2022).",
            },
          ],
        },
        collinearity: {
          title: "Filtro de colinealidad",
          blocks: [
            {
              p: "Los predictores correlacionados hacen inestables las formas de respuesta y las contribuciones. Antes del ajuste, NicheRS calcula las correlaciones de Pearson sobre la muestra de fondo y conserva las variables **en su orden de prioridad**, descartando cualquier variable con |r| > 0,7 respecto a una variable ya conservada (Dormann et al. 2013). Las variables descartadas y aquella con la que están correlacionadas se listan en los resultados y en el informe. El umbral se ajusta en los ajustes del modelo.",
            },
          ],
        },
        contributions: {
          title: "Contribución de las variables",
          blocks: [
            {
              p: "La contribución porcentual es una estimación heurística de MaxEnt de cuánto aportó cada variable al ajuste. Depende del camino de optimización y de las variables correlacionadas; no es una medida de importancia causal.",
            },
            {
              p: "La importancia por permutación (segunda barra) es la otra estimación de MaxEnt: los valores de cada variable se permutan al azar entre presencias y puntos de fondo, y la caída del AUC de entrenamiento se normaliza a 100. Solo depende del modelo final, no del camino de optimización. Cuando las dos estimaciones difieren, el papel de la variable es incierto; ambas reparten el crédito entre variables correlacionadas.",
            },
            {
              p: "Las curvas de respuesta muestran la idoneidad relativa (cloglog) cuando varía una variable, con todas las demás fijadas en su media sobre las presencias. Las zonas sombreadas quedan fuera de los datos de entrenamiento: allí el modelo está acotado (clamping) y la curva es una extrapolación. Son curvas marginales; con variables correlacionadas pueden ocultar efectos de variables que cambian juntas. Las curvas completas (501 puntos, como `maxent.jar`) están en `response_curves.csv`.",
            },
          ],
        },
      },
    },

    predictors: {
      title: "Relieve, cobertura del suelo y agua",
      summary:
        "Predictores opcionales del Copernicus DEM y de ESA WorldCover 2021: qué miden, cuándo usarlos, sus límites y sus fuentes.",
      sections: {
        overview: {
          title: "Qué aportan",
          blocks: [
            {
              p: "El clima explica la distribución general de una especie; en una cuadrícula de 1 km muchas especies responden también al relieve y a la composición del paisaje. Además de las 19 variables climáticas de CHELSA, NicheRS ofrece 12 predictores opcionales: tres variables de relieve, ocho proporciones de cobertura del suelo y la distancia al agua permanente. Están en la misma cuadrícula de 30″ que CHELSA.",
            },
            {
              note: "Ninguno está seleccionado por defecto: la selección recomendada sigue siendo solo climática (ADR-0009). Estos predictores se proponen en el ADR-0015, pendiente de revisión por los ecólogos de Natural Solutions.",
              tone: "info",
            },
            {
              p: "Siguen el mismo tratamiento que el clima: el filtro de colinealidad conserva las variables **en el orden en que las seleccionó** (seleccione primero el clima, para que una variable de cobertura del suelo solo se conserve si aporta información), y después MESS, curvas de respuesta y contribuciones.",
            },
            {
              p: "Elija pocos predictores, útiles para la ecología de la especie. Cada variable añadida solo puede aumentar la parte del área fuera de las condiciones de entrenamiento (MESS toma el mínimo sobre las variables), lo que puede bajar el nivel de evidencia. Es honesto, no un fallo.",
            },
          ],
        },
        terrain: {
          title: "Relieve (Copernicus DEM)",
          blocks: [
            {
              table: {
                caption: "Predictores de relieve",
                head: ["Variable", "Unidad", "Valor por celda de 1 km"],
                rows: [
                  [
                    "Altitud (ELEV)",
                    "m",
                    "Media de las altitudes a 90 m en la celda.",
                  ],
                  [
                    "Pendiente (SLOPE)",
                    "grados",
                    "Media de las pendientes calculadas a 90 m (Horn 1981) **antes** de agregar, para no aplanar el relieve.",
                  ],
                  [
                    "Rugosidad (ROUGH)",
                    "m",
                    "Desviación típica de las altitudes a 90 m en la celda.",
                  ],
                ],
              },
            },
            {
              ul: [
                "**Cuándo usarlos**: pendiente y rugosidad para especies ligadas a terrenos empinados o rocosos, o que los evitan (aves rupícolas, reptiles de roquedos, ungulados de montaña).",
                "La altitud repite en gran parte la temperatura (BIO1): el filtro de colinealidad la descarta a menudo y rara vez aporta información.",
              ],
            },
            {
              note: "El Copernicus DEM es un modelo **de superficie** (TanDEM-X, 2011–2015): las altitudes incluyen el dosel forestal y los edificios, de modo que las celdas arboladas y urbanas parecen más altas y rugosas que el suelo.",
              tone: "warn",
            },
          ],
        },
        landcover: {
          title: "Cobertura del suelo (ESA WorldCover 2021)",
          blocks: [
            {
              p: "Cada variable de cobertura del suelo es la **proporción** (de 0 a 1) de los píxeles de 10 m de la celda que pertenecen a un grupo de clases de WorldCover; una celda de 1 km contiene varios miles de píxeles, así que se conservan los mosaicos y los bordes en lugar de una sola clase dominante.",
            },
            {
              table: {
                caption: "Grupos de cobertura del suelo",
                head: ["Variable", "Clases de WorldCover"],
                rows: [
                  ["Cubierta arbórea (TREE)", "10 cubierta arbórea"],
                  ["Matorral (SHRUB)", "20 matorral"],
                  ["Pastizal (GRASS)", "30 pastizal"],
                  ["Cultivos (CROP)", "40 cultivos"],
                  ["Edificado (BUILT)", "50 edificado"],
                  [
                    "Suelo desnudo y vegetación escasa (BARE)",
                    "60 suelo desnudo/vegetación escasa, 70 nieve y hielo, 100 musgos y líquenes",
                  ],
                  ["Agua permanente (WATER)", "80 masas de agua permanentes"],
                  ["Humedales (WETL)", "90 humedales herbáceos, 95 manglares"],
                ],
              },
            },
            {
              note: "Las ocho proporciones suman 1 en cada celda. Elija los grupos pertinentes para la especie (por ejemplo la cubierta arbórea para una especie forestal, pastizal y cultivos para un ave esteparia), no los ocho: con los ocho uno es redundante y el filtro de correlación por pares no lo detecta. Los resultados llevan entonces un aviso.",
              tone: "warn",
            },
            {
              ul: [
                "**Desfase temporal**: la cobertura del suelo es la de 2021, la climatología 1981–2010 y los registros pueden ser de 1981. Donde el uso del suelo cambió, los registros antiguos quedan descritos por la cobertura actual.",
                "**Edificado y sesgo de muestreo**: lo edificado está muy correlacionado con dónde se observa. Una respuesta positiva a BUILT puede reflejar a los observadores, no a la especie; los resultados lo avisan cuando se usa.",
                "WorldCover no distingue frondosas de coníferas, ni cultivos anuales de permanentes (viñedos, frutales); su exactitud global es de un 77 %.",
                "Una clase al 0 % en todas las celdas de entrenamiento aparece en MESS como fuera de rango aunque no sea ecológicamente notable; lea el mapa MESS teniéndolo en cuenta.",
              ],
            },
          ],
        },
        water: {
          title: "Distancia al agua",
          blocks: [
            {
              p: "**Distancia al agua (DWAT, km)**: distancia del centro de la celda a la celda más cercana cubierta al menos en su mitad por agua permanente en WorldCover 2021, **limitada a 25 km**. Los valores hasta 25 km son exactos y no dependen de la extensión del área de estudio; las celdas más lejanas valen 25 km.",
            },
            {
              ul: [
                "**Cuándo usarla**: especies ligadas a lagos, embalses, grandes ríos o la costa (aves acuáticas, algunos anfibios y odonatos).",
                "Solo se ve el agua que cubre al menos la mitad de una celda de 1 km: lagos, embalses, ríos anchos, lagunas y el mar. Los arroyos y charcas no se resuelven: no sirve para especies que dependen de pequeños cursos de agua.",
                "Mismo desfase temporal que la cobertura del suelo (2021).",
              ],
            },
          ],
        },
        aggregation: {
          title: "Cómo se calculan los valores",
          blocks: [
            {
              ul: [
                "No se remuestrea nada: cada píxel de origen se asigna a la celda de CHELSA que contiene su centro, así que todas las capas coinciden exactamente.",
                "Una celda queda sin dato cuando menos de la mitad de sus píxeles de origen son válidos. En cuadrículas más gruesas (×2, ×3…) los valores a 1 km se promedian, como el clima.",
                "Una celda solo se modela donde **todos** los predictores seleccionados tienen valor. En la costa el DEM puede faltar donde no existen teselas de mar; los registros allí se excluyen como «Faltan datos ambientales».",
                "Un primer estudio en una región nueva descarga y agrega las teselas de origen (medido: unos 4–5 s y 8 MB por grado para WorldCover, alrededor de 1 s por grado para el DEM); los estudios siguientes leen la caché de la instancia en milisegundos.",
              ],
            },
          ],
        },
        sources: {
          title: "Fuentes, licencias y atribución",
          blocks: [
            {
              table: {
                caption: "Fuentes de los predictores adicionales",
                head: [
                  "Fuente",
                  "Licencia",
                  "Atribución (reproducida en los informes)",
                ],
                rows: [
                  [
                    "Copernicus DEM GLO-90 (ESA, doi:10.5270/ESA-c5d3d65)",
                    "Licencia Copernicus WorldDEM-90: gratuita, mundial; se permiten reproducción, distribución y adaptación",
                    "produced using Copernicus WorldDEM-90 © DLR e.V. 2010-2014 and © Airbus Defence and Space GmbH 2014-2018 provided under COPERNICUS by the European Union and ESA; all rights reserved",
                  ],
                  [
                    "ESA WorldCover 10 m 2021 v200 (Zanaga et al. 2022, doi:10.5281/zenodo.7254221)",
                    "CC BY 4.0",
                    "© ESA WorldCover project 2021 / Contains modified Copernicus Sentinel data (2021) processed by ESA WorldCover consortium",
                  ],
                ],
              },
            },
            {
              p: "Ambas permiten el uso comercial con atribución (comprobado el 2026-10-20; la licencia del DEM concede los derechos sin nombrar el uso comercial, y conviene una confirmación jurídica). Los informes y el manifiesto enumeran cada fuente usada con su licencia y su atribución; conserve la atribución al reutilizar un mapa.",
            },
          ],
        },
      },
    },

    algorithms: {
      title: "Algoritmos de modelización",
      summary:
        "MaxEnt (por defecto), el bosque aleatorio submuestreado y el ensamble MaxEnt + bosque aleatorio: qué son, cuándo usarlos y cómo leer sus resultados.",
      sections: {
        overview: {
          title: "MaxEnt sigue siendo la opción por defecto",
          blocks: [
            {
              p: "Todo estudio usa **MaxEnt** salvo que elija otra cosa en modo experto (paso del modelo, *Algoritmo*). MaxEnt es la opción por defecto porque su adaptación en NicheRS está probada frente a `maxent.jar` 3.4.4 en casos de referencia, su comportamiento con datos de presencia–fondo está bien documentado y sus curvas de respuesta suaves son fáciles de discutir con un cliente (ADR-0002, ADR-0009).",
            },
            {
              p: "Hay dos alternativas opcionales: un **bosque aleatorio submuestreado** y un **ensamble** de MaxEnt y ese bosque. Ambas se proponen en el ADR-0017, pendiente de revisión por los ecólogos de Natural Solutions.",
            },
            {
              note: "Todos los algoritmos producen **idoneidad relativa del hábitat**, nunca una probabilidad de presencia: los datos de presencia–fondo no permiten estimar lo común que es la especie.",
              tone: "warn",
            },
          ],
        },
        rf: {
          title: "Bosque aleatorio submuestreado",
          blocks: [
            {
              p: "Un bosque aleatorio (Breiman 2001) es un conjunto de árboles de clasificación, cada uno construido con una muestra aleatoria de los datos, que votan «presencia» o «fondo». Con datos de presencia–fondo un bosque por defecto falla: hay muchos más puntos de fondo que presencias, así que la mayoría de los árboles votan «fondo» casi en todas partes.",
            },
            {
              p: "El bosque **submuestreado** lo corrige (Valavi et al. 2021): cada árbol se construye con un remuestreo con reemplazo de las presencias y **el mismo número** de puntos de fondo. Valavi et al. 2022 lo situaron en el grupo de cabeza de los métodos para datos de solo presencia, en 225 especies con datos de prueba independientes, mientras que el bosque por defecto estaba entre los peores.",
            },
            {
              ul: [
                "Ajustes: 500 árboles por defecto (100–2 000), desarrollados por completo, `mtry` = √p variables probadas en cada división (opcionalmente también p/2, elegido por la misma regla de validación que MaxEnt).",
                "Salida: la **proporción de árboles que votan «presencia»**, de 0 a 1. Es un índice de idoneidad relativa: el equilibrio 1:1 de cada árbol fija su prevalencia aparente.",
                "Los umbrales usan las puntuaciones **fuera de la bolsa** (out-of-bag) de las presencias de entrenamiento (cada presencia puntuada solo por los árboles que no la vieron), porque los árboles desarrollados por completo puntúan sus propias presencias de entrenamiento casi en 1.",
                "Mismos pliegues, métricas, regla de selección, modelos nulos, MESS, área de aplicabilidad y reglas de nivel de evidencia que MaxEnt: ambos motores se juzgan igual.",
              ],
            },
          ],
        },
        ensemble: {
          title: "Ensamble de MaxEnt y el bosque aleatorio",
          blocks: [
            {
              p: "El ensamble calibra cada miembro como se calibraría por separado y después los promedia. Como una puntuación cloglog y una proporción de votos no están en la misma escala, el valor de cada miembro se convierte antes en su **rango percentil** entre sus propios valores en los puntos de fondo; el ensamble es la media de los dos rangos.",
            },
            {
              p: "Un valor de ensamble de 0,8 significa, por tanto, que en promedio los dos modelos sitúan la celda por encima del 80 % de los puntos de fondo del área de estudio. Son rangos propios de esta área: no los compare entre estudios.",
            },
            {
              p: "El ensamble se evalúa como un modelo propio: en cada pliegue se combinan los modelos de pliegue de los dos miembros y se prueban con los datos reservados. Sus métricas determinan el nivel de evidencia; la pestaña Validación muestra también las métricas de cada miembro en los mismos pliegues. Los umbrales se calculan sobre los valores del ensamble con las mismas reglas (P10, máxima sensibilidad + especificidad, presencia mínima de entrenamiento).",
            },
          ],
        },
        choose: {
          title: "Cuál elegir",
          blocks: [
            {
              ul: [
                "**MaxEnt** para un estudio habitual: documentado, reproducible frente al software de referencia, curvas suaves.",
                "**El ensamble** para comprobar que las conclusiones no dependen de un solo algoritmo. Valavi et al. 2022 encontraron que el ensamble de modelos calibrados era el mejor método en promedio; cuando los dos miembros discrepan mucho (vea sus métricas en la pestaña Validación), indíquelo en el informe.",
                "**El bosque aleatorio solo**, sobre todo para comparar. Los árboles no extrapolan: fuera de las condiciones de entrenamiento el bosque predice valores constantes, y con bloques espaciales puede puntuar peor que MaxEnt (en la demostración sintética, Boyce 0,22 frente a 0,57 con 2 000 puntos de fondo, pero equivalente con pliegues aleatorios).",
              ],
            },
            {
              note: "Coste: cada bosque predice un millón de celdas en unos 5 s (500 árboles, 8 hilos), y los modelos de pliegue también se predicen para el mapa de incertidumbre: unos 25 s por especie para la Francia metropolitana a 1 km con 4 bloques. El ensamble cuesta MaxEnt más el bosque; los modelos nulos reajustan el ajuste seleccionado, bosques incluidos.",
              tone: "info",
            },
            {
              p: "Estos resultados son promedios sobre muchas especies, no garantías para la suya: si MaxEnt y el bosque dan mapas distintos, ninguno es «el correcto» por defecto.",
            },
          ],
        },
        reading: {
          title: "Leer los resultados de un bosque aleatorio",
          blocks: [
            {
              ul: [
                "**Importancia de las variables** (pestaña Variables): importancia por permutación, la caída del AUC de entrenamiento al mezclar los valores de una variable (3 repeticiones), normalizada a 100. Indica cuánto se apoya el bosque en una variable, no su efecto ecológico; las variables correlacionadas se reparten el mérito.",
                "**Dependencia parcial** (en lugar de curvas de respuesta): la idoneidad media cuando una variable varía, promediada sobre los valores observados de las demás (Friedman 2001). Las curvas de respuesta de MaxEnt fijan las demás variables en su media, así que ambos tipos de curva no son directamente comparables. Las curvas del bosque son escalonadas y planas fuera del rango de entrenamiento.",
                "**Archivos**: `model.rf.json` contiene el bosque (formato, ajustes, semilla, árboles); `partial_dependence.csv`, las curvas. Con el ensamble, `model.lambdas` es el miembro MaxEnt.",
              ],
            },
          ],
        },
      },
    },
    "field-data": {
      title: "Sus datos de campo",
      summary:
        "Importe las observaciones de su organización (CSV), úselas con GBIF o en su lugar, o resérvelas para validar el modelo con datos de presencia–ausencia independientes.",
      sections: {
        overview: {
          title: "Para qué sirve",
          blocks: [
            {
              p: "Muchas consultoras tienen sus propios muestreos: inventarios, transectos, parcelas con protocolo, a menudo con lugares donde se buscó la especie y **no se encontró**. NicheRS puede usarlos de dos maneras (ADR-0108):",
            },
            {
              ul: [
                "**Como presencias**, junto con GBIF o en su lugar: sus presencias pasan exactamente por las mismas reglas de limpieza que los registros de GBIF.",
                "**Reservados para la validación**: el modelo se ajusta solo con GBIF y luego se evalúa en sus sitios prospectados (presencias y ausencias). Es el único control independiente de un mapa y la única vía hacia el nivel *Validated*.",
              ],
            },
            {
              note: "Estas opciones se proponen en el ADR-0108, pendientes de revisión por los ecólogos de Natural Solutions. Un estudio sin sus datos se calcula exactamente como antes.",
              tone: "info",
            },
          ],
        },
        import: {
          title: "Importar un archivo",
          blocks: [
            {
              p: "En el paso **Datos** (modo Experto), abra *Datos de campo de la organización* e importe un archivo CSV (UTF-8 o Windows-1252, separador `;`, `,` o tabulación). Excel: «Guardar como CSV».",
            },
            {
              ol: [
                "Elija el archivo: se lee **en su navegador**; se muestra un extracto y todavía no se envía nada.",
                "Indique la columna del nombre de la especie, de X y de Y, y si es posible la fecha o el año, la precisión de las coordenadas en metros, el estado presencia/ausencia y el protocolo o muestreo.",
                "Elija el sistema de coordenadas (sección siguiente) y, para una columna de estado, marque los valores que significan «prospectado, no encontrado».",
                "Importe: solo se envían las columnas elegidas. El informe de importación da las filas conservadas y rechazadas, con el motivo y las líneas del archivo de las primeras filas rechazadas.",
              ],
            },
            {
              p: "Las filas rechazadas son problemas del archivo (sin nombre de especie, coordenadas ilegibles, fuera del sistema elegido, fecha o precisión ilegibles); son distintas de las reglas de limpieza científicas aplicadas después. Una fecha vacía no es un rechazo: el registro queda «sin fecha» para la limpieza.",
            },
            {
              note: "Un conjunto de datos no se modifica: para cambiar la asignación de columnas, importe de nuevo el archivo. Eliminar un conjunto no afecta a los estudios ya lanzados: cada uno conserva su propia copia, que se elimina con el estudio.",
              tone: "info",
            },
          ],
        },
        crs: {
          title: "Sistemas de coordenadas",
          blocks: [
            {
              p: "Las coordenadas se convierten a longitud/latitud WGS 84 en el servidor, con fórmulas comprobadas frente a PROJ. La lista es cerrada: WGS 84 en grados (EPSG:4326), Lambert-93 (EPSG:2154, la mayoría de datos franceses), LAEA Europa (EPSG:3035) y los husos UTM 28N–32N (ETRS89 y WGS 84).",
            },
            {
              p: "RGF93 y ETRS89 se asimilan a WGS 84 (menos de 1 m de diferencia, muy por debajo de la celda de ~1 km). Las coordenadas fuera del rango plausible del sistema elegido se rechazan «fuera del sistema elegido»: normalmente se eligió un sistema equivocado o la longitud y la latitud están intercambiadas.",
            },
            {
              note: "Lambert II extendido (EPSG:27572) y los sistemas de ultramar aún no se ofrecen: convierta esos archivos a Lambert-93 o WGS 84 antes.",
              tone: "warn",
            },
          ],
        },
        matching: {
          title: "Emparejar los nombres de especies",
          blocks: [
            {
              p: "Cada modelo corresponde a un taxón de la taxonomía de GBIF. Los nombres distintos de su columna de especie se emparejan con el servicio de correspondencia de GBIF: **solo se envían los nombres a GBIF**, nunca coordenadas ni recuentos.",
            },
            {
              p: "Una correspondencia exacta en rango especie o inferior se propone automáticamente; una aproximada debe revisarse; una en un rango superior (género…) no se modela. Usted decide: asigne cada nombre a una especie del estudio (o añada la especie encontrada) y deje sin usar los nombres que no quiera. Varios nombres pueden ir a una misma especie (sinónimos, subespecies).",
            },
          ],
        },
        sources: {
          title: "Fuentes por especie",
          blocks: [
            {
              table: {
                caption:
                  "Cómo puede usar una especie los datos de la organización",
                head: ["Opción", "Datos de entrenamiento", "Sus ausencias"],
                rows: [
                  ["Solo GBIF", "registros de GBIF", "no se usan"],
                  [
                    "GBIF + sus presencias",
                    "sus presencias primero, luego GBIF; una celda registrada por ambos cuenta una vez, como suya",
                    "nunca son presencias; se evalúan si es posible (ver validación)",
                  ],
                  [
                    "Solo sus presencias",
                    "sus presencias; sin descarga de GBIF",
                    "nunca son presencias",
                  ],
                  [
                    "GBIF para el entrenamiento, sus datos reservados",
                    "solo registros de GBIF",
                    "con sus presencias, sitios de validación independiente",
                  ],
                ],
              },
            },
            {
              p: "La pestaña Datos de los resultados muestra, para cada fuente, los registros, las celdas de presencia conservadas y por qué no se conservaron las demás, y cuántos registros de GBIF caen en celdas ya cubiertas por sus datos.",
            },
          ],
        },
        cleaning: {
          title: "Las mismas reglas que GBIF",
          blocks: [
            {
              p: "Sus presencias pasan las mismas reglas, en el mismo orden: fechadas, posteriores al inicio del periodo climático, precisión dentro del límite, dentro del área de estudio, en tierra, con datos ambientales, un registro por celda, filtrado espacial opcional. Solo dos diferencias: la regla de licencia acepta siempre los datos propios de la organización, y un **registro de ausencia nunca es una presencia** (regla *absence*, la primera).",
            },
            {
              note: "Los datos de campo suelen estar agrupados (un transecto, un sitio de estudio). Cuando sus celdas de presencia abarcan menos del 10 % de la extensión del área de estudio, una nota recuerda que las zonas no prospectadas no son ausencias.",
              tone: "info",
            },
          ],
        },
        validation: {
          title: "Validación independiente",
          blocks: [
            {
              p: "Con datos reservados para la validación, el mapa final se evalúa en sus sitios prospectados. Los registros pasan las mismas reglas de limpieza y se reducen a **un sitio por celda** (una celda con alguna detección es un sitio de presencia).",
            },
            {
              note: "**Regla anti-fuga.** Un sitio en una celda usada como presencia de entrenamiento (GBIF o sus datos) se excluye: el modelo ya conoce su resultado. También elimina sus observaciones que GBIF republica (SINP → INPN → GBIF) en la misma celda.",
              tone: "warn",
            },
            {
              ul: [
                "**AUC (presencia–ausencia)**: probabilidad de que un sitio de presencia quede por encima de uno de ausencia.",
                "**Sensibilidad, especificidad, TSS y kappa** en dos umbrales tomados de los datos de entrenamiento (percentil 10 de las presencias de entrenamiento, máximo de sensibilidad + especificidad), nunca ajustados con sus datos (Allouche et al. 2006). Lea el TSS junto con la sensibilidad y la especificidad: depende de la prevalencia cuando los errores están desequilibrados (Leroy et al. 2018).",
                "**Puntuación de Brier y tabla de calibración**: solo diagnósticos. La idoneidad es relativa, no una probabilidad de presencia; muestran con qué frecuencia estaban ocupados los sitios según la idoneidad, sin convertirla en probabilidad.",
                "**Distribución**: extensión de los sitios y distancia media entre sitios vecinos. Una validación en un pequeño grupo de sitios dice poco del resto del área.",
              ],
            },
            {
              p: "Se necesitan al menos 10 sitios de presencia y 10 de ausencia para calcular las métricas. Con las ausencias de un conjunto cuyas presencias entrenaron el modelo, no queda ningún sitio de presencia independiente: reserve el conjunto para evaluar con él.",
            },
          ],
        },
        validated: {
          title: "El nivel Validated",
          blocks: [
            {
              p: "*Validated* está por encima de *Supported*. Solo se asigna si se cumplen **todos** los criterios: datos reservados del entrenamiento; al menos 30 sitios de presencia y 30 de ausencia tras la regla anti-fuga; AUC presencia–ausencia ≥ 0,75; TSS ≥ 0,40 en el umbral máx. SSS de los datos de entrenamiento; y el modelo cumple todos los criterios de *Supported* (tras cualquier limitación).",
            },
            {
              note: "Estos criterios se proponen en el ADR-0108 y esperan la revisión de los ecólogos de Natural Solutions. Cuando se usaron datos independientes, los motivos del nivel lo indican y enumeran cada criterio (✓/✗); si no, mantienen «No independent validation data…».",
              tone: "info",
            },
          ],
        },
        confidentiality: {
          title: "Confidencialidad",
          blocks: [
            {
              ul: [
                "Solo las columnas elegidas salen de su navegador; observadores, comentarios y demás columnas nunca.",
                "Los conjuntos de datos solo son visibles para su organización; otra organización recibe «no encontrado». El soporte de Natural Solutions no los lista.",
                "Las coordenadas nunca aparecen en los registros del servidor, el manifiesto ni los informes: estos recogen la huella SHA-256 del archivo, la asignación de columnas, el sistema y los recuentos. Sus puntos solo aparecen en el mapa de su estudio y en su archivo de ocurrencias (visibles para su organización).",
                "Los conjuntos se eliminan con el mismo plazo de conservación que los estudios, o cuando usted quiera.",
              ],
            },
          ],
        },
      },
    },
    future: {
      title: "Proyecciones con clima futuro",
      summary:
        "Proyectar un modelo sobre los climas futuros CHELSA CMIP6: periodos, escenarios de emisiones (SSP) y modelos climáticos (GCM), cómo leer los mapas de cambio y sus límites.",
      sections: {
        overview: {
          title: "Qué hace una proyección",
          blocks: [
            {
              p: "En modo Experto (paso del modelo, *Proyecciones con clima futuro*) un estudio puede proyectar su modelo final sobre climas futuros. El modelo se ajusta una sola vez con el clima 1981–2010; para cada escenario, cada predictor **climático** se sustituye por su valor futuro según cada modelo climático elegido y el modelo vuelve a predecir. Relieve, cobertura del suelo y agua conservan sus valores actuales.",
            },
            {
              p: "Las capas futuras son las climatologías CHELSA v2.1 CMIP6 (Karger et al. 2017), reducidas a 30″ a partir de cinco modelos climáticos ISIMIP3b; se leen exactamente en la misma ventana de la malla y se agregan como las capas actuales. Propuesto en el ADR-0016, pendiente de revisión por los ecólogos de Natural Solutions.",
            },
            {
              note: "Una proyección muestra dónde podría haber **condiciones climáticas favorables** en el futuro. No es una predicción de dónde vivirá la especie: no se modelan dispersión, dinámica de poblaciones, adaptación, interacciones bióticas ni cambios de uso del suelo.",
              tone: "warn",
            },
          ],
        },
        scenarios: {
          title: "Periodos, SSP y modelos climáticos",
          blocks: [
            {
              ul: [
                "**Periodos**: climatologías de 30 años 2011–2040, 2041–2070 y 2071–2100.",
                "**SSP** (trayectorias socioeconómicas compartidas; O'Neill et al. 2016): SSP1-2.6 mitigación fuerte (unos 2 °C en 2100), SSP3-7.0 rivalidad regional con poca mitigación (unos 3,5–4 °C), SSP5-8.5 desarrollo basado en combustibles fósiles (el más cálido). Son escenarios, no predicciones: ninguno es más probable que otro, compare al menos dos.",
                "**Modelos climáticos (GCM)**: GFDL-ESM4, IPSL-CM6A-LR, MPI-ESM1-2-HR, MRI-ESM2-0 y UKESM1-0-LL. Dan futuros distintos para el mismo escenario; **use los cinco** para que esa dispersión sea visible (Thuiller et al. 2019).",
              ],
            },
            {
              note: "Coste: cada combinación escenario × modelo lee una vez las capas climáticas del modelo (descargadas la primera vez y luego en caché) y predice una vez toda el área de estudio; un estudio con 5 modelos cuesta unas 5 predicciones actuales por escenario. Como máximo 15 combinaciones escenario × modelo por estudio.",
              tone: "info",
            },
          ],
        },
        reading: {
          title: "Leer los mapas y las superficies",
          blocks: [
            {
              ul: [
                "**Idoneidad futura**: la media, entre modelos climáticos, de la idoneidad relativa predicha con el clima futuro. Misma escala que el mapa actual, nunca una probabilidad.",
                "**Dispersión entre modelos**: la desviación típica de esa idoneidad entre los modelos. Valores altos indican que los modelos discrepan en la celda.",
                "**Cambio en las condiciones favorables**: el umbral P10 actual aplicado a ambos periodos. Cada celda es *favorable hoy y en el futuro*, *deja de ser favorable*, *nuevamente favorable* o *desfavorable en ambos*, según la media de los modelos. Celdas pálidas: menos de una mayoría de modelos coincide con esa clase.",
                "**Rayado**: condiciones futuras fuera del rango de los datos de entrenamiento en al menos un modelo (MESS futuro < 0). Ahí las proyecciones son extrapolaciones; no las interprete sin decirlo.",
              ],
            },
            {
              p: "La tabla de superficies da cada clase en km² (superficies exactas de las celdas sobre el elipsoide) y como proporción de la superficie favorable actual, la parte en la que todos los modelos coinciden y el rango de la superficie favorable futura entre modelos. Indique el rango, no solo la media.",
            },
          ],
        },
        limits: {
          title: "Límites",
          blocks: [
            {
              ul: [
                "**Sin dispersión**: *nuevamente favorable* designa condiciones que la especie podría tolerar, no lugares a los que llegará; *deja de ser favorable* no significa extinción local en una fecha dada.",
                "**Solo cambia el clima**: cobertura y uso del suelo, relieve y agua siguen como hoy, aunque también cambiarán.",
                "**Transferencia del modelo**: relaciones ajustadas con las condiciones 1981–2010 pueden no mantenerse con otro clima (Yates et al. 2018), sobre todo donde las condiciones futuras son nuevas (rayado; Elith et al. 2010).",
                "**La incertidumbre de los modelos climáticos** se muestra, no se elimina; la diferencia entre SSP suele ser mayor a final de siglo.",
                "Escriba «cambio en la superficie de condiciones favorables», nunca «desplazamiento del área de distribución» ni «distribución futura».",
              ],
            },
          ],
        },
        files: {
          title: "Ficheros y cita",
          blocks: [
            {
              p: "Por escenario: `future_<periodo>_<ssp>_mean.tif` (idoneidad media), `_sd.tif` (dispersión), `_change.tif` (clases 1 desfavorable en ambos, 2 deja de ser favorable, 3 nuevamente favorable, 4 favorable en ambos), `_agreement.tif` (modelos que coinciden con la clase) y `_mess.tif` (MESS futuro más bajo). `future_areas.csv` lista las superficies por escenario y por modelo; los informes tienen una sección *Future climate projections* y `manifest.json` lista cada fichero futuro leído.",
            },
            {
              p: "Cite CHELSA (Karger et al. 2017), CMIP6 (Eyring et al. 2016) y la corrección de sesgo ISIMIP3b (Lange 2019), y agradezca al Programa Mundial de Investigaciones Climáticas (WCRP) y a los cinco grupos de modelización: los informes incluyen este texto. Las salidas de los modelos CMIP6 están bajo CC BY 4.0; la licencia de los ficheros futuros de CHELSA está pendiente de confirmación con CHELSA.",
            },
          ],
        },
      },
    },
    decision: {
      title: "Apoyo a la decisión",
      summary:
        "Prioridad de prospección, huellas de proyecto, especies con condiciones favorables y comparación de dos estudios: resultados calculados a partir de un estudio terminado para apoyar una decisión.",
      sections: {
        overview: {
          title: "Para qué sirven estos resultados",
          blocks: [
            {
              p: "Un mapa de idoneidad relativa no responde por sí solo a la pregunta de un cliente. La pestaña **Apoyo a la decisión** de un estudio terminado calcula, a petición, cuatro resultados que sí lo hacen: dónde prospectar, qué abarca la huella de un proyecto, qué zonas importan para varias especies y cuánto cambia un mapa entre dos estudios.",
            },
            {
              ul: [
                "Se calculan a partir de los resultados guardados del estudio: **no se reajusta ningún modelo**, y los archivos y el manifiesto del estudio no cambian.",
                "Heredan todos los límites de los modelos de los que proceden: lea primero el nivel de evidencia de cada especie.",
              ],
            },
            {
              note: "Estos resultados no sustituyen a un inventario de campo ni a un estudio de impacto. Una idoneidad baja no es prueba de ausencia.",
              tone: "warn",
            },
          ],
        },
        priority: {
          title:
            "Prioridad de prospección: zonas que merece la pena prospectar",
          blocks: [
            {
              p: "**A qué responde:** dónde merece más la pena prospectar una especie en el campo — lugares de idoneidad relativa más alta y poco muestreados. **A qué no responde:** dónde está la especie. Una prioridad alta significa «merece la pena buscar», no «presente».",
            },
            {
              p: "Cada celda del área de estudio recibe una puntuación de 0 a 1: su rango de idoneidad multiplicado por lo poco que se ha muestreado. Cuatro clases cruzan la idoneidad (por encima o por debajo del umbral P10 de la especie) con el muestreo (menos o más que la celda mediana):",
            },
            {
              ul: [
                "**Prospectar primero** — idoneidad más alta, poco muestreada.",
                "**Conocida** — idoneidad más alta, ya bien muestreada.",
                "**Idoneidad más baja, poco muestreada** e **idoneidad más baja, bien muestreada**.",
              ],
            },
            {
              p: "**Cómo se mide el muestreo.** Si el estudio usó un fondo de grupo objetivo, el esfuerzo de muestreo es el número de registros del grupo objetivo por celda ([sesgo de muestreo](#data)). Si no, NicheRS usa una aproximación, la **distancia al registro conocido más cercano de la especie**, y lo indica. Esa aproximación es más débil: un lugar lejos de los registros puede haberse prospectado sin encontrar la especie.",
            },
            {
              p: "Las zonas candidatas se extraen del 10 % del área de estudio con las puntuaciones de prioridad más altas: grupos de celdas vecinas «prospectar primero», ordenados por su puntuación media, con centroide, extensión, superficie y proporción donde el modelo extrapola. Se descargan en GeoJSON y CSV para planificar el trabajo de campo. El muestreo guiado por modelos ha ayudado a encontrar especies raras (Guisan et al. 2006; Le Lay et al. 2010).",
            },
          ],
        },
        footprint: {
          title: "Huella de proyecto: un prediagnóstico",
          blocks: [
            {
              p: "Suba la huella de un proyecto (polígono GeoJSON en longitud/latitud) a un estudio terminado. Para cada especie, NicheRS indica lo que dicen los modelos dentro de ella:",
            },
            {
              ul: [
                "la superficie con condiciones favorables dentro de la huella con el umbral P10, con la horquilla entre el umbral más inclusivo (MTP) y el de máxima sensibilidad + especificidad;",
                "la proporción de la huella que representa, y la proporción de la superficie favorable de la especie en toda el área de estudio que cubre la huella;",
                "la idoneidad relativa media y máxima, la incertidumbre y las proporciones donde el modelo extrapola o queda fuera de su área de aplicabilidad;",
                "el nivel de evidencia de cada modelo.",
              ],
            },
            {
              p: "Las superficies son exactas: la huella se interseca con cada celda del modelo sobre el elipsoide WGS84. Pero los valores del modelo corresponden a celdas enteras: una huella más pequeña que una celda se describe por las celdas que toca, y NicheRS lo advierte.",
            },
            {
              note: "Solo un prediagnóstico: no sustituye a un inventario de campo ni a un estudio de impacto, y una idoneidad baja dentro de una huella no es prueba de que la especie esté ausente.",
              tone: "warn",
            },
            {
              p: "Las huellas se guardan con el estudio, solo las ve su organización y se eliminan con el estudio.",
            },
          ],
        },
        richness: {
          title: "Especies con condiciones favorables y puntos calientes",
          blocks: [
            {
              p: "En un estudio con varias especies, NicheRS apila sus mapas. Cada celda recibe el **número de especies con condiciones favorables** (cada especie con su propio umbral P10) y la **suma de idoneidades relativas**, un índice para ordenar celdas.",
            },
            {
              note: "No es el número de especies presentes. Apilar modelos sobrestima: cada mapa muestra dónde las condiciones convienen a una especie, no dónde vive, e ignora la competencia, la dispersión y cuántas especies puede albergar un lugar (Calabrese et al. 2014; Guisan & Rahbek 2011).",
              tone: "warn",
            },
            {
              ul: [
                "Solo se apilan las especies con el nivel de evidencia mínimo o superior — Indicative por defecto, así que **los modelos Exploratory quedan fuera** salvo que baje el mínimo. Las especies excluidas se listan con el motivo.",
                "Los **puntos calientes** son el 10 % de las celdas con más especies con condiciones favorables (empates resueltos por la suma de idoneidades). El corte del 10 % es una convención, no un umbral ecológico.",
              ],
            },
          ],
        },
        compare: {
          title: "Comparar dos estudios",
          blocks: [
            {
              p: "Compare dos estudios terminados de la misma especie — por ejemplo fondo aleatorio frente a grupo objetivo, MaxEnt frente a bosque aleatorio, o solo clima frente a clima + cobertura del suelo. NicheRS muestra los ajustes que difieren, las métricas, niveles de evidencia y superficies de ambos estudios, y un mapa de la diferencia (B − A).",
            },
            {
              ul: [
                "Los dos estudios deben estar en la **misma malla** (misma área de estudio y resolución); si no, la comparación se rechaza en lugar de remuestrear.",
                "La **D de Schoener** (0 = sin solapamiento, 1 = idénticas) mide el parecido de las dos superficies de idoneidad una vez escaladas a suma 1 (Warren et al. 2008); la **correlación de Spearman** compara los rangos de las celdas.",
                "La concordancia con el umbral P10 da la superficie favorable en ambos estudios, en uno solo o en ninguno.",
              ],
            },
            {
              p: "Es una prueba de sensibilidad: indica cuánto depende la respuesta de una elección, no qué estudio tiene razón. Juzgue el ajuste con las métricas de validación de cada estudio, teniendo en cuenta que diseños de validación distintos no son directamente comparables.",
            },
          ],
        },
        files: {
          title: "Archivos y trazabilidad",
          blocks: [
            {
              p: "Cada resultado se guarda en una carpeta `analyses/` del estudio, con sus ráster (GeoTIFF), tablas (CSV, GeoJSON), una página imprimible (`report.html`) y su propio manifiesto. Ese manifiesto registra la suma de control del manifiesto de ejecución del estudio y de cada archivo del estudio leído, el método y los ajustes, para poder remontarse al estudio exacto.",
            },
            {
              p: "Los resultados y el manifiesto del estudio nunca cambian. Eliminar un estudio elimina sus resultados de apoyo a la decisión.",
            },
          ],
        },
      },
    },
    exports: {
      title: "Exportaciones y reproducibilidad",
      summary:
        "Rásteres GeoTIFF, GeoJSON, .lambdas, curvas de respuesta, un informe imprimible y el manifiesto de ejecución.",
      sections: {
        files: {
          title: "Archivos por especie",
          blocks: [
            {
              table: {
                caption: "Archivos exportados",
                head: ["Archivo", "Contenido"],
                rows: [
                  [
                    "`suitability.tif`",
                    "Idoneidad relativa del hábitat (cloglog 0–1) del modelo final; no es una probabilidad.",
                  ],
                  [
                    "`uncertainty.tif`",
                    "Desviación típica de la idoneidad entre los modelos de validación cruzada.",
                  ],
                  [
                    "`mess.tif`",
                    "MESS; los valores negativos son extrapolación.",
                  ],
                  [
                    "`effort.tif`",
                    "Solo con fondo por grupo objetivo: registros de GBIF del grupo objetivo por celda (registros, no visitas ni observadores; sintético en la demostración).",
                  ],
                  [
                    "`model.lambdas`",
                    "Modelo final en el formato `.lambdas` de MaxEnt 3.4.4.",
                  ],
                  [
                    "`occurrences.geojson`",
                    "Presencias conservadas y registros excluidos con su motivo (con límite; recuentos exactos en el informe). Coordenadas tal como las publica GBIF: consulte el référentiel regional del SINP antes de reproducir puntos en un informe público.",
                  ],
                  [
                    "`report.md`",
                    "Resumen de métodos y resultados (estilo ODMAP), punto de partida para el apartado de métodos de un informe.",
                  ],
                  [
                    "`report.html`",
                    "Informe imprimible: mapas con las presencias, incertidumbre, extrapolación, curvas de respuesta, tablas, límites y citas. «Abrir el informe imprimible» lo muestra en el navegador; imprímalo en PDF para adjuntarlo a un estudio.",
                  ],
                  [
                    "`response_curves.csv`",
                    "Curvas de respuesta marginales del modelo final (`variable,x,y`, 501 puntos por variable).",
                  ],
                  [
                    "`model.rf.json`",
                    "Solo bosque aleatorio o ensamble: el bosque aleatorio submuestreado final (ajustes, semilla, árboles; JSON). Con el ensamble, `model.lambdas` es su miembro MaxEnt.",
                  ],
                  [
                    "`partial_dependence.csv`",
                    "Solo bosque aleatorio o ensamble: dependencia parcial del bosque final (`variable,x,y`, 51 puntos por variable).",
                  ],
                  [
                    "`manifest.json`",
                    "Manifiesto de ejecución del estudio (botón «Descargar el manifiesto de ejecución»).",
                  ],
                ],
              },
            },
            {
              p: "Para una especie *Insufficient* solo se generan `occurrences.geojson`, `report.md` y `report.html`.",
            },
          ],
        },
        geotiff: {
          title: "Formato GeoTIFF",
          blocks: [
            {
              p: "Los rásteres son GeoTIFF Float32 de una banda en coordenadas geográficas WGS84 (EPSG:4326) sobre la malla nativa de CHELSA, en teselas de 256 × 256 con compresión Deflate, organizados como un Cloud-Optimized GeoTIFF pero sin vistas generales (overviews). NoData es NaN. Se abren directamente en QGIS y GDAL.",
            },
            {
              p: "Para calcular superficies en un SIG, use áreas elipsoidales o una proyección equivalente, nunca un número de celdas.",
            },
          ],
        },
        manifest: {
          title: "Manifiesto de ejecución",
          blocks: [
            {
              p: "El manifiesto registra lo necesario para trazar un estudio: la especificación completa (especies, área de estudio, filtros, ajustes del modelo y de la validación) y su SHA-256, la semilla aleatoria, la versión del software, la procedencia de los datos ambientales, la fuente GBIF con su descarga, su cita con DOI y su fecha de acceso, los métodos (selección, incertidumbre, extrapolación, superficie, reglas de evidencia) y los resultados por especie con la suma SHA-256 de cada archivo.",
            },
            {
              p: "Los estudios son inmutables. Use «Duplicar y editar» para crear una variante; el nuevo estudio tiene su propio manifiesto. Conserve el manifiesto con el entregable: es la referencia si se cuestiona un resultado.",
            },
          ],
        },
      },
    },

    account: {
      title: "Cuota y cuenta",
      summary:
        "Servicio alojado: especies modelizadas al mes, invitaciones, restablecimiento, borrado y conservación.",
      sections: {
        quota: {
          title: "Cuota mensual: especies modelizadas",
          blocks: [
            {
              p: "El servicio alojado cuenta **especies modelizadas**: una especie cuenta cuando se ajusta un modelo para ella, sea cual sea su nivel de evidencia (*Exploratory* incluido), en el mes natural (UTC) en que termina.",
            },
            {
              ul: [
                "**No cuentan:** las especies *Insufficient* (sin modelo ajustado), las especies fallidas o canceladas, ni la consulta y exportación de resultados existentes.",
                "Un estudio reanudado nunca se cuenta dos veces para la misma especie.",
                "Borrar un estudio no devuelve la cuota.",
                "Al lanzar, las especies ya consumidas, más las reservadas por estudios en cola o en curso, más las nuevas deben caber en la cuota; si no, el estudio se rechaza («Monthly quota reached»). Los resultados existentes siguen disponibles.",
                "La cuota es mensual y no se acumula. El menú de la cuenta, en la barra superior, muestra las especies que quedan este mes; Mi organización → Resumen detalla las consumidas, reservadas y restantes.",
              ],
            },
          ],
        },
        limits: {
          title: "Límites por estudio",
          blocks: [
            {
              ul: [
                "Como máximo 25 especies por estudio.",
                "Extensión del área de estudio de como máximo 2,5 millones de celdas de la malla de modelización (la Francia metropolitana a 1 km tiene unos 2,0 millones).",
                "Los estudios se ponen en cola de forma equitativa entre organizaciones; la primera descarga de GBIF de un estudio puede tardar (cola propia de GBIF).",
              ],
            },
          ],
        },
        access: {
          title: "Invitaciones y contraseñas",
          blocks: [
            {
              ul: [
                "El acceso es solo por invitación, emitida por Natural Solutions o por los **gestores** de su organización (menú de la cuenta → Mi organización): invitan a sus colegas, envían enlaces para restablecer la contraseña y desactivan cuentas de la organización, dentro de su cuota. Pida a Natural Solutions que nombre un gestor.",
                "Un enlace de invitación es de un solo uso y caduca a los 7 días. NicheRS no envía correos; quien le invita le hace llegar el enlace, con un mensaje ya redactado.",
                "Las contraseñas tienen al menos 12 caracteres; cambie la suya en Mi cuenta (sus otras sesiones se cierran). Contraseña olvidada: pida un enlace de restablecimiento a un gestor de su organización o a Natural Solutions (mismo mecanismo); cierra sus sesiones abiertas.",
                "Tras 10 intentos fallidos de inicio de sesión para un mismo correo en 15 minutos, el inicio de sesión queda bloqueado para ese correo hasta que pase ese intervalo.",
              ],
            },
          ],
        },
        gbif: {
          title: "Su cuenta de GBIF",
          blocks: [
            {
              p: "Los estudios descargan los registros de GBIF con una cuenta de GBIF; cada descarga recibe un DOI para citar en los informes. GBIF limita cuántas descargas puede hacer una cuenta a la vez, así que una cuenta compartida muy usada hace esperar a los estudios.",
            },
            {
              ol: [
                "**Su propia cuenta** (menú de la cuenta → Mi cuenta → Cuenta de GBIF), si ha configurado una.",
                "Si no, **la cuenta de su organización**, configurada por un gestor (Mi organización).",
                "Si no, **la cuenta compartida de Natural Solutions**.",
              ],
            },
            {
              ul: [
                "Escriba su **nombre de usuario** de gbif.org (no su correo) y su contraseña. NicheRS pregunta enseguida a GBIF si los acepta; una cuenta que GBIF rechaza se omite hasta que se corrija.",
                "La contraseña se cifra en el servidor, no se vuelve a mostrar y nunca se escribe en los registros. El manifiesto de cada estudio guarda qué tipo de cuenta hizo su descarga (la suya, la de la organización o la compartida), no el nombre de usuario.",
                "Cada cuenta conserva sus propias descargas: un estudio nunca cita una descarga hecha con la cuenta de otra organización.",
                "La cuenta se fija al empezar la preparación de los datos de un estudio; cambiarla después no afecta a los estudios ya preparados.",
              ],
            },
          ],
        },
        retention: {
          title: "Borrado y conservación",
          blocks: [
            {
              ul: [
                "Puede borrar en cualquier momento un estudio terminado; sus archivos se eliminan.",
                "Los estudios terminados se borran automáticamente a los 365 días. Descargue lo que deba conservar (archivos y manifiesto).",
                "Los estudios de su organización solo son visibles para sus miembros. Los administradores de Natural Solutions pueden abrirlos para dar soporte; cada uno de esos accesos queda registrado.",
                "Para que se borre su cuenta (nombre, e-mail y contraseña), pídalo a Natural Solutions: el borrado es inmediato y definitivo; su organización conserva sus estudios. Los nombres de observadores de GBIF nunca se guardan.",
              ],
            },
          ],
        },
      },
    },

    glossary: {
      title: "Glosario",
      summary: "Definiciones breves de los términos usados en NicheRS.",
      sections: {
        terms: {
          title: "Términos",
          blocks: [
            {
              dl: [
                [
                  "Idoneidad relativa del hábitat",
                  "Puntuación MaxEnt de 0 a 1 por celda, relativa al área de estudio; no es una probabilidad de presencia.",
                ],
                [
                  "Cloglog",
                  "Transformación monótona que reescala la salida bruta de MaxEnt a 0–1 (Phillips et al. 2017); no es una calibración.",
                ],
                [
                  "MaxEnt",
                  "Método de modelización presencia-fondo (Phillips et al. 2006); NicheRS usa una versión portada y contrastada con maxent.jar 3.4.4.",
                ],
                [
                  "Registro / observación",
                  "Un registro de GBIF de la especie; observado, no predicho.",
                ],
                [
                  "Celda de presencia",
                  "Celda de la malla con al menos un registro conservado; la unidad que usan el modelo y las reglas de evidencia.",
                ],
                [
                  "Fondo (background)",
                  "10 000 celdas terrestres al azar del área de estudio que describen las condiciones disponibles.",
                ],
                [
                  "Área de estudio",
                  "Área donde se ajusta y proyecta el modelo.",
                ],
                [
                  "Clases de variables (features)",
                  "Formas de respuesta permitidas a MaxEnt: L lineal, Q cuadrática, H bisagra, P producto.",
                ],
                [
                  "Multiplicador de regularización (β)",
                  "Penalización de la complejidad del modelo; valores mayores dan respuestas más suaves.",
                ],
                [
                  "Validación cruzada por bloques espaciales",
                  "Validación en la que cada uno de 4 bloques geográficos se predice con un modelo entrenado con los otros tres.",
                ],
                [
                  "Pliegue (fold)",
                  "Parte de los datos reservada en la validación cruzada.",
                ],
                [
                  "Índice de Boyce continuo (CBI)",
                  "De −1 a 1: cómo ordenan las predicciones las presencias reservadas frente al fondo.",
                ],
                [
                  "OR10",
                  "Fracción de presencias reservadas bajo el umbral del percentil 10 de entrenamiento; se espera ≈ 10 %.",
                ],
                [
                  "ORmtp",
                  "Fracción de presencias reservadas bajo la presencia mínima de entrenamiento; se espera 0 %.",
                ],
                [
                  "AUC presencia-fondo",
                  "Discriminación de presencias frente al fondo; depende del área de estudio; indicador secundario.",
                ],
                [
                  "Regla de umbral",
                  "MTP, P10 o máx. SSS: cómo se elige el valor de idoneidad que delimita las condiciones favorables.",
                ],
                [
                  "Superficie de condiciones favorables",
                  "Suma de las áreas exactas de las celdas por encima de un umbral; no es el área ocupada.",
                ],
                [
                  "MESS / extrapolación",
                  "Similitud ambiental con las condiciones de entrenamiento; los valores negativos indican extrapolación.",
                ],
                [
                  "Incertidumbre (desviación típica entre pliegues)",
                  "Desviación típica por celda de la idoneidad entre los modelos de los pliegues.",
                ],
                [
                  "Nivel de evidencia",
                  "*Insufficient*, *Exploratory*, *Indicative* o *Supported*, según reglas fijas (ADR-0009).",
                ],
                [
                  "Colinealidad",
                  "Correlación entre predictores; |r| > 0,7 hace que se descarte una variable.",
                ],
                [
                  "CHELSA v2.1",
                  "Climatologías mundiales 1981–2010 a 30″, licencia CC0 (Karger et al. 2017).",
                ],
                [
                  "Proporción de cobertura del suelo",
                  "Parte (0–1) de una celda cubierta por un grupo de clases de ESA WorldCover 2021.",
                ],
                [
                  "DOI de la descarga de GBIF",
                  "Identificador permanente del conjunto exacto de registros de GBIF usado; hay que citarlo.",
                ],
                [
                  "Manifiesto de ejecución",
                  "Archivo JSON con las entradas, versiones, semilla, ajustes y sumas de control de los archivos de un estudio.",
                ],
                [
                  "Especie modelizada",
                  "Unidad de la cuota: una especie para la que se ajusta un modelo.",
                ],
                [
                  "Demostración sintética",
                  "Conjunto de datos artificial integrado para aprender la herramienta; nunca datos reales.",
                ],
              ],
            },
          ],
        },
      },
    },

    responsible: {
      title: "Límites y uso responsable",
      summary:
        "Quién responde de la interpretación, cómo citar resultados en un informe para un cliente, atribución y límites conocidos.",
      sections: {
        responsibility: {
          title: "Responsabilidad",
          blocks: [
            {
              note: "NicheRS es una herramienta de apoyo a la decisión. **La interpretación de los resultados y su uso en un entregable son responsabilidad del profesional que firma el informe.**",
              tone: "warn",
            },
            {
              p: "El software aplica reglas documentadas. No conoce el sitio, ni la ecología de la especie más allá de los registros, ni el contexto reglamentario.",
            },
          ],
        },
        citing: {
          title: "Citar un resultado en un informe para un cliente",
          blocks: [
            {
              ol: [
                "Indique el software y su versión (en `report.md`) y el método: MaxEnt (Phillips et al. 2006, 2017), versión portada por NicheRS de MaxEnt 3.4.4.",
                "Adapte el texto de métodos de `report.md`: datos, filtros, exclusiones, predictores, calibración, validación.",
                "Dé el nivel de evidencia y sus motivos literalmente, incluida la frase sobre la ausencia de validación independiente.",
                "Describa la salida como «idoneidad relativa del hábitat (cloglog, 0–1), que no es una probabilidad de presencia».",
                "Dé cada superficie con su regla de umbral y la horquilla según las reglas.",
                "Muestre el rayado de extrapolación y el mapa de incertidumbre, o indique sus valores.",
                "Cite el DOI de la descarga de GBIF y CHELSA; archive `manifest.json` (y el identificador del estudio) con el entregable.",
              ],
            },
          ],
        },
        attribution: {
          title: "Atribución: GBIF y CHELSA",
          blocks: [
            {
              ul: [
                "GBIF: *GBIF.org (fecha) GBIF Occurrence Download* seguido del DOI del estudio. Los registros CC BY se atribuyen mediante esta cita.",
                "CHELSA: Karger D.N. et al. (2017) Climatologies at high resolution for the earth's land surface areas. *Scientific Data* 4:170122, doi:10.1038/sdata.2017.122; datos CHELSA v2.1, doi:10.16904/envidat.228.",
                "Máscara terrestre: Natural Earth (dominio público).",
              ],
            },
          ],
        },
        avoid: {
          title: "Nunca",
          blocks: [
            {
              ul: [
                "Nunca escriba «probabilidad de presencia», un porcentaje de confianza, «área de distribución» ni «área ocupada» sobre estos resultados.",
                "Nunca quite el rayado de extrapolación de un mapa publicado.",
                "Nunca compare puntuaciones de idoneidad ni el AUC entre especies o áreas de estudio.",
                "Nunca use una idoneidad baja como prueba de ausencia en un estudio de impacto.",
                "Nunca base una decisión reglamentaria solo en un mapa *Exploratory*.",
                "Nunca use resultados de la demostración sintética, ni resultados con registros CC BY-NC, en un entregable facturado.",
              ],
            },
          ],
        },
        limits: {
          title: "Límites conocidos de esta versión",
          blocks: [
            {
              ul: [
                "Solo especies terrestres; clima 1981–2010 por defecto (relieve, cobertura del suelo 2021 y distancia al agua opcionales; sin suelos; proyecciones con clima futuro opcionales).",
                "Sin corrección del sesgo de muestreo por defecto (el fondo por grupo objetivo es una opción Experto que necesita la réplica regional de GBIF); validación independiente solo con los datos de presencia–ausencia de su organización (opción Experto, [Sus datos de campo](#field-data)): sin ellos, la evidencia procede de la validación cruzada de los mismos registros.",
                "MaxEnt por defecto; un bosque aleatorio submuestreado y un ensamble MaxEnt + bosque aleatorio son opciones expertas pendientes de revisión ([Algoritmos de modelización](#algorithms)).",
                "Las celdas costeras dependen de una máscara terrestre con una precisión de unos 0,5–1 km.",
              ],
            },
          ],
        },
      },
    },
  },
};
