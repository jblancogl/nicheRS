// Aide en français. Structure imposée par `en.ts` (TranslationOf) : mêmes articles, mêmes sections.
// Vocabulaire aligné sur docs/offer : « idonéité relative », « niveau de preuve », « bureau d'études ».
// Les noms des niveaux (Exploratory, Indicative…) restent en anglais, comme dans l'interface.

import type { EnContent } from "./en";
import type { TranslationOf } from "./types";

export const fr: TranslationOf<EnContent> = {
  ui: {
    help: "Aide",
    helpCentre: "Centre d'aide",
    openHelp: "Ouvrir l'aide",
    close: "Fermer l'aide",
    language: "Langue de l'aide",
    search: "Rechercher dans l'aide",
    searchPlaceholder: "Rechercher dans l'aide…",
    noResults: "Aucun article ne correspond à votre recherche.",
    articles: "Articles",
    onThisPage: "Dans cet article",
    helpOn: "Aide :",
    openPage: "Ouvrir en pleine page",
    backToApp: "Retour à NicheRS",
    back: "Tous les articles",
    previous: "Précédent",
    next: "Suivant",
    reviewNote:
      "Contenu d'aide en attente de relecture par les écologues de Natural Solutions. Il décrit les règles mises en œuvre dans cette version de NicheRS (ADR-0009) ; pour une étude donnée, son rapport et son manifeste font foi.",
    languageNote:
      "L'aide et l'interface sont disponibles en anglais, français et espagnol. La langue se choisit dans le menu utilisateur, à droite de la barre supérieure, et s'applique à toute l'interface, aide comprise (dans le service hébergé, elle est enregistrée avec votre compte).",
    hintTitle: "Première utilisation de NicheRS ?",
    hintBody:
      "« Prise en main » présente en cinq minutes le parcours, de l'espèce aux résultats, et la lecture de la carte.",
    hintOpen: "Lire la prise en main",
    hintDismiss: "Masquer",
    demoHelp: "Qu'est-ce que la démonstration synthétique ?",
    readingHelp: "Comment lire ces résultats",
    pathHelp: "Que décide le mode rapide à ma place ?",
    decisionHelp:
      "Ce à quoi ces résultats répondent, et ce à quoi ils ne répondent pas",
  },
  articles: {
    "getting-started": {
      title: "Prise en main",
      summary:
        "Ce que fait NicheRS et les sept étapes, de l'espèce aux résultats relus.",
      sections: {
        overview: {
          title: "Ce que fait NicheRS",
          blocks: [
            {
              p: "NicheRS ajuste un modèle de distribution d'espèce MaxEnt à partir des occurrences GBIF et du climat CHELSA, le valide par validation croisée spatiale et cartographie l'**idonéité relative de l'habitat** sur votre zone d'étude.",
            },
            {
              p: "Il indique où les conditions climatiques ressemblent à celles des lieux où l'espèce a été observée. Il n'indique pas où se trouve l'espèce : voir [Ce que montre la carte](#map).",
            },
          ],
        },
        paths: {
          title: "Rapide ou Expert",
          blocks: [
            {
              p: "« Nouvelle étude » demande comment la préparer ; le choix est mémorisé dans ce navigateur et se change à tout moment avec le sélecteur **Rapide / Expert** au-dessus des étapes, sans perdre ce qui a été choisi.",
            },
            {
              ul: [
                "**Rapide** — trois étapes : une espèce, la zone d'étude, puis **Vérifier et lancer**. Le nettoyage des données, les variables climatiques CHELSA, la résolution de la grille (la plus fine que la zone permet) et la calibration du modèle suivent les réglages recommandés ([Nettoyage des données](#data), [Variables et colinéarité](#variables), [Validation](#validation)).",
                "**Expert** — les six étapes décrites ci-dessous : plusieurs espèces à la fois, filtres et résolution des données, choix des variables, réglages du modèle et de la validation.",
              ],
            },
            {
              note: "Les deux parcours utilisent le même modèle validé, affichent les mêmes avertissements sur les données et consignent tous les réglages dans le rapport et le manifeste de l'étude. Une étude rapide n'est pas une analyse allégée : c'est l'analyse recommandée, avec moins de questions.",
              tone: "info",
            },
          ],
        },
        workflow: {
          title: "Le parcours, étape par étape",
          blocks: [
            {
              ol: [
                "**Espèces** — dans l'espace de travail (la page d'accueil, avec vos projets et vos études), « Nouvelle étude » ouvre cette étape, dans le projet sélectionné (« Reprendre cette étude » reprend une étude en préparation dans ce navigateur). Recherchez dans le référentiel taxonomique GBIF et choisissez une ou plusieurs espèces (jusqu'à 25 par étude dans le service hébergé). Choisissez une espèce, pas un genre : un rang supérieur mélange plusieurs niches et plafonne le niveau de preuve à *Exploratory*.",
                "**Zone d'étude** — recherchez un **lieu** — une ou plusieurs communes, intercommunalités, départements ou provinces, régions, pays ou espaces protégés (parcs, réserves, Natura 2000), dont la réunion forme la zone ; ou cliquez sur la carte pour lister les lieux à cet endroit (France et Espagne, limites officielles de l'IGN, de l'INPN et du MITECO hébergées par le serveur) — ou choisissez une zone prédéfinie, dessinez un polygone sur la carte ou importez un fichier GeoJSON en WGS84. Le modèle est ajusté et projeté uniquement dans cette zone, d'où sont tirés les points de fond.",
                "**Données** — NicheRS télécharge les occurrences GBIF, les nettoie et indique combien sont conservées et pourquoi les autres sont exclues. Vérifiez sur la carte l'absence d'occurrences invraisemblables avant de continuer. Les filtres d'année, d'incertitude des coordonnées et de licence, ainsi que la résolution de la grille, se règlent ici. Voir [Nettoyage des données](#data).",
                "**Variables** — choisissez les prédicteurs climatiques CHELSA (un jeu recommandé est présélectionné) et, en option, des prédicteurs de relief, d'occupation du sol et de distance à l'eau ([Relief, occupation du sol et eau](#predictors)). Les variables fortement corrélées sont écartées automatiquement pendant le calcul. Voir [Variables et colinéarité](#variables).",
                "**Modèle** — les réglages recommandés sont préremplis (calibration automatique, validation croisée par blocs spatiaux). Ne les modifiez que pour une raison documentée ; l'étape Synthèse liste toute valeur différente du défaut.",
                "**Lancement** — « Lancer l'étude » à l'étape Synthèse lance le calcul sur le serveur. Vous pouvez quitter la page et revenir par **Études** dans la barre supérieure ; l'avancement et les avertissements s'affichent en direct. Une étude attend d'abord que GBIF prépare le fichier complet des occurrences (avec DOI), en général 2 à 15 minutes dans la file de GBIF : la page montre l'attente, le statut chez GBIF et la dernière vérification, avec un lien vers le téléchargement sur gbif.org. Si GBIF est exceptionnellement lent (30 minutes), vous pouvez continuer sans le téléchargement : les occurrences viennent alors de l'API de recherche GBIF, sans DOI et avec un nombre maximal par espèce.",
                "**Résultats** — pour chaque espèce : **Synthèse** (avertissements, niveau de preuve, surface de conditions favorables, indicateurs clés), **Validation**, **Variables** et **Données et exports**.",
              ],
            },
          ],
        },
        reading: {
          title: "Lire les résultats dans le bon ordre",
          blocks: [
            {
              ol: [
                "Lisez d'abord les **avertissements** (qualité des données, extrapolation, rang taxonomique, téléchargement tronqué).",
                "Lisez le **niveau de preuve** et chacun des motifs listés en dessous. Voir [Niveaux de preuve](#evidence).",
                "Regardez la carte avec les **hachures d'extrapolation** affichées, puis passez à la couche d'**incertitude**.",
                "Ensuite seulement, utilisez la **surface de conditions favorables**, toujours avec sa règle de seuil et sa fourchette. Voir [Fourchette de surface favorable](#area).",
              ],
            },
          ],
        },
        demo: {
          title: "La démonstration synthétique",
          blocks: [
            {
              note: "La démonstration utilise des **données synthétiques**. Ses résultats décrivent une espèce virtuelle dans un environnement artificiel : ils ne doivent jamais figurer dans un livrable ni être présentés comme une prédiction pour une espèce réelle.",
              tone: "warn",
            },
            {
              p: "« Essayer la démonstration synthétique », dans l'espace de travail tant qu'il ne contient aucune étude, ou à l'étape Espèces, exécute tout le parcours sur des données artificielles : une espèce virtuelle, *Virtualis demonstrandum*, à réponse connue à trois prédicteurs synthétiques (température, précipitations, rugosité), sur une région fixe du sud-est de la France (3–6° E, 43,8–45,8° N). Aucune occurrence GBIF ni donnée climatique CHELSA n'est utilisée.",
            },
            {
              p: "Les résultats de démonstration sont signalés comme synthétiques : un badge **SYNTHÉTIQUE** à côté du nom de l'espèce et un avertissement sur la page de résultats, la mention « (SYNTHETIC) », en anglais, dans les noms de l'espèce et de la zone d'étude, et `synthetic_data: true` dans `report.md` et le manifeste. Servez-vous-en pour apprendre l'interface et la lecture des sorties.",
            },
            {
              p: "Dans le service hébergé, une étude de démonstration est une étude ordinaire : lorsque son modèle est ajusté, elle compte pour une espèce modélisée dans votre quota mensuel. En installation locale, la même démonstration est disponible en ligne de commande (`niche demo`).",
            },
          ],
        },
        keyboard: {
          title: "Raccourcis clavier",
          blocks: [
            {
              table: {
                caption: "Raccourcis clavier",
                head: ["Touches", "Action"],
                rows: [
                  ["Flèches / + −", "Déplacer / zoomer la carte active"],
                  ["I", "Lire les valeurs au centre de la carte (résultats)"],
                  ["0", "Zoomer sur la zone d'étude"],
                  [
                    "Entrée / Échap / Retour arrière",
                    "Terminer / annuler / défaire pendant le dessin",
                  ],
                  ["Échap", "Fermer ce panneau d'aide"],
                ],
              },
            },
          ],
        },
      },
    },

    map: {
      title: "Ce que montre la carte",
      summary:
        "L'idonéité relative de l'habitat, la transformation cloglog et ce que la carte n'est pas.",
      sections: {
        suitability: {
          title: "Idonéité relative de l'habitat",
          blocks: [
            {
              p: "La couche principale est l'**idonéité relative de l'habitat** : un score de 0 à 1 calculé par le modèle MaxEnt pour chaque cellule de la grille (environ 1 km) de la zone d'étude. Plus la valeur est élevée, plus les conditions environnementales ressemblent à celles des cellules où l'espèce a été observée, relativement aux conditions disponibles dans la zone d'étude (le fond).",
            },
            {
              ul: [
                "Les scores sont relatifs à cette zone d'étude, à ces occurrences et à ces variables. Ne comparez pas les scores entre espèces ni entre zones d'étude.",
                "La rampe de couleurs est fixe de 0 à 1 et n'est jamais étirée : une carte pâle correspond réellement à des scores faibles.",
              ],
            },
          ],
        },
        cloglog: {
          title: "La transformation cloglog",
          blocks: [
            {
              p: "La sortie brute de MaxEnt est ramenée entre 0 et 1 par la transformation **cloglog** (Phillips et al. 2017). Elle conserve l'ordre des cellules ; elle ne fait pas du score une probabilité calibrée. Avec des données de présence seule et une prévalence inconnue (fréquence de l'espèce dans la zone), aucune transformation ne le peut.",
            },
          ],
        },
        not: {
          title: "Ce que la carte n'est pas",
          blocks: [
            {
              note: "La carte n'est **pas une probabilité de présence**, **pas la surface occupée** (ni l'aire de répartition) et **pas une preuve de présence ou d'absence**.",
              tone: "warn",
            },
            {
              p: "Un score élevé signifie que les conditions d'une cellule ressemblent à celles des occurrences connues. L'espèce peut pourtant être absente (barrières à la dispersion, habitat détruit, compétition, occupation du sol ou structure de l'habitat que les variables climatiques ne décrivent pas). Un score faible ne prouve pas non plus l'absence, surtout dans les secteurs peu prospectés.",
            },
          ],
        },
        layers: {
          title: "Couches de la carte",
          blocks: [
            {
              ul: [
                "**Idonéité (relative, 0–1)** — idonéité relative de l'habitat du modèle final (couche principale).",
                "**Incertitude (écart-type entre plis)** — dispersion de l'idonéité entre les modèles de validation croisée. Voir [Incertitude et extrapolation](#uncertainty).",
                "**Extrapolation (MESS < 0)** — cellules hors des conditions rencontrées à l'apprentissage. Les **hachures d'extrapolation** les montrent par-dessus les autres couches et sont actives par défaut.",
                "**Favorable / non favorable au seuil** — cellules dont l'idonéité atteint le seuil de la règle choisie dans l'onglet Synthèse. Voir [Fourchette de surface favorable](#area).",
                "**Effort d'échantillonnage (données du groupe cible)** — seulement pour les études avec un fond par groupe cible : données GBIF du groupe cible par cellule, en orange sur une échelle logarithmique. C'est l'effort observé, pas une prédiction. Voir [Fond par groupe cible](#data/targetgroup).",
                "**Occurrences** — les occurrences, affichées sous forme de symboles, conservées pour la modélisation ou exclues par un filtre : ce sont des données observées, non prédites.",
              ],
            },
            {
              p: "Cliquez sur une cellule (ou appuyez sur **I**) pour lire ses valeurs locales.",
            },
          ],
        },
        replaces: {
          title: "Ce qu'elle ne remplace pas",
          blocks: [
            {
              ul: [
                "Les inventaires de terrain et les protocoles de prospection.",
                "Une étude d'impact et ses analyses réglementaires.",
                "La connaissance experte du site : habitats, occupation du sol, obstacles, micro-habitats.",
                "Les données de présence des bases naturalistes et les statuts de protection.",
              ],
            },
            {
              p: "Utilisez-la pour hiérarchiser l'effort de prospection, poser les questions et documenter une hypothèse argumentée sur les secteurs aux conditions favorables.",
            },
          ],
        },
      },
    },

    evidence: {
      title: "Niveaux de preuve",
      summary:
        "Insufficient, Exploratory, Indicative et Supported : critères exacts et conduite à tenir.",
      sections: {
        overview: {
          title: "Des règles explicites, sans pourcentage",
          blocks: [
            {
              p: "Chaque espèce reçoit l'un des quatre **niveaux de preuve**, calculé par des règles fixes à partir des données et des résultats de validation (ADR-0009). Il n'y a pas de pourcentage de confiance : le niveau et les motifs listés en dessous (✓ satisfait, ✗ non satisfait) constituent l'énoncé complet.",
            },
          ],
        },
        levels: {
          title: "Niveaux et critères",
          blocks: [
            {
              table: {
                caption: "Niveaux de preuve et critères",
                head: ["Niveau", "Critères (tous requis)", "Signification"],
                rows: [
                  [
                    "*Insufficient* (insuffisant)",
                    "Moins de 10 cellules de présence utilisables après nettoyage",
                    "Aucun modèle n'est ajusté ; rien n'est cartographié.",
                  ],
                  [
                    "*Exploratory* (exploratoire)",
                    "Un modèle est ajusté, mais au moins un critère *Indicative* n'est pas satisfait",
                    "Utile pour explorer des tendances ; pas assez robuste à lui seul pour décider.",
                  ],
                  [
                    "*Indicative* (indicatif)",
                    "≥ 25 cellules de présence ; indice de Boyce continu moyen ≥ 0,40 ; OR10 moyen ≤ 0,25 ; part extrapolée de la zone d'étude ≤ 25 %",
                    "Appui raisonnable ; à utiliser avec l'expertise et les données de terrain.",
                  ],
                  [
                    "*Supported* (étayé)",
                    "≥ 50 cellules de présence ; Boyce moyen ≥ 0,70 ; OR10 moyen ≤ 0,15 ; part extrapolée ≤ 10 % ; validation croisée par blocs spatiaux",
                    "L'adéquation des données et la validation satisfont les critères documentés.",
                  ],
                ],
              },
            },
            {
              p: "Les moyennes portent sur les plis de validation croisée du modèle retenu. Si l'indice de Boyce ne peut pas être calculé, le critère correspondant est considéré comme non satisfait. Les « cellules de présence » sont les cellules de la grille contenant au moins une occurrence conservée (une par cellule). La part extrapolée est la fraction de la zone d'étude où MESS < 0.",
            },
            {
              p: "Au-dessus de *Supported*, le niveau *Validated* exige des données de terrain présence–absence indépendantes, mises de côté de l'apprentissage, et des critères stricts (ADR-0108, proposé) : voir [Vos données de terrain](#field-data/validated). Sans ces données, aucun niveau ne dépasse *Supported*.",
            },
          ],
        },
        independent: {
          title: "Pas de validation indépendante",
          blocks: [
            {
              note: "Chaque résultat indique : « No independent validation data: evidence comes from cross-validation of the same, possibly biased, records » (pas de données de validation indépendantes : la preuve vient de la validation croisée des mêmes occurrences, potentiellement biaisées). Même *Supported* ne signifie pas que la carte a été confrontée à des inventaires de terrain.",
              tone: "warn",
            },
          ],
        },
        taxon: {
          title: "Genre ou famille sélectionné",
          blocks: [
            {
              p: "Si le taxon GBIF choisi est de rang supérieur à l'espèce (genre, famille…), les occurrences de plusieurs espèces aux niches différentes sont mélangées. La revue des données et les résultats affichent un avertissement, et le niveau est plafonné à *Exploratory* quels que soient les indicateurs. Choisissez plutôt l'espèce (ou la sous-espèce).",
            },
          ],
        },
        insufficient: {
          title: "Que faire d'une espèce Insufficient",
          blocks: [
            {
              ul: [
                "Vérifiez les motifs d'exclusion (étape Données ou onglet Données et exports) : de nombreuses occurrences ont pu être écartées par les filtres de licence, de date ou d'incertitude. Assouplir un filtre est possible mais doit être justifié ; c'est consigné dans le rapport et le manifeste, et inclure des occurrences CC BY-NC restreint les résultats à un usage non commercial.",
                "Vérifiez que la zone d'étude ne coupe pas une grande partie de la répartition connue.",
                "Une grille plus grossière n'aide pas : les occurrences sont ramenées à une par cellule, donc des cellules plus grandes donnent moins de cellules de présence.",
                "Si l'espèce est simplement trop peu observée, indiquez qu'elle n'est pas modélisable avec les données disponibles et appuyez-vous sur l'expertise et les inventaires de terrain.",
                "Une espèce *Insufficient* ne compte pas dans le quota mensuel.",
              ],
            },
          ],
        },
        exploratory: {
          title: "Que faire d'une espèce Exploratory",
          blocks: [
            {
              ul: [
                "Lisez les motifs ✗ : ils nomment le critère non satisfait (effectif, indice de Boyce, omission, extrapolation, rang taxonomique).",
                "Peu de cellules de présence : présentez la carte comme une hypothèse à vérifier sur le terrain.",
                "OR10 élevé : le modèle est peut-être surajusté ou se transfère mal d'une région à l'autre ; regardez les valeurs par pli dans l'onglet Validation.",
                "Extrapolation élevée : la zone d'étude contient des conditions absentes des données d'apprentissage ; gardez les hachures visibles ou restreignez la zone d'étude au secteur d'intérêt, pour des raisons écologiques.",
                "Indice de Boyce faible : les prédictions classent mal les occurrences mises de côté ; cherchez des erreurs sur la carte de revue des données et interrogez le choix des variables.",
                "Dans un rapport : citez le niveau et ses motifs tels quels, ne tirez pas de surfaces décisionnelles de la seule carte, et croisez-la avec des données de terrain.",
              ],
            },
            {
              note: "Ne modifiez pas les réglages dans le seul but d'obtenir un niveau supérieur. Toute modification doit avoir une justification écologique ; elle est consignée dans le manifeste.",
              tone: "info",
            },
          ],
        },
      },
    },

    validation: {
      title: "Indicateurs de validation",
      summary:
        "Validation croisée par blocs spatiaux, indice de Boyce continu, OR10, ORmtp et AUC présence-fond.",
      sections: {
        cv: {
          title: "Validation croisée par blocs spatiaux",
          blocks: [
            {
              p: "Par défaut, les présences sont séparées à leur latitude médiane, puis chaque moitié à sa longitude médiane, ce qui donne **4 blocs spatiaux** d'effectifs proches ; les points de fond sont répartis selon les mêmes lignes (Muscarella et al. 2014). Chaque bloc est tour à tour mis de côté et prédit par un modèle entraîné sur les trois autres. Le modèle est ainsi testé sur une région qu'il n'a pas vue, ce qui réduit l'optimisme dû à l'autocorrélation spatiale.",
            },
            {
              p: "La validation aléatoire en k plis est proposée dans les réglages du modèle, mais elle est optimiste quand les occurrences sont regroupées dans l'espace (Roberts et al. 2017). Une étude validée par plis aléatoires ne peut pas atteindre *Supported*.",
            },
            {
              p: "Les indicateurs sont présentés en moyenne ± écart-type entre plis.",
            },
          ],
        },
        grid: {
          title:
            "Blocs spatiaux en grille dimensionnés par l'autocorrélation (Expert)",
          blocks: [
            {
              p: "Une alternative aux 4 blocs médians : la zone d'étude est couverte d'une grille de **blocs carrés**, et des blocs entiers sont répartis en k plis (5 par défaut) pour que chaque pli ait un nombre de présences comparable (blockCV, Valavi et al. 2019). Les présences de test sont ainsi éloignées des présences d'apprentissage, ce qui réduit l'optimisme de la validation en présence d'autocorrélation spatiale (Roberts et al. 2017).",
            },
            {
              p: "**Taille des blocs.** Laissez le champ vide pour le choix automatique : NicheRS ajuste un variogramme à chaque prédicteur sur les points de fond et retient la **portée médiane** — la distance au-delà de laquelle les valeurs ne sont plus corrélées — bornée pour garder assez de blocs pour la validation croisée. Vous pouvez aussi saisir une taille en km. L'onglet Validation indique la taille utilisée, comment elle a été choisie, la portée de chaque prédicteur et le nombre de plis réellement obtenus.",
            },
            {
              p: "Si trop peu de blocs contiennent des présences, le nombre de plis peut être inférieur à celui demandé ; un avertissement le signale. Les deux méthodes spatiales comptent comme validation par blocs spatiaux pour le niveau de preuve.",
            },
          ],
        },
        nulls: {
          title:
            "Modèles nuls : mieux que des modèles sur des localisations aléatoires ? (Expert)",
          blocks: [
            {
              p: "Test facultatif (Raes & ter Steege 2007 ; Bohl et al. 2019 ; ENMeval 2.0, Kass et al. 2021). NicheRS tire N jeux de **localisations aléatoires** dans la zone d'étude, autant que de présences de l'espèce, et ajuste chacun avec exactement le réglage retenu et la même validation croisée. L'indice de Boyce et l'OR10 sur les plis mis de côté du modèle réel sont comparés à ceux des modèles sur localisations aléatoires.",
            },
            {
              p: "**Valeur p** : la part des modèles sur localisations aléatoires qui font au moins aussi bien que le modèle réel, comptée comme (1 + nombre aussi bons) / (N + 1). Un p ≤ 0,05 pour l'indice de Boyce signifie que le modèle est **meilleur que des modèles sur localisations aléatoires**. Avec N = 20, le plus petit p possible vaut 1/21 ≈ 0,048 (il en faut au moins 19 pour que p puisse atteindre 0,05 ; moins n'est pas accepté) ; utilisez 50 à 100 pour un test plus solide.",
            },
            {
              p: "Avec un [fond par groupe cible](#data/targetgroup), les localisations aléatoires sont tirées proportionnellement aux données du groupe cible par cellule (modèles nuls **appariés au biais**) : la comparaison n'est pas gonflée par le biais des observateurs.",
            },
            {
              note: "Un résultat significatif dit que le modèle fait mieux que le hasard, pas qu'il est écologiquement juste. S'il ne fait pas mieux que des localisations aléatoires, le niveau de preuve est plafonné à *Exploratory*. Ce n'est pas un niveau de confiance. Coût : N × plis ajustements supplémentaires (par ex. 100 × 4 = 400), ce qui peut multiplier la durée de l'étude.",
              tone: "warn",
            },
          ],
        },
        boyce: {
          title: "Indice de Boyce continu (CBI)",
          blocks: [
            {
              p: "De −1 à 1 (Hirzel et al. 2006). Il mesure si les présences mises de côté tombent dans des cellules d'idonéité élevée plus souvent qu'attendu d'après le fond. Au-dessus de 0 : mieux que le hasard ; proche de 1 : prédictions cohérentes avec les lieux d'observation de l'espèce ; 0 ou moins : pas mieux que le hasard.",
            },
            {
              p: "Calculé avec les paramètres par défaut d'ecospat (101 fenêtres glissantes). Il n'est pas calculé pour un pli comptant trop peu de présences de test (affiché —).",
            },
          ],
        },
        or10: {
          title: "OR10 — omission au 10e percentile",
          blocks: [
            {
              p: "Part des présences mises de côté situées sous le seuil qui exclut 10 % des présences d'apprentissage. Attendu ≈ 10 % pour un modèle non surajusté (Radosavljevic & Anderson 2014) ; des valeurs nettement supérieures signalent un surajustement ou un mauvais transfert vers de nouveaux secteurs. C'est le premier critère de sélection du modèle.",
            },
          ],
        },
        ormtp: {
          title: "ORmtp — omission au minimum des présences d'apprentissage",
          blocks: [
            {
              p: "Part des présences mises de côté dont l'idonéité est inférieure à la plus faible idonéité d'une présence d'apprentissage. Attendu 0 % ; des valeurs supérieures signifient que des occurrences mises de côté se trouvent dans des conditions que le modèle juge moins favorables que toutes celles d'apprentissage.",
            },
          ],
        },
        auc: {
          title: "AUC présence-fond et ses limites",
          blocks: [
            {
              p: "Indicateur secondaire uniquement. Elle compare les présences aux points de fond, et non à des absences connues : son maximum est inférieur à 1 et elle reflète surtout l'étendue et la diversité environnementale de la zone d'étude (Lobo et al. 2008 ; Jiménez-Valverde 2012). Une zone d'étude plus grande la fait augmenter.",
            },
            {
              note: "Ne comparez l'AUC qu'entre modèles de la même espèce sur la même zone d'étude. Ne la comparez jamais entre espèces ou entre zones, ne l'appelez jamais « précision ». Elle ne sert pas à sélectionner le modèle.",
              tone: "warn",
            },
          ],
        },
        selection: {
          title: "Calibration et sélection du modèle",
          blocks: [
            {
              p: "En calibration automatique, NicheRS évalue **25 réglages** : classes de variables L, LQ, H, LQH, LQHP (L linéaire, Q quadratique, H charnière, P produit) × multiplicateur de régularisation β 0,5, 1, 2, 3, 4 (un β plus élevé donne des réponses plus lisses). Il retient, dans l'ordre : l'OR10 moyen le plus faible, puis l'indice de Boyce moyen le plus élevé, puis le moins de coefficients (Radosavljevic & Anderson 2014 ; Kass et al. 2021). Le modèle final est réajusté sur toutes les présences et tous les points de fond avec le réglage retenu.",
            },
            {
              p: "Cette règle privilégie des modèles plus lisses et mieux transférables plutôt qu'une discrimination apparente. Le tableau complet se trouve sous Validation › Calibration.",
            },
          ],
        },
      },
    },

    area: {
      title: "Fourchette de surface favorable",
      summary:
        "Seuils MTP, P10 et max SSS, fourchette de surfaces et surfaces exactes des cellules WGS84.",
      sections: {
        band: {
          title: "Surface de conditions favorables",
          blocks: [
            {
              p: "La **surface de conditions favorables** affichée est la somme des surfaces des cellules dont l'idonéité atteint ou dépasse un seuil. Le seuil étant un choix, NicheRS affiche toujours la **fourchette** (minimum – maximum) obtenue avec trois règles à côté de la valeur principale, ainsi que la part de la zone d'étude. Changer la règle dans l'onglet Synthèse met aussi à jour la couche « Favorable / non favorable au seuil ».",
            },
          ],
        },
        rules: {
          title: "Règles de seuil",
          blocks: [
            {
              table: {
                caption: "Règles de seuil",
                head: ["Règle", "Définition", "Comportement"],
                rows: [
                  [
                    "**P10** — 10e percentile des présences d'apprentissage (défaut)",
                    "Idonéité sous laquelle se trouvent 10 % des présences d'apprentissage",
                    "Tolère les ~10 % d'occurrences erronées ou en habitat marginal des données opportunistes, tout en restant prudent.",
                  ],
                  [
                    "**MTP** — minimum des présences d'apprentissage",
                    "Plus faible idonéité d'une présence d'apprentissage",
                    "Le plus inclusif ; une seule occurrence aberrante peut beaucoup agrandir la surface.",
                  ],
                  [
                    "**Max SSS** — maximum sensibilité + spécificité",
                    "Maximise sensibilité + spécificité, présences contre fond (Liu et al. 2013)",
                    "Utilise les points de fond comme pseudo-absences.",
                  ],
                ],
              },
            },
            {
              p: "Les seuils sont calculés à partir des prédictions du modèle final aux cellules de présence d'apprentissage (et aux points de fond pour max SSS).",
            },
          ],
        },
        cells: {
          title: "Surfaces exactes des cellules WGS84",
          blocks: [
            {
              p: "Le modèle travaille sur la grille géographique native de CHELSA (30 secondes d'arc, environ 0,93 × 0,6 km à 50° N), sans reprojection ni rééchantillonnage. La surface de chaque cellule est calculée exactement comme un quadrilatère sur l'ellipsoïde WGS84 (Snyder 1987), puis les surfaces sont additionnées. Les cellules rétrécissent vers les pôles : un nombre de cellules n'est pas une surface.",
            },
            {
              p: "La surface de la zone d'étude est la surface terrestre disposant de données de prédicteurs : la mer et les grands lacs sont exclus.",
            },
          ],
        },
        wording: {
          title: "Comment le formuler",
          blocks: [
            {
              note: "Écrivez : « surface de conditions favorables (règle P10) : X km², fourchette Y–Z km² selon les règles de seuil ». Jamais « aire de répartition », « surface d'habitat » ni « surface occupée ».",
              tone: "info",
            },
          ],
        },
      },
    },

    uncertainty: {
      title: "Incertitude et extrapolation",
      summary:
        "Dispersion entre modèles des plis (écart-type) et nouveauté environnementale (MESS).",
      sections: {
        foldsd: {
          title: "Incertitude : dispersion entre modèles des plis",
          blocks: [
            {
              p: "Pour chaque cellule, la couche **Incertitude** montre l'écart-type de l'idonéité entre les modèles de validation croisée du réglage retenu (4 modèles avec les blocs spatiaux). Elle mesure à quel point la prédiction dépend des occurrences utilisées pour l'apprentissage. La légende va de 0 à ≥ 0,25 ; l'onglet Synthèse en donne la moyenne sur la zone d'étude.",
            },
            {
              p: "Ce n'est pas un intervalle de confiance et elle n'intègre pas toutes les sources d'erreur (erreurs d'occurrence, variables manquantes, biais d'échantillonnage).",
            },
          ],
        },
        mess: {
          title: "Extrapolation (MESS)",
          blocks: [
            {
              p: "La couche **Extrapolation** est la surface de similarité environnementale multivariée (MESS, Elith et al. 2010), calculée par rapport aux environnements des cellules de fond et de présence. MESS < 0 signifie qu'au moins une variable sort de la gamme rencontrée à l'apprentissage : la prédiction y est une extrapolation, guidée par les hypothèses du modèle plutôt que par les données.",
            },
            {
              p: "Ces cellules sont **hachurées par défaut sur la carte** ; gardez les hachures sur les cartes publiées. La part de la zone d'étude où MESS < 0 entre dans le niveau de preuve (≤ 25 % pour *Indicative*, ≤ 10 % pour *Supported*), et un avertissement s'affiche au-delà de 10 %.",
            },
          ],
        },
        aoa: {
          title: "Domaine d'applicabilité (AOA, Expert)",
          blocks: [
            {
              p: "Couche facultative (Meyer & Pebesma 2021, package R CAST). Pour chaque cellule, l'**indice de dissimilarité (DI)** est la distance, dans l'espace des prédicteurs, au point d'apprentissage le plus proche (présence ou fond), divisée par la distance moyenne entre points d'apprentissage. Les prédicteurs sont standardisés et pondérés par leur importance par permutation dans le modèle final : les écarts sur les variables importantes comptent davantage.",
            },
            {
              p: "Le **seuil** est déduit des données d'apprentissage avec les mêmes plis de validation croisée (le plus grand DI des points d'apprentissage qui n'est pas une valeur aberrante). Les cellules au-dessus sont **hors du domaine d'applicabilité** et hachurées en violet quand la couche est activée. Un avertissement s'affiche si plus de 10 % de la zone d'étude est hors du domaine.",
            },
            {
              p: "**Différence avec MESS.** MESS signale les valeurs hors de la gamme de chaque variable prise séparément ; l'AOA signale aussi des *combinaisons* nouvelles de valeurs à l'intérieur de ces gammes, et pondère les variables par leur importance. Les deux peuvent diverger : lisez les deux.",
            },
            {
              note: "L'AOA ne modifie pas le niveau de preuve : MESS reste le critère d'extrapolation (ADR-0009). Être dans le domaine d'applicabilité ne garantit pas que la prédiction est juste.",
              tone: "info",
            },
          ],
        },
        together: {
          title: "Les lire ensemble",
          blocks: [
            {
              ul: [
                "Idonéité élevée, faible dispersion, pas de hachures : la partie la plus robuste de la carte.",
                "Forte dispersion : la prédiction change selon le sous-ensemble d'apprentissage ; prudence.",
                "Hachures : hors des conditions d'apprentissage ; n'interprétez pas le score à cet endroit, quelle que soit sa valeur.",
                "Aucune des deux couches ne montre le biais d'échantillonnage : avec des données opportunistes, les secteurs bien prospectés paraissent plus favorables (pas de correction de biais par défaut ; voir [Biais d'échantillonnage](#data/bias)).",
              ],
            },
          ],
        },
      },
    },

    data: {
      title: "Nettoyage des données et motifs d'exclusion",
      summary:
        "Occurrences GBIF, règles de nettoyage dans l'ordre, licences et citation du téléchargement GBIF.",
      sections: {
        source: {
          title: "Origine des occurrences",
          blocks: [
            {
              p: "Les occurrences proviennent du GBIF (gbif.org), via le référentiel taxonomique GBIF. Lorsque le serveur dispose d'un compte GBIF (le service hébergé utilise celui de Natural Solutions), elles proviennent d'un **téléchargement GBIF avec DOI**.",
            },
            {
              p: "Sinon, l'API de recherche GBIF est utilisée, plafonnée par défaut à 20 000 occurrences par espèce ; au-delà, un sous-échantillon spatialement stratifié est tiré et la revue des données le signale. Pour un rapport final, utilisez une étude avec téléchargement DOI.",
            },
            {
              p: "Fond : 10 000 cellules terrestres tirées au hasard dans la zone d'étude, avec la graine de l'étude. Aucune correction du biais d'échantillonnage n'est appliquée par défaut ; le mode Expert propose un [fond par groupe cible](#data/targetgroup).",
            },
          ],
        },
        rules: {
          title: "Règles de nettoyage, dans l'ordre",
          blocks: [
            {
              table: {
                caption: "Règles de nettoyage des occurrences",
                head: ["#", "Motif d'exclusion affiché", "Défaut", "Pourquoi"],
                rows: [
                  [
                    "1",
                    "Spécimen fossile ou captif/cultivé",
                    "Exclu",
                    "Pas des populations sauvages dans leur niche environnementale.",
                  ],
                  [
                    "2",
                    "Licence non autorisée pour ce projet",
                    "CC0 et CC BY 4.0 uniquement",
                    "CC BY-NC est non commerciale (voir plus bas).",
                  ],
                  [
                    "3",
                    "Occurrence non datée",
                    "Exclu",
                    "Impossible à rattacher à la période climatique de référence.",
                  ],
                  [
                    "4",
                    "Observée avant 1981 (période de référence climatique)",
                    "Année ≥ 1981",
                    "La climatologie CHELSA couvre 1981–2010.",
                  ],
                  [
                    "5",
                    "Coordonnées généralisées (floutées) par le fournisseur ; précision non indiquée ou > 1000 m",
                    "Exclu ; les occurrences généralisées de précision indiquée ≤ 1000 m sont conservées",
                    "Position floutée de précision inconnue. Voir [Coordonnées généralisées](#data/generalised).",
                  ],
                  [
                    "6",
                    "Incertitude des coordonnées > 1000 m",
                    "≤ 1000 m ; les occurrences sans cette information sont conservées",
                    "Plus grande que la cellule d'environ 1 km.",
                  ],
                  ["7", "Hors de la zone d'étude", "Exclu", "—"],
                  [
                    "8",
                    "En mer ou dans un grand lac (masque terrestre)",
                    "Exclu",
                    "Niche terrestre ; les données climatiques existent aussi en mer.",
                  ],
                  [
                    "9",
                    "Données environnementales manquantes",
                    "Exclu",
                    "Pas de valeur de prédicteur dans la cellule.",
                  ],
                  [
                    "10",
                    "Doublon dans une même cellule de la grille",
                    "Une occurrence par cellule",
                    "Réduit la pseudo-réplication.",
                  ],
                  [
                    "11",
                    "Filtrage spatial (à moins de X km d'une occurrence retenue)",
                    "Désactivé (option Expert)",
                    "Réduit les agrégats. Voir [Filtrage spatial par distance](#data/thinning).",
                  ],
                ],
              },
            },
            {
              p: "Chaque occurrence exclue est comptée sous la **première** règle qu'elle ne satisfait pas : les décomptes s'additionnent. Les étapes de nettoyage ne peuvent pas être désactivées ; les filtres d'année, d'incertitude et de licence se règlent à l'étape Données et sont consignés dans le rapport et le manifeste.",
            },
            {
              p: "Le masque terrestre (Natural Earth 1:10m) a une précision du trait de côte d'environ 0,5–1 km : certaines occurrences côtières peuvent être exclues à tort. Elles sont listées avec leur motif ; vérifiez-les pour les espèces littorales. Les espèces marines sont hors du champ.",
            },
          ],
        },
        generalised: {
          title: "Coordonnées généralisées (floutées)",
          blocks: [
            {
              p: "**Les fournisseurs floutent les coordonnées des espèces sensibles. NicheRS n'utilise pas les points floutés dont la précision est inconnue.**",
            },
            {
              p: "Certains fournisseurs de données *généralisent* volontairement les coordonnées — arrondies à une maille de 10 km ou ramenées au centroïde de la commune, par exemple — pour protéger des espèces sensibles (rapaces nicheurs, orchidées, gîtes de chauves-souris) ou à la demande de l'observateur. GBIF l'indique dans les champs Darwin Core `dataGeneralizations` et `informationWithheld`.",
            },
            {
              p: "Une occurrence généralisée n'est conservée que si l'incertitude de ses coordonnées est indiquée et dans la limite d'incertitude (1000 m par défaut). Si la précision n'est pas indiquée, ou est plus grande, elle est exclue avec le motif **Coordonnées généralisées par le fournisseur** (règle 5, juste avant la règle d'incertitude) et affichée sur la carte par un cercle magenta en tirets. L'onglet Données et exports donne les décomptes : généralisées, exclues, retenues.",
            },
            {
              p: "En France, le SINP (Système d'information de l'inventaire du patrimoine naturel) définit, région par région, les espèces dont la localisation est sensible (référentiels régionaux de sensibilité) ; leurs données sont souvent diffusées à une résolution grossière. Les présences retenues peuvent alors sous-représenter les sites les plus sensibles. L'avertissement `generalised_records` donne les nombres ; il devient un Avertissement quand au moins 10 % des occurrences de la zone sont généralisées.",
            },
            {
              note: "L'API de recherche GBIF indique la généralisation ; les téléchargements GBIF (format SIMPLE_CSV) et le miroir GBIF régional ne l'indiquent pas. Les études qui les utilisent affichent la remarque *indicateurs de généralisation indisponibles* : les occurrences généralisées dont l'incertitude est faible ou absente ne peuvent pas être détectées ; vérifiez vous-même les espèces sensibles dans le référentiel régional.",
              tone: "warn",
            },
            {
              p: "`occurrences.geojson` conserve les coordonnées telles que publiées par GBIF. Certaines espèces sont sensibles au sens des règles du SINP : vérifiez le référentiel régional avant de reproduire des points dans un rapport public.",
            },
          ],
        },
        thinning: {
          title: "Filtrage spatial par distance (Expert)",
          blocks: [
            {
              p: "Facultatif, désactivé par défaut, réglé à l'étape Modèle (mode expert). Après le nettoyage, les cellules de présence retenues sont filtrées pour qu'aucune paire de cellules restantes ne soit plus proche que la distance choisie (plus de 0 et au plus 100 km). Cela réduit les agrégats d'occurrences issus des lieux très prospectés (villes, réserves, sites connus), qui pèseraient sinon fortement dans le modèle, ainsi que l'autocorrélation spatiale entre présences.",
            },
            {
              p: "La méthode suit spThin (Aiello-Lammens et al. 2015) : les cellules sont retirées dans un ordre aléatoire, le tirage est répété plusieurs fois et le plus grand ensemble de cellules est conservé. Les tirages proviennent de la graine de l'étude : l'aperçu des données, l'exécution et toute relance de la même étude donnent le même résultat. Les cellules retirées sont comptées sous **Filtrage spatial (à moins de X km d'une occurrence retenue)**, après la règle du doublon par cellule, et affichées sur la carte par de petits points gris.",
            },
            {
              note: "Le filtrage coûte des données : chaque cellule retirée est une vraie présence. Avec peu de présences, il peut faire passer une espèce vers *Exploratory* ou sous le minimum. Choisissez une distance adaptée aux agrégats à retirer (par exemple quelques km autour des points chauds d'observation), pas plus grande que nécessaire.",
              tone: "warn",
            },
            {
              p: "La distance est consignée dans la spécification (`filters.thin_km`), le rapport et le manifeste.",
            },
          ],
        },
        bias: {
          title:
            "Biais d'échantillonnage : les données montrent où l'on regarde",
          blocks: [
            {
              p: "Les données GBIF sont surtout opportunistes : elles viennent des lieux où l'on va — près des villes, des routes, des sentiers, des réserves et des sites connus — autant que des lieux où vit l'espèce. Un modèle ajusté sur elles apprend l'habitat **et** les habitudes des observateurs : les conditions bien prospectées paraissent plus favorables qu'elles ne le sont, les secteurs reculés moins.",
            },
            {
              p: "Par défaut, NicheRS ne corrige pas ce biais et le signale par l'avertissement `sampling_bias`. Lisez la carte en conséquence : un score faible dans un secteur peu prospecté peut seulement signifier que personne n'y a cherché.",
            },
            {
              ul: [
                "**Fond par groupe cible** (Expert) : tire les points de fond là où des organismes similaires ont été notés, pour que présences et fond partagent le biais. Voir [Fond par groupe cible](#data/targetgroup).",
                "**Filtrage spatial par distance** (Expert) : retire les présences agrégées. Voir [Filtrage spatial par distance](#data/thinning).",
                "La validation croisée par blocs spatiaux ne supprime pas le biais : les plis de test viennent des mêmes données biaisées.",
              ],
            },
          ],
        },
        targetgroup: {
          title: "Fond par groupe cible (Expert)",
          blocks: [
            {
              p: "Au lieu de cellules aléatoires, les points de fond sont tirés parmi les cellules de la zone d'étude qui ont au moins une donnée GBIF d'un **groupe cible** — des organismes prospectés de la même façon que l'espèce, par défaut sa propre classe GBIF (Aves pour un oiseau, Insecta pour un coléoptère, Magnoliopsida pour une plante à fleurs). Présences et fond partagent alors le biais des observateurs, qui se compense en grande partie dans le modèle (Phillips et al. 2009). Les cellules sont tirées uniformément, une par cellule, avec la graine de l'étude.",
            },
            {
              p: "Il nécessite les **grilles d'effort d'échantillonnage du miroir GBIF régional** (données de chaque classe par cellule). Si aucun miroir ne couvre la zone, si le miroir n'a pas encore de grilles, si la classe n'a aucune donnée ou si trop peu de cellules en ont, le fond revient à un tirage aléatoire et l'avertissement `target_group_unavailable` en donne le motif : aucune correction de biais n'a été appliquée. Si moins de cellules que demandé ont des données, le fond est plus petit (remarque `target_group_background_smaller`).",
            },
            {
              p: "**Carte d'effort.** Ces études ont une couche *Effort d'échantillonnage* : données du groupe cible par cellule, en orange sur une échelle logarithmique (1 donnée → le maximum dans une cellule ; les cellules sans donnée sont transparentes). Les décomptes sont des données, pas des visites ni des observateurs. Dans la démonstration, la surface d'effort est synthétique et signalée comme telle.",
            },
            {
              p: "**Diagnostic.** L'onglet Données et exports indique la part des cellules de présence situées dans les cellules les plus prospectées couvrant ≈ 10 % de la zone d'étude. Sans biais, environ 10 % seraient attendus ; une part bien plus grande signifie que les présences suivent l'effort d'observation. C'est descriptif seulement et ne modifie pas le niveau de preuve.",
            },
            {
              p: "**Modèles nuls appariés au biais.** Si des modèles nuls sont demandés, leurs localisations aléatoires sont tirées proportionnellement aux données du groupe cible par cellule plutôt qu'uniformément : le modèle est comparé à des modèles sur localisations aléatoires qui portent le même biais.",
            },
            {
              note: "Seule la part du biais commune au groupe cible et à l'espèce est corrigée. Les espèces prospectées autrement que leur classe (un oiseau rare recherché par des spécialistes, une espèce notée surtout par un protocole) gardent leur propre biais, et un groupe cible avec très peu de données donne un fond médiocre.",
              tone: "warn",
            },
          ],
        },
        licences: {
          title: "Licences : pourquoi CC BY-NC est exclue",
          blocks: [
            {
              p: "Par défaut, seules les occurrences **CC0** et **CC BY 4.0** sont utilisées. Les occurrences **CC BY-NC** sont exclues car elles ne peuvent pas servir dans des livrables commerciaux de bureau d'études sans lecture juridique de la licence (ADR-0006).",
            },
            {
              note: "Vous pouvez inclure les occurrences CC BY-NC, mais les résultats sont alors réservés à un usage non commercial : ne les intégrez pas à un livrable facturé.",
              tone: "warn",
            },
            {
              p: "Les occurrences CC BY exigent une attribution : citez le téléchargement GBIF (ci-dessous).",
            },
          ],
        },
        citation: {
          title: "Citer le téléchargement GBIF (DOI)",
          blocks: [
            {
              p: "Le DOI figure dans l'onglet Données et exports (Source), dans `report.md` et dans le manifeste (`occurrence_source.citation`). Citez-le sous la forme recommandée par le GBIF : *GBIF.org (date) GBIF Occurrence Download https://doi.org/10.15468/dl.xxxxxx*.",
            },
            {
              p: "Un téléchargement avec DOI est la trace permanente et citable des occurrences exactement utilisées. Une étude via l'API de recherche n'a pas de DOI.",
            },
          ],
        },
        review: {
          title: "Que vérifier à l'étape Données",
          blocks: [
            {
              ul: [
                "Des occurrences à des endroits invraisemblables : jardins, musées, coordonnées inversées, centroïdes de pays ou de région.",
                "Une forte proportion exclue par une seule règle mérite un examen.",
                "Des regroupements autour des villes, des réserves ou des routes : biais d'échantillonnage, non corrigé par défaut (voir [Biais d'échantillonnage](#data/bias)).",
                "Les occurrences généralisées ou filtrées (cercles magenta en tirets, points gris) et leurs décomptes dans le tableau des exclusions.",
                "Les avertissements : taxon de rang supérieur à l'espèce, téléchargement tronqué ou sous-échantillonné.",
                "La résolution de la grille : ×1 correspond à environ 1 km ; une grille plus grossière donne moins de cellules de présence à cause de la règle d'une occurrence par cellule.",
              ],
            },
          ],
        },
      },
    },

    variables: {
      title: "Variables et colinéarité",
      summary:
        "Climat CHELSA v2.1 1981–2010, jeu par défaut et filtre |r| > 0,7.",
      sections: {
        chelsa: {
          title: "Climat CHELSA v2.1",
          blocks: [
            {
              p: "Les prédicteurs sont les 19 variables bioclimatiques **CHELSA v2.1**, climatologie 1981–2010, à 30 secondes d'arc (environ 1 km), licence CC0 (Karger et al. 2017).",
            },
            {
              note: "Elles décrivent **uniquement le climat**. Relief, occupation du sol et distance à l'eau peuvent être ajoutés mais ne sont pas sélectionnés par défaut ([Relief, occupation du sol et eau](#predictors)) ; les sols ne sont pas proposés. Des cellules au climat favorable peuvent ne comporter aucun habitat favorable. Les projections sous climat futur sont une option Expert ([Projections sous climat futur](#future)).",
              tone: "warn",
            },
            {
              p: "WorldClim n'est pas utilisé car sa licence n'autorise pas l'usage commercial.",
            },
          ],
        },
        defaults: {
          title: "Jeu de variables par défaut",
          blocks: [
            {
              p: "Présélectionnées : BIO1 (température moyenne annuelle), BIO4 (saisonnalité des températures), BIO5 (température maximale du mois le plus chaud), BIO6 (température minimale du mois le plus froid), BIO12 (précipitations annuelles), BIO14 (précipitations du mois le plus sec), BIO15 (saisonnalité des précipitations) : énergie, eau, leur saisonnalité et leurs extrêmes.",
            },
            {
              p: "BIO8, BIO9, BIO18 et BIO19 sont signalées et non sélectionnées par défaut : elles combinent température et précipitations de trimestres particuliers et présentent des artefacts spatiaux (Booth 2022).",
            },
          ],
        },
        collinearity: {
          title: "Filtre de colinéarité",
          blocks: [
            {
              p: "Des prédicteurs corrélés rendent instables les formes de réponse et les contributions. Avant l'ajustement, NicheRS calcule les corrélations de Pearson sur l'échantillon de fond et conserve les variables **dans votre ordre de priorité**, en écartant toute variable dont |r| > 0,7 avec une variable déjà conservée (Dormann et al. 2013). Les variables écartées et la variable avec laquelle elles sont corrélées sont listées dans les résultats et le rapport. Le seuil se règle dans les réglages du modèle.",
            },
          ],
        },
        contributions: {
          title: "Contributions des variables",
          blocks: [
            {
              p: "La contribution en pourcentage est une estimation heuristique de MaxEnt de la part de chaque variable dans l'ajustement. Elle dépend du chemin d'optimisation et des variables corrélées ; ce n'est pas une mesure d'importance causale.",
            },
            {
              p: "L'importance par permutation (seconde barre) est l'autre estimation de MaxEnt : les valeurs de chaque variable sont permutées au hasard entre présences et points de fond, et la baisse de l'AUC d'entraînement est ramenée à 100. Elle ne dépend que du modèle final, pas du chemin d'optimisation. Quand les deux estimations divergent, le rôle de la variable est incertain ; toutes deux partagent le crédit entre variables corrélées.",
            },
            {
              p: "Les courbes de réponse montrent l'idonéité relative (cloglog) quand une variable varie, toutes les autres étant fixées à leur moyenne sur les présences. Les parties grisées sont hors des données d'entraînement : le modèle y est borné (clamping) et la courbe est une extrapolation. Ce sont des courbes marginales ; avec des variables corrélées, elles peuvent masquer l'effet de variables qui varient ensemble. Les courbes complètes (501 points, comme `maxent.jar`) sont dans `response_curves.csv`.",
            },
          ],
        },
      },
    },

    predictors: {
      title: "Relief, occupation du sol et eau",
      summary:
        "Prédicteurs optionnels issus du Copernicus DEM et d'ESA WorldCover 2021 : ce qu'ils mesurent, quand les utiliser, leurs limites et leurs sources.",
      sections: {
        overview: {
          title: "Ce qu'ils apportent",
          blocks: [
            {
              p: "Le climat explique la répartition générale d'une espèce ; sur une grille de 1 km, beaucoup d'espèces répondent aussi au relief et à la composition du paysage. En plus des 19 variables climatiques CHELSA, NicheRS propose 12 prédicteurs optionnels : trois variables de relief, huit proportions d'occupation du sol et la distance à l'eau permanente. Ils sont sur la même grille de 30″ que CHELSA.",
            },
            {
              note: "Aucun n'est sélectionné par défaut : la sélection recommandée reste uniquement climatique (ADR-0009). Ces prédicteurs sont proposés par l'ADR-0015, en attente de la revue des écologues de Natural Solutions.",
              tone: "info",
            },
            {
              p: "Ils suivent le même traitement que le climat : le filtre de colinéarité garde les variables **dans l'ordre où vous les avez sélectionnées** (sélectionnez d'abord le climat, pour qu'une variable d'occupation du sol ne soit gardée que si elle apporte une information), puis MESS, courbes de réponse et contributions.",
            },
            {
              p: "Choisissez quelques prédicteurs utiles à l'écologie de l'espèce. Chaque variable ajoutée ne peut qu'augmenter la part de la zone hors des conditions d'apprentissage (MESS prend le minimum sur les variables), ce qui peut abaisser le niveau de preuve. C'est honnête, pas un défaut.",
            },
          ],
        },
        terrain: {
          title: "Relief (Copernicus DEM)",
          blocks: [
            {
              table: {
                caption: "Prédicteurs de relief",
                head: ["Variable", "Unité", "Valeur par cellule de 1 km"],
                rows: [
                  [
                    "Altitude (ELEV)",
                    "m",
                    "Moyenne des altitudes à 90 m dans la cellule.",
                  ],
                  [
                    "Pente (SLOPE)",
                    "degrés",
                    "Moyenne des pentes calculées à 90 m (Horn 1981) **avant** agrégation, pour ne pas aplanir le relief.",
                  ],
                  [
                    "Rugosité (ROUGH)",
                    "m",
                    "Écart-type des altitudes à 90 m dans la cellule.",
                  ],
                ],
              },
            },
            {
              ul: [
                "**Quand les utiliser** : pente et rugosité pour les espèces liées aux terrains pentus ou rocheux, ou qui les évitent (oiseaux rupestres, reptiles des rochers, ongulés de montagne).",
                "L'altitude répète en grande partie la température (BIO1) : elle est souvent écartée par le filtre de colinéarité et apporte rarement une information.",
              ],
            },
            {
              note: "Le Copernicus DEM est un modèle **de surface** (TanDEM-X, 2011–2015) : les altitudes incluent la canopée et les bâtiments, si bien que les cellules boisées et urbaines paraissent plus hautes et plus rugueuses que le sol.",
              tone: "warn",
            },
          ],
        },
        landcover: {
          title: "Occupation du sol (ESA WorldCover 2021)",
          blocks: [
            {
              p: "Chaque variable d'occupation du sol est la **proportion** (de 0 à 1) des pixels de 10 m de la cellule qui appartiennent à un groupe de classes WorldCover ; une cellule de 1 km contient plusieurs milliers de pixels, si bien que les mosaïques et les lisières sont conservées au lieu d'une seule classe dominante.",
            },
            {
              table: {
                caption: "Groupes d'occupation du sol",
                head: ["Variable", "Classes WorldCover"],
                rows: [
                  ["Couvert arboré (TREE)", "10 couvert arboré"],
                  ["Arbustes (SHRUB)", "20 arbustes"],
                  ["Prairies et herbacées (GRASS)", "30 herbacées"],
                  ["Cultures (CROP)", "40 cultures"],
                  ["Bâti (BUILT)", "50 bâti"],
                  [
                    "Sol nu et végétation clairsemée (BARE)",
                    "60 sol nu/végétation clairsemée, 70 neige et glace, 100 mousses et lichens",
                  ],
                  ["Eau permanente (WATER)", "80 plans d'eau permanents"],
                  [
                    "Zones humides (WETL)",
                    "90 zones humides herbacées, 95 mangroves",
                  ],
                ],
              },
            },
            {
              note: "Les huit proportions font 1 dans chaque cellule. Choisissez les groupes pertinents pour l'espèce (par exemple le couvert arboré pour une espèce forestière, prairies et cultures pour un oiseau des milieux agricoles), pas les huit : avec les huit, l'un est redondant et le filtre de corrélation par paires ne le détecte pas. Les résultats portent alors un avertissement.",
              tone: "warn",
            },
            {
              ul: [
                "**Décalage temporel** : l'occupation du sol est celle de 2021, la normale climatique 1981–2010 et les données peuvent dater de 1981. Là où l'usage du sol a changé, les données anciennes sont décrites par l'occupation actuelle.",
                "**Bâti et biais d'échantillonnage** : le bâti est fortement corrélé aux lieux d'observation. Une réponse positive à BUILT peut refléter les observateurs, pas l'espèce ; les résultats le signalent quand il est utilisé.",
                "WorldCover ne distingue ni feuillus et résineux, ni cultures annuelles et permanentes (vignes, vergers) ; sa précision globale est d'environ 77 %.",
                "Une classe à 0 % dans toutes les cellules d'apprentissage est signalée par MESS comme hors domaine même quand ce n'est pas remarquable écologiquement ; lisez la carte MESS en conséquence.",
              ],
            },
          ],
        },
        water: {
          title: "Distance à l'eau",
          blocks: [
            {
              p: "**Distance à l'eau (DWAT, km)** : distance du centre de la cellule à la cellule la plus proche couverte au moins pour moitié d'eau permanente dans WorldCover 2021, **plafonnée à 25 km**. Les valeurs jusqu'à 25 km sont exactes et ne dépendent pas de l'étendue de la zone d'étude ; les cellules plus lointaines valent 25 km.",
            },
            {
              ul: [
                "**Quand l'utiliser** : espèces liées aux lacs, retenues, grands cours d'eau ou au littoral (oiseaux d'eau, certains amphibiens et odonates).",
                "Seule l'eau couvrant au moins la moitié d'une cellule de 1 km est vue : lacs, retenues, larges rivières, lagunes et mer. Les ruisseaux et les mares ne sont pas résolus : elle ne convient pas aux espèces dépendant de petits cours d'eau.",
                "Même décalage temporel que l'occupation du sol (2021).",
              ],
            },
          ],
        },
        aggregation: {
          title: "Comment les valeurs sont calculées",
          blocks: [
            {
              ul: [
                "Rien n'est rééchantillonné : chaque pixel source est affecté à la cellule CHELSA qui contient son centre, si bien que toutes les couches se superposent exactement.",
                "Une cellule est sans donnée quand moins de la moitié de ses pixels sources sont valides. Sur les grilles plus grossières (×2, ×3…), les valeurs à 1 km sont moyennées, comme le climat.",
                "Une cellule n'est modélisée que là où **chaque** prédicteur sélectionné a une valeur. Le long des côtes, le DEM peut manquer là où des tuiles entières de mer sont absentes ; les données y sont exclues comme « Données environnementales manquantes ».",
                "Une première étude dans une nouvelle région télécharge et agrège les tuiles sources (mesuré : environ 4–5 s et 8 Mo par degré pour WorldCover, environ 1 s par degré pour le DEM) ; les études suivantes lisent le cache de l'instance en quelques millisecondes.",
              ],
            },
          ],
        },
        sources: {
          title: "Sources, licences et attribution",
          blocks: [
            {
              table: {
                caption: "Sources des prédicteurs supplémentaires",
                head: [
                  "Source",
                  "Licence",
                  "Attribution (reproduite dans les rapports)",
                ],
                rows: [
                  [
                    "Copernicus DEM GLO-90 (ESA, doi:10.5270/ESA-c5d3d65)",
                    "Licence Copernicus WorldDEM-90 : gratuite, mondiale ; reproduction, distribution et adaptation autorisées",
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
              p: "Les deux autorisent l'usage commercial avec attribution (vérifié le 2026-10-20 ; la licence du DEM accorde les droits sans nommer l'usage commercial, une confirmation juridique est conseillée). Les rapports et le manifeste listent chaque source utilisée avec sa licence et son attribution ; conservez l'attribution quand vous réutilisez une carte.",
            },
          ],
        },
      },
    },

    algorithms: {
      title: "Algorithmes de modélisation",
      summary:
        "MaxEnt (par défaut), la forêt aléatoire sous-échantillonnée et l'ensemble MaxEnt + forêt aléatoire : ce qu'ils sont, quand les utiliser et comment lire leurs résultats.",
      sections: {
        overview: {
          title: "MaxEnt reste le choix par défaut",
          blocks: [
            {
              p: "Toute étude utilise **MaxEnt**, sauf choix contraire en mode expert (étape du modèle, *Algorithme*). MaxEnt est le choix par défaut parce que son portage dans NicheRS est testé contre `maxent.jar` 3.4.4 sur des cas de référence, que son comportement avec des données présence–fond est bien documenté et que ses courbes de réponse lisses se discutent facilement avec un client (ADR-0002, ADR-0009).",
            },
            {
              p: "Deux alternatives sont proposées sur demande : une **forêt aléatoire sous-échantillonnée** et un **ensemble** de MaxEnt et de cette forêt. Les deux relèvent de l'ADR-0017, en attente de revue par les écologues de Natural Solutions.",
            },
            {
              note: "Tous les algorithmes produisent une **idonéité relative de l'habitat**, jamais une probabilité de présence : des données présence–fond ne permettent pas d'estimer la fréquence de l'espèce.",
              tone: "warn",
            },
          ],
        },
        rf: {
          title: "Forêt aléatoire sous-échantillonnée",
          blocks: [
            {
              p: "Une forêt aléatoire (Breiman 2001) est un ensemble d'arbres de classification, chacun construit sur un échantillon aléatoire des données, qui votent « présence » ou « fond ». Avec des données présence–fond, une forêt par défaut échoue : les points de fond sont bien plus nombreux que les présences, si bien que la plupart des arbres votent « fond » presque partout.",
            },
            {
              p: "La forêt **sous-échantillonnée** corrige ce défaut (Valavi et al. 2021) : chaque arbre est construit sur un tirage avec remise des présences et d'**autant** de points de fond. Valavi et al. 2022 la classent dans le groupe de tête des méthodes pour données de présence seule, sur 225 espèces avec des données de test indépendantes, alors que la forêt par défaut figurait parmi les moins bonnes.",
            },
            {
              ul: [
                "Réglages : 500 arbres par défaut (100–2 000), développés entièrement, `mtry` = √p variables essayées à chaque division (en option aussi p/2, choisi par la même règle de validation que MaxEnt).",
                "Sortie : la **part des arbres qui votent « présence »**, de 0 à 1. C'est un indice d'idonéité relative : l'équilibre 1:1 de chaque arbre fixe sa prévalence apparente.",
                "Les seuils utilisent les scores **hors sac** (out-of-bag) des présences d'apprentissage (chaque présence notée seulement par les arbres qui ne l'ont pas vue), car des arbres entièrement développés notent leurs propres présences d'apprentissage presque à 1.",
                "Mêmes plis, métriques, règle de sélection, modèles nuls, MESS, domaine d'applicabilité et règles de niveau de preuve que MaxEnt : les deux moteurs sont jugés de la même façon.",
              ],
            },
          ],
        },
        ensemble: {
          title: "Ensemble de MaxEnt et de la forêt aléatoire",
          blocks: [
            {
              p: "L'ensemble calibre chaque membre comme s'il était seul, puis en fait la moyenne. Comme un score cloglog et une part de votes ne sont pas sur la même échelle, la valeur de chaque membre est d'abord convertie en **rang centile** parmi ses propres valeurs aux points de fond ; l'ensemble est la moyenne des deux rangs.",
            },
            {
              p: "Une valeur d'ensemble de 0,8 signifie donc qu'en moyenne les deux modèles classent la cellule au-dessus de 80 % des points de fond de la zone d'étude. Ce sont des rangs propres à cette zone : ne les comparez pas d'une étude à l'autre.",
            },
            {
              p: "L'ensemble est évalué comme un modèle à part entière : dans chaque pli, les modèles de pli des deux membres sont combinés et testés sur les données mises de côté. Ses métriques déterminent le niveau de preuve ; l'onglet Validation indique aussi les métriques de chaque membre sur les mêmes plis. Les seuils sont calculés sur les valeurs de l'ensemble avec les mêmes règles (P10, maximum sensibilité + spécificité, minimum des présences d'apprentissage).",
            },
          ],
        },
        choose: {
          title: "Lequel choisir",
          blocks: [
            {
              ul: [
                "**MaxEnt** pour une étude courante : documenté, reproductible par rapport au logiciel de référence, courbes lisses.",
                "**L'ensemble** pour vérifier que les conclusions ne dépendent pas d'un seul algorithme. Valavi et al. 2022 ont trouvé l'ensemble de modèles calibrés le meilleur en moyenne ; quand les deux membres divergent nettement (voir leurs métriques dans l'onglet Validation), dites-le dans le rapport.",
                "**La forêt aléatoire seule** surtout pour comparer. Les arbres ne savent pas extrapoler : hors des conditions d'apprentissage, la forêt prédit des valeurs constantes, et avec des blocs spatiaux elle peut obtenir de moins bons scores que MaxEnt (sur la démonstration synthétique, Boyce 0,22 contre 0,57 avec 2 000 points de fond, mais équivalent avec des plis aléatoires).",
              ],
            },
            {
              note: "Coût : chaque forêt prédit environ un million de cellules en 5 s (500 arbres, 8 cœurs), et les modèles de pli sont aussi prédits pour la carte d'incertitude : environ 25 s par espèce pour la France métropolitaine à 1 km avec 4 blocs. L'ensemble coûte MaxEnt plus la forêt ; les modèles nuls réajustent le réglage retenu, forêts comprises.",
              tone: "info",
            },
            {
              p: "Ces résultats sont des moyennes sur de nombreuses espèces, pas des garanties pour la vôtre : si MaxEnt et la forêt donnent des cartes différentes, aucune n'a raison par défaut.",
            },
          ],
        },
        reading: {
          title: "Lire les résultats d'une forêt aléatoire",
          blocks: [
            {
              ul: [
                "**Importance des variables** (onglet Variables) : importance par permutation, la baisse de l'AUC d'apprentissage quand les valeurs d'une variable sont mélangées (3 répétitions), normalisée à 100. Elle indique combien la forêt s'appuie sur une variable, pas son effet écologique ; des variables corrélées se partagent le crédit.",
                "**Dépendance partielle** (à la place des courbes de réponse) : l'idonéité moyenne quand une variable varie, moyennée sur les valeurs observées des autres (Friedman 2001). Les courbes de réponse de MaxEnt fixent les autres variables à leur moyenne : les deux types de courbes ne se comparent pas directement. Les courbes de la forêt sont en escalier et plates au-delà de la plage d'apprentissage.",
                "**Fichiers** : `model.rf.json` contient la forêt (format, réglages, graine, arbres) ; `partial_dependence.csv` les courbes. Avec l'ensemble, `model.lambdas` est le membre MaxEnt.",
              ],
            },
          ],
        },
      },
    },
    "field-data": {
      title: "Vos données de terrain",
      summary:
        "Importez les observations de votre organisation (CSV), utilisez-les avec GBIF ou à sa place, ou mettez-les de côté pour valider le modèle sur des données présence–absence indépendantes.",
      sections: {
        overview: {
          title: "À quoi cela sert",
          blocks: [
            {
              p: "Beaucoup de bureaux d'études disposent de leurs propres relevés : inventaires, transects, placettes protocolées, souvent avec des lieux où l'espèce a été cherchée et **non trouvée**. NicheRS peut les utiliser de deux façons (ADR-0108) :",
            },
            {
              ul: [
                "**Comme présences**, avec GBIF ou à sa place : vos présences passent exactement par les mêmes règles de nettoyage que les données GBIF.",
                "**Mises de côté pour la validation** : le modèle est ajusté sur GBIF seul, puis évalué sur vos sites prospectés (présences et absences). C'est le seul contrôle indépendant d'une carte, et la seule voie vers le niveau *Validated*.",
              ],
            },
            {
              note: "Ces options sont proposées dans l'ADR-0108, en attente de revue par les écologues de Natural Solutions. Une étude sans vos données est calculée exactement comme avant.",
              tone: "info",
            },
          ],
        },
        import: {
          title: "Importer un fichier",
          blocks: [
            {
              p: "À l'étape **Données** (parcours Expert), ouvrez *Données de terrain de l'organisation* et importez un fichier CSV (UTF-8 ou Windows-1252, séparateur `;`, `,` ou tabulation). Excel : « Enregistrer sous CSV ».",
            },
            {
              ol: [
                "Choisissez le fichier : il est lu **dans votre navigateur** ; un extrait s'affiche et rien n'est encore envoyé.",
                "Indiquez la colonne du nom d'espèce, de X et de Y, et si possible la date ou l'année, la précision des coordonnées en mètres, le statut présence/absence et le protocole ou relevé.",
                "Choisissez le système de coordonnées (section suivante) et, pour une colonne de statut, cochez les valeurs signifiant « prospecté, non trouvé ».",
                "Importez : seules les colonnes choisies sont envoyées. Le rapport d'import donne les lignes retenues et rejetées, avec le motif et les numéros de ligne des premières lignes rejetées.",
              ],
            },
            {
              p: "Les lignes rejetées sont des problèmes du fichier (pas de nom d'espèce, coordonnées illisibles, hors du système choisi, date ou précision illisible) ; elles sont distinctes des règles de nettoyage scientifiques appliquées ensuite. Une date vide n'est pas un rejet : la donnée est « non datée » pour le nettoyage.",
            },
            {
              note: "Un jeu de données ne se modifie pas : pour changer la correspondance des colonnes, importez de nouveau le fichier. Supprimer un jeu ne touche pas les études déjà lancées : chacune garde sa propre copie, supprimée avec l'étude.",
              tone: "info",
            },
          ],
        },
        crs: {
          title: "Systèmes de coordonnées",
          blocks: [
            {
              p: "Les coordonnées sont converties en longitude/latitude WGS 84 sur le serveur, avec des formules vérifiées contre PROJ. La liste est fermée : WGS 84 en degrés (EPSG:4326), Lambert-93 (EPSG:2154, la plupart des données françaises), LAEA Europe (EPSG:3035) et les fuseaux UTM 28N–32N (ETRS89 et WGS 84).",
            },
            {
              p: "RGF93 et ETRS89 sont assimilés à WGS 84 (écart inférieur à 1 m, très en dessous de la maille d'environ 1 km). Les coordonnées hors de l'étendue plausible du système choisi sont rejetées « hors du système choisi » : en général le mauvais système a été choisi, ou longitude et latitude sont inversées.",
            },
            {
              note: "Le Lambert II étendu (EPSG:27572) et les systèmes d'outre-mer ne sont pas encore proposés : convertissez ces fichiers en Lambert-93 ou en WGS 84 au préalable.",
              tone: "warn",
            },
          ],
        },
        matching: {
          title: "Rapprochement des noms d'espèces",
          blocks: [
            {
              p: "Chaque modèle porte sur un taxon du référentiel GBIF. Les noms distincts de votre colonne espèce sont rapprochés avec l'outil de correspondance de GBIF : **seuls les noms partent vers GBIF**, jamais les coordonnées ni les effectifs.",
            },
            {
              p: "Une correspondance exacte au rang espèce ou inférieur est proposée automatiquement ; une correspondance approchée doit être vérifiée ; une correspondance à un rang supérieur (genre…) n'est pas modélisée. Vous décidez : attribuez chaque nom à une espèce de l'étude (ou ajoutez l'espèce trouvée) et laissez de côté les noms inutiles. Plusieurs noms peuvent aller à une même espèce (synonymes, sous-espèces).",
            },
          ],
        },
        sources: {
          title: "Sources par espèce",
          blocks: [
            {
              table: {
                caption:
                  "Comment une espèce peut utiliser les données de l'organisation",
                head: ["Choix", "Données d'apprentissage", "Vos absences"],
                rows: [
                  ["GBIF seul", "données GBIF", "non utilisées"],
                  [
                    "GBIF + vos présences",
                    "vos présences d'abord, puis GBIF ; une maille renseignée par les deux compte une fois, comme vôtre",
                    "jamais des présences ; évaluées si possible (voir validation)",
                  ],
                  [
                    "Vos présences seules",
                    "vos présences ; aucun téléchargement GBIF",
                    "jamais des présences",
                  ],
                  [
                    "GBIF pour l'apprentissage, vos données mises de côté",
                    "données GBIF seules",
                    "avec vos présences, sites de validation indépendante",
                  ],
                ],
              },
            },
            {
              p: "L'onglet Données des résultats montre, pour chaque source, les données, les mailles de présence retenues et pourquoi les autres ne l'ont pas été, ainsi que le nombre de données GBIF tombant dans des mailles déjà renseignées par vos données.",
            },
          ],
        },
        cleaning: {
          title: "Mêmes règles que GBIF",
          blocks: [
            {
              p: "Vos présences passent les mêmes règles, dans le même ordre : datées, postérieures au début de la période climatique, précision dans la limite, dans la zone d'étude, sur terre, avec données environnementales, une donnée par maille, filtrage spatial facultatif. Deux différences seulement : la règle de licence accepte toujours les données propres de l'organisation, et une **donnée d'absence n'est jamais une présence** (règle *absence*, en premier).",
            },
            {
              note: "Les données de terrain sont souvent groupées (un transect, un site d'étude). Quand vos mailles de présence couvrent moins de 10 % de l'emprise de la zone d'étude, une note rappelle que les zones non prospectées ne sont pas des absences.",
              tone: "info",
            },
          ],
        },
        validation: {
          title: "Validation indépendante",
          blocks: [
            {
              p: "Avec des données mises de côté pour la validation, la carte finale est évaluée sur vos sites prospectés. Les données passent les mêmes règles de nettoyage et sont réduites à **un site par maille** (une maille avec au moins une détection est un site de présence).",
            },
            {
              note: "**Règle anti-fuite.** Un site situé dans une maille de présence d'apprentissage (GBIF ou vos données) est exclu : le modèle connaît déjà son issue. Cela retire aussi vos observations republiées par GBIF (SINP → INPN → GBIF) dans la même maille.",
              tone: "warn",
            },
            {
              ul: [
                "**AUC (présence–absence)** : probabilité qu'un site de présence soit mieux classé qu'un site d'absence.",
                "**Sensibilité, spécificité, TSS et kappa** à deux seuils issus des données d'apprentissage (10e percentile des présences d'apprentissage, maximum de sensibilité + spécificité), jamais ajustés sur vos données (Allouche et al. 2006). Lisez le TSS avec la sensibilité et la spécificité : il dépend de la prévalence quand les erreurs sont déséquilibrées (Leroy et al. 2018).",
                "**Score de Brier et table de calibration** : diagnostics seulement. La favorabilité est relative, pas une probabilité de présence ; ils montrent la part des sites prospectés occupés selon la favorabilité, sans la transformer en probabilité.",
                "**Répartition** : emprise des sites et distance moyenne entre sites voisins. Une validation sur un petit groupe de sites dit peu du reste de la zone.",
              ],
            },
            {
              p: "Il faut au moins 10 sites de présence et 10 d'absence pour calculer les métriques. Avec les absences d'un jeu dont les présences ont servi à l'apprentissage, il ne reste aucun site de présence indépendant : mettez le jeu de côté pour l'évaluer.",
            },
          ],
        },
        validated: {
          title: "Le niveau Validated",
          blocks: [
            {
              p: "*Validated* est au-dessus de *Supported*. Il n'est attribué que si **tous** les critères sont remplis : données mises de côté de l'apprentissage ; au moins 30 sites de présence et 30 d'absence après la règle anti-fuite ; AUC présence–absence ≥ 0,75 ; TSS ≥ 0,40 au seuil max. SSS des données d'apprentissage ; et le modèle remplit tous les critères de *Supported* (après tout plafonnement).",
            },
            {
              note: "Ces critères sont proposés dans l'ADR-0108 et attendent la revue des écologues de Natural Solutions. Quand des données indépendantes ont été utilisées, les motifs du niveau le disent et listent chaque critère (✓/✗) ; sinon ils gardent « No independent validation data… ».",
              tone: "info",
            },
          ],
        },
        confidentiality: {
          title: "Confidentialité",
          blocks: [
            {
              ul: [
                "Seules les colonnes choisies quittent votre navigateur ; observateurs, commentaires et autres colonnes jamais.",
                "Les jeux de données ne sont visibles que de votre organisation ; une autre organisation reçoit « introuvable ». Le support Natural Solutions ne les liste pas.",
                "Les coordonnées n'apparaissent jamais dans les journaux, le manifeste ou les rapports : ceux-ci donnent l'empreinte SHA-256 du fichier, la correspondance des colonnes, le système et les effectifs. Vos points n'apparaissent que sur la carte de votre étude et dans son fichier d'occurrences (visibles de votre organisation).",
                "Les jeux sont supprimés avec la même durée de conservation que les études, ou par vous à tout moment.",
              ],
            },
          ],
        },
      },
    },
    future: {
      title: "Projections sous climat futur",
      summary:
        "Projeter un modèle sur les climats futurs CHELSA CMIP6 : périodes, scénarios d'émissions (SSP) et modèles climatiques (GCM), lecture des cartes d'évolution et limites.",
      sections: {
        overview: {
          title: "Ce que fait une projection",
          blocks: [
            {
              p: "En mode Expert (étape modèle, *Projections sous climat futur*), une étude peut projeter son modèle final sur des climats futurs. Le modèle est ajusté une seule fois sur le climat 1981–2010 ; pour chaque scénario, chaque prédicteur **climatique** est remplacé par sa valeur future selon chaque modèle climatique choisi, et le modèle prédit à nouveau. Relief, occupation du sol et eau gardent leurs valeurs actuelles.",
            },
            {
              p: "Les couches futures sont les climatologies CHELSA v2.1 CMIP6 (Karger et al. 2017), descendues à 30″ à partir de cinq modèles climatiques ISIMIP3b ; elles sont lues sur exactement la même fenêtre de grille et agrégées comme les couches actuelles. Proposé dans l'ADR-0016, en attente de revue par les écologues de Natural Solutions.",
            },
            {
              note: "Une projection montre où des **conditions climatiques favorables** pourraient se trouver à l'avenir. Ce n'est pas une prévision de l'endroit où vivra l'espèce : ni dispersion, ni dynamique des populations, ni adaptation, ni interactions biotiques, ni changement d'usage des sols ne sont modélisés.",
              tone: "warn",
            },
          ],
        },
        scenarios: {
          title: "Périodes, SSP et modèles climatiques",
          blocks: [
            {
              ul: [
                "**Périodes** : climatologies de 30 ans 2011–2040, 2041–2070 et 2071–2100.",
                "**SSP** (trajectoires socio-économiques partagées ; O'Neill et al. 2016) : SSP1-2.6 forte atténuation (environ 2 °C en 2100), SSP3-7.0 rivalités régionales et peu d'atténuation (environ 3,5–4 °C), SSP5-8.5 développement fondé sur les énergies fossiles (le plus chaud). Ce sont des scénarios, pas des prévisions : aucun n'est plus probable qu'un autre, comparez-en au moins deux.",
                "**Modèles climatiques (GCM)** : GFDL-ESM4, IPSL-CM6A-LR, MPI-ESM1-2-HR, MRI-ESM2-0 et UKESM1-0-LL. Ils donnent des futurs différents pour un même scénario ; **utilisez les cinq** pour que cet écart soit visible (Thuiller et al. 2019).",
              ],
            },
            {
              note: "Coût : chaque combinaison scénario × modèle lit une fois les couches climatiques du modèle (téléchargées à la première utilisation, puis en cache) et prédit une fois toute la zone d'étude ; une étude avec 5 modèles coûte donc environ 5 prédictions actuelles par scénario. Au plus 15 combinaisons scénario × modèle par étude.",
              tone: "info",
            },
          ],
        },
        reading: {
          title: "Lire les cartes et les surfaces",
          blocks: [
            {
              ul: [
                "**Idonéité future** : la moyenne, sur les modèles climatiques, de l'idonéité relative prédite avec le climat futur. Même échelle que la carte actuelle, jamais une probabilité.",
                "**Écart entre modèles** : l'écart-type de cette idonéité entre les modèles. Des valeurs élevées signifient que les modèles divergent pour la cellule.",
                "**Évolution des conditions favorables** : le seuil P10 actuel appliqué aux deux périodes. Chaque cellule est *favorable aujourd'hui et dans le futur*, *plus favorable*, *nouvellement favorable* ou *défavorable dans les deux cas*, d'après la moyenne des modèles. Cellules pâles : moins d'une majorité de modèles concordent avec cette classe.",
                "**Hachures** : conditions futures hors de l'étendue des données d'apprentissage sous au moins un modèle (MESS futur < 0). Les projections y sont des extrapolations ; ne les interprétez pas sans le dire.",
              ],
            },
            {
              p: "Le tableau des surfaces donne chaque classe en km² (surfaces exactes des cellules sur l'ellipsoïde) et en part de la surface favorable actuelle, la partie où tous les modèles concordent, et l'écart de la surface favorable future entre modèles. Rapportez l'écart, pas la moyenne seule.",
            },
          ],
        },
        limits: {
          title: "Limites",
          blocks: [
            {
              ul: [
                "**Pas de dispersion** : *nouvellement favorable* désigne des conditions que l'espèce pourrait tolérer, pas des lieux qu'elle atteindra ; *plus favorable* ne signifie pas une extinction locale à une date donnée.",
                "**Seul le climat change** : occupation et usage du sol, relief et eau restent ceux d'aujourd'hui, alors qu'ils changeront aussi.",
                "**Transfert du modèle** : des relations ajustées sur les conditions 1981–2010 peuvent ne pas tenir sous un autre climat (Yates et al. 2018), surtout là où les conditions futures sont nouvelles (hachures ; Elith et al. 2010).",
                "**L'incertitude des modèles climatiques** est montrée, pas supprimée ; l'écart entre SSP est généralement plus grand en fin de siècle.",
                "Écrivez « évolution de la surface de conditions favorables », jamais « déplacement d'aire » ni « distribution future ».",
              ],
            },
          ],
        },
        files: {
          title: "Fichiers et citation",
          blocks: [
            {
              p: "Par scénario : `future_<période>_<ssp>_mean.tif` (idonéité moyenne), `_sd.tif` (écart), `_change.tif` (classes 1 défavorable dans les deux cas, 2 plus favorable, 3 nouvellement favorable, 4 favorable dans les deux cas), `_agreement.tif` (modèles concordant avec la classe) et `_mess.tif` (MESS futur le plus bas). `future_areas.csv` liste les surfaces par scénario et par modèle ; les rapports ont une section *Future climate projections* et `manifest.json` liste chaque fichier futur lu.",
            },
            {
              p: "Citez CHELSA (Karger et al. 2017), CMIP6 (Eyring et al. 2016) et la correction de biais ISIMIP3b (Lange 2019), et remerciez le Programme mondial de recherche sur le climat (WCRP) et les cinq groupes de modélisation : les rapports contiennent ce texte. Les sorties des modèles CMIP6 sont sous CC BY 4.0 ; la licence des fichiers futurs CHELSA est en cours de confirmation auprès de CHELSA.",
            },
          ],
        },
      },
    },
    decision: {
      title: "Aide à la décision",
      summary:
        "Priorité de prospection, emprises de projet, espèces aux conditions favorables et comparaison de deux études : des résultats calculés à partir d'une étude terminée pour appuyer une décision.",
      sections: {
        overview: {
          title: "À quoi servent ces résultats",
          blocks: [
            {
              p: "Une carte d'idonéité relative ne répond pas seule à la question d'un client. L'onglet **Aide à la décision** d'une étude terminée calcule, à la demande, quatre résultats qui y répondent : où prospecter, ce que recoupe l'emprise d'un projet, quelles zones comptent pour plusieurs espèces, et à quel point une carte change entre deux études.",
            },
            {
              ul: [
                "Ils sont calculés à partir des résultats enregistrés de l'étude : **aucun modèle n'est réajusté**, et les fichiers et le manifeste de l'étude ne sont pas modifiés.",
                "Ils héritent de toutes les limites des modèles dont ils viennent : lisez d'abord le niveau de preuve de chaque espèce.",
              ],
            },
            {
              note: "Ces résultats ne remplacent ni un inventaire de terrain ni une étude d'impact. Une faible idonéité n'est pas une preuve d'absence.",
              tone: "warn",
            },
          ],
        },
        priority: {
          title: "Priorité de prospection : les zones à prospecter",
          blocks: [
            {
              p: "**Ce à quoi elle répond :** où des prospections de terrain pour une espèce valent le plus la peine — des lieux d'idonéité relative plus élevée et peu échantillonnés. **Ce à quoi elle ne répond pas :** où se trouve l'espèce. Une priorité élevée signifie « à prospecter », pas « présente ».",
            },
            {
              p: "Chaque cellule de la zone d'étude reçoit un score de 0 à 1 : son rang d'idonéité multiplié par le faible échantillonnage. Quatre classes croisent l'idonéité (au-dessus ou au-dessous du seuil P10 de l'espèce) et l'échantillonnage (moins ou plus que la cellule médiane) :",
            },
            {
              ul: [
                "**À prospecter en priorité** — idonéité plus élevée, peu échantillonnée.",
                "**Connue** — idonéité plus élevée, déjà bien échantillonnée.",
                "**Idonéité plus faible, peu échantillonnée** et **idonéité plus faible, bien échantillonnée**.",
              ],
            },
            {
              p: "**Comment l'échantillonnage est mesuré.** Si l'étude a utilisé un fond de groupe cible, l'effort d'échantillonnage est le nombre de données du groupe cible par cellule ([biais d'échantillonnage](#data)). Sinon NicheRS utilise une approximation, la **distance à la donnée connue la plus proche de l'espèce**, et l'indique. Cette approximation est plus faible : un lieu éloigné des données a pu être prospecté sans que l'espèce y soit trouvée.",
            },
            {
              p: "Les zones candidates sont tirées des 10 % de la zone d'étude aux scores de priorité les plus élevés : des groupes de cellules voisines « à prospecter en priorité », classés par score moyen, avec centroïde, emprise, surface et part où le modèle extrapole. Elles se téléchargent en GeoJSON et CSV pour préparer le terrain. L'échantillonnage guidé par un modèle a aidé à trouver des espèces rares (Guisan et al. 2006 ; Le Lay et al. 2010).",
            },
          ],
        },
        footprint: {
          title: "Emprise de projet : un pré-diagnostic",
          blocks: [
            {
              p: "Déposez l'emprise d'un projet (polygone GeoJSON en longitude/latitude) sur une étude terminée. Pour chaque espèce, NicheRS indique ce que disent les modèles à l'intérieur :",
            },
            {
              ul: [
                "la surface aux conditions favorables dans l'emprise au seuil P10, avec la fourchette entre le seuil le plus inclusif (MTP) et le maximum de sensibilité + spécificité ;",
                "la part de l'emprise qu'elle représente, et la part de la surface favorable de l'espèce dans toute la zone d'étude que l'emprise recouvre ;",
                "l'idonéité relative moyenne et maximale, l'incertitude, et les parts où le modèle extrapole ou sort de son domaine d'applicabilité ;",
                "le niveau de preuve de chaque modèle.",
              ],
            },
            {
              p: "Les surfaces sont exactes : l'emprise est intersectée avec chaque cellule du modèle sur l'ellipsoïde WGS84. Mais les valeurs du modèle portent sur des cellules entières : une emprise plus petite qu'une cellule est décrite par les cellules qu'elle touche, et NicheRS le signale.",
            },
            {
              note: "Un pré-diagnostic seulement : il ne remplace ni un inventaire de terrain ni une étude d'impact, et une faible idonéité dans une emprise n'est pas une preuve d'absence de l'espèce.",
              tone: "warn",
            },
            {
              p: "Les emprises sont enregistrées avec l'étude, visibles de votre seule organisation, et supprimées avec l'étude.",
            },
          ],
        },
        richness: {
          title: "Espèces aux conditions favorables et points chauds",
          blocks: [
            {
              p: "Pour une étude de plusieurs espèces, NicheRS empile leurs cartes. Chaque cellule reçoit le **nombre d'espèces aux conditions favorables** (chaque espèce à son propre seuil P10) et la **somme des idonéités relatives**, un indice pour classer les cellules.",
            },
            {
              note: "Ce n'est pas le nombre d'espèces présentes. L'empilement surestime : chaque carte montre où les conditions conviennent à une espèce, pas où elle vit, et ignore la compétition, la dispersion et le nombre d'espèces qu'un lieu peut accueillir (Calabrese et al. 2014 ; Guisan & Rahbek 2011).",
              tone: "warn",
            },
            {
              ul: [
                "Seules les espèces au niveau de preuve minimal ou au-dessus sont empilées — Indicative par défaut, donc **les modèles Exploratory sont exclus** sauf si vous abaissez le minimum. Les espèces exclues sont listées avec la raison.",
                "Les **points chauds** sont les 10 % de cellules ayant le plus d'espèces aux conditions favorables (égalités départagées par la somme des idonéités). Le seuil de 10 % est une convention, pas un seuil écologique.",
              ],
            },
          ],
        },
        compare: {
          title: "Comparer deux études",
          blocks: [
            {
              p: "Comparez deux études terminées de la même espèce — par exemple fond aléatoire ou de groupe cible, MaxEnt ou forêt aléatoire, climat seul ou climat + occupation du sol. NicheRS montre les paramètres qui diffèrent, les métriques, niveaux de preuve et surfaces des deux études, et une carte de la différence (B − A).",
            },
            {
              ul: [
                "Les deux études doivent être sur la **même grille** (même zone d'étude et même résolution) ; sinon la comparaison est refusée plutôt que rééchantillonnée.",
                "Le **D de Schoener** (0 = aucun recouvrement, 1 = identiques) mesure la ressemblance des deux surfaces d'idonéité une fois chacune ramenée à une somme de 1 (Warren et al. 2008) ; la **corrélation de Spearman** compare les rangs des cellules.",
                "L'accord au seuil P10 donne la surface favorable dans les deux études, dans une seule ou dans aucune.",
              ],
            },
            {
              p: "C'est un test de sensibilité : il indique à quel point la réponse dépend d'un choix, pas quelle étude a raison. Jugez l'ajustement avec les métriques de validation de chaque étude, en gardant à l'esprit que des plans de validation différents ne sont pas directement comparables.",
            },
          ],
        },
        files: {
          title: "Fichiers et traçabilité",
          blocks: [
            {
              p: "Chaque résultat est enregistré dans un dossier `analyses/` de l'étude, avec ses rasters (GeoTIFF), tableaux (CSV, GeoJSON), une page imprimable (`report.html`) et son propre petit manifeste. Ce manifeste enregistre la somme de contrôle du manifeste d'exécution de l'étude et de chaque fichier de l'étude lu, la méthode et les paramètres, pour remonter à l'étude exacte.",
            },
            {
              p: "Les résultats et le manifeste de l'étude ne sont jamais modifiés. Supprimer une étude supprime ses résultats d'aide à la décision.",
            },
          ],
        },
      },
    },
    exports: {
      title: "Exports et reproductibilité",
      summary:
        "Rasters GeoTIFF, GeoJSON, .lambdas, courbes de réponse, rapport imprimable et manifeste d'exécution.",
      sections: {
        files: {
          title: "Fichiers par espèce",
          blocks: [
            {
              table: {
                caption: "Fichiers exportés",
                head: ["Fichier", "Contenu"],
                rows: [
                  [
                    "`suitability.tif`",
                    "Idonéité relative de l'habitat (cloglog 0–1) du modèle final ; pas une probabilité.",
                  ],
                  [
                    "`uncertainty.tif`",
                    "Écart-type de l'idonéité entre les modèles de validation croisée.",
                  ],
                  [
                    "`mess.tif`",
                    "MESS ; les valeurs négatives sont de l'extrapolation.",
                  ],
                  [
                    "`effort.tif`",
                    "Seulement avec un fond par groupe cible : données GBIF du groupe cible par cellule (des données, pas des visites ni des observateurs ; synthétique dans la démonstration).",
                  ],
                  [
                    "`model.lambdas`",
                    "Modèle final au format `.lambdas` de MaxEnt 3.4.4.",
                  ],
                  [
                    "`occurrences.geojson`",
                    "Présences conservées et occurrences exclues avec leur motif (plafonné ; décomptes exacts dans le rapport). Coordonnées telles que publiées par GBIF : vérifiez le référentiel régional du SINP avant de reproduire des points dans un rapport public.",
                  ],
                  [
                    "`report.md`",
                    "Synthèse des méthodes et résultats (style ODMAP), point de départ de la partie méthodes d'un rapport.",
                  ],
                  [
                    "`report.html`",
                    "Rapport imprimable : cartes avec les présences, incertitude, extrapolation, courbes de réponse, tableaux, limites et citations. « Ouvrir le rapport imprimable » l'affiche dans le navigateur ; imprimez-le en PDF pour le joindre à une étude.",
                  ],
                  [
                    "`response_curves.csv`",
                    "Courbes de réponse marginales du modèle final (`variable,x,y`, 501 points par variable).",
                  ],
                  [
                    "`model.rf.json`",
                    "Forêt aléatoire ou ensemble seulement : la forêt aléatoire sous-échantillonnée finale (réglages, graine, arbres ; JSON). Avec l'ensemble, `model.lambdas` est son membre MaxEnt.",
                  ],
                  [
                    "`partial_dependence.csv`",
                    "Forêt aléatoire ou ensemble seulement : dépendance partielle de la forêt finale (`variable,x,y`, 51 points par variable).",
                  ],
                  [
                    "`manifest.json`",
                    "Manifeste d'exécution de l'étude (bouton « Télécharger le manifeste d'exécution »).",
                  ],
                ],
              },
            },
            {
              p: "Pour une espèce *Insufficient*, seuls `occurrences.geojson`, `report.md` et `report.html` sont produits.",
            },
          ],
        },
        geotiff: {
          title: "Format GeoTIFF",
          blocks: [
            {
              p: "Les rasters sont des GeoTIFF Float32 mono-bande en coordonnées géographiques WGS84 (EPSG:4326) sur la grille native CHELSA, tuilés 256 × 256 avec compression Deflate, organisés comme un Cloud-Optimized GeoTIFF mais sans aperçus (overviews). NoData vaut NaN. Ils s'ouvrent directement dans QGIS et GDAL.",
            },
            {
              p: "Pour calculer des surfaces dans un SIG, utilisez des surfaces ellipsoïdales ou une projection équivalente, jamais un nombre de cellules.",
            },
          ],
        },
        manifest: {
          title: "Manifeste d'exécution",
          blocks: [
            {
              p: "Le manifeste consigne ce qu'il faut pour tracer une étude : la spécification complète (espèces, zone d'étude, filtres, réglages du modèle et de la validation) et son SHA-256, la graine aléatoire, la version du logiciel, la provenance des données environnementales, la source GBIF avec son téléchargement, sa citation DOI et sa date d'accès, les méthodes (sélection, incertitude, extrapolation, surface, règles de preuve) et les résultats par espèce avec l'empreinte SHA-256 de chaque fichier.",
            },
            {
              p: "Les études sont immuables. Utilisez « Dupliquer et modifier » pour créer une variante ; la nouvelle étude a son propre manifeste. Conservez le manifeste avec le livrable : c'est la référence si un résultat est contesté.",
            },
          ],
        },
      },
    },

    account: {
      title: "Quota et compte",
      summary:
        "Service hébergé : espèces modélisées par mois, invitations, réinitialisation, suppression et conservation.",
      sections: {
        quota: {
          title: "Quota mensuel : espèces modélisées",
          blocks: [
            {
              p: "Le service hébergé compte les **espèces modélisées** : une espèce compte dès qu'un modèle est ajusté pour elle, quel que soit son niveau de preuve (*Exploratory* compris), dans le mois calendaire (UTC) où elle se termine.",
            },
            {
              ul: [
                "**Ne comptent pas :** les espèces *Insufficient* (aucun modèle ajusté), les espèces en échec ou annulées, la consultation et l'export de résultats existants.",
                "Une étude reprise n'est jamais comptée deux fois pour la même espèce.",
                "Supprimer une étude ne rend pas le quota.",
                "Au lancement, les espèces déjà consommées, plus celles réservées par les études en file ou en cours, plus les nouvelles doivent tenir dans le quota ; sinon l'étude est refusée (« Monthly quota reached »). Les résultats existants restent consultables.",
                "Le quota est mensuel et non reportable. Le menu du compte, dans la barre supérieure, indique les espèces restantes ce mois-ci ; Mon organisation → Vue d'ensemble détaille les espèces consommées, réservées et restantes.",
              ],
            },
          ],
        },
        limits: {
          title: "Limites par étude",
          blocks: [
            {
              ul: [
                "25 espèces au plus par étude.",
                "Emprise de la zone d'étude d'au plus 2,5 millions de cellules de la grille de modélisation (la France métropolitaine à 1 km en compte environ 2,0 millions).",
                "Les études sont mises en file équitablement entre organisations ; le premier téléchargement GBIF d'une étude peut prendre du temps (file d'attente propre au GBIF).",
              ],
            },
          ],
        },
        access: {
          title: "Invitations et mots de passe",
          blocks: [
            {
              ul: [
                "L'accès se fait uniquement sur invitation, émise par Natural Solutions ou par les **gestionnaires** de votre organisation (menu du compte → Mon organisation) : ils invitent leurs collègues, envoient des liens de réinitialisation de mot de passe et désactivent des comptes de l'organisation, dans la limite de son quota. Demandez à Natural Solutions de nommer un gestionnaire.",
                "Un lien d'invitation est à usage unique et expire au bout de 7 jours. NicheRS n'envoie aucun e-mail ; la personne qui vous invite vous transmet le lien, avec un message tout prêt.",
                "Les mots de passe comptent au moins 12 caractères ; changez le vôtre dans Mon compte (vos autres sessions sont fermées). Mot de passe oublié : demandez un lien de réinitialisation à un gestionnaire de votre organisation ou à Natural Solutions (même mécanisme) ; il ferme vos sessions ouvertes.",
                "Après 10 échecs de connexion pour un même e-mail en 15 minutes, la connexion est bloquée pour cet e-mail jusqu'à la fin de cette fenêtre.",
              ],
            },
          ],
        },
        gbif: {
          title: "Votre compte GBIF",
          blocks: [
            {
              p: "Les études téléchargent les occurrences GBIF avec un compte GBIF ; chaque téléchargement reçoit un DOI à citer dans les rapports. GBIF limite le nombre de téléchargements simultanés par compte : un compte partagé très sollicité fait attendre les études.",
            },
            {
              ol: [
                "**Votre propre compte** (menu du compte → Mon compte → Compte GBIF), si vous en avez configuré un.",
                "Sinon, **le compte de votre organisation**, configuré par un gestionnaire (Mon organisation).",
                "Sinon, **le compte partagé de Natural Solutions**.",
              ],
            },
            {
              ul: [
                "Saisissez votre **nom d'utilisateur** gbif.org (pas votre e-mail) et votre mot de passe. NicheRS demande aussitôt à GBIF s'il les accepte ; un compte refusé par GBIF est ignoré tant qu'il n'est pas corrigé.",
                "Le mot de passe est chiffré sur le serveur, jamais réaffiché et jamais écrit dans les journaux. Le manifeste de chaque étude consigne le type de compte qui a fait son téléchargement (le vôtre, celui de l'organisation ou le compte partagé), pas le nom d'utilisateur.",
                "Chaque compte garde ses propres téléchargements : une étude ne cite jamais un téléchargement fait avec le compte d'une autre organisation.",
                "Le compte est fixé au début de la préparation des données d'une étude ; le changer ensuite ne modifie pas les études déjà préparées.",
              ],
            },
          ],
        },
        retention: {
          title: "Suppression et conservation",
          blocks: [
            {
              ul: [
                "Vous pouvez supprimer à tout moment une étude terminée ; ses fichiers sont effacés.",
                "Les études terminées sont supprimées automatiquement au bout de 365 jours. Téléchargez ce que vous devez conserver (fichiers et manifeste).",
                "Les études de votre organisation ne sont visibles que par ses membres. Les administrateurs de Natural Solutions peuvent les ouvrir pour le support ; chaque accès de ce type est journalisé.",
                "Pour faire effacer votre compte (nom, e-mail et mot de passe), demandez-le à Natural Solutions : l'effacement est immédiat et définitif ; votre organisation conserve ses études. Les noms d'observateurs GBIF ne sont jamais conservés.",
              ],
            },
          ],
        },
      },
    },

    glossary: {
      title: "Glossaire",
      summary: "Définitions courtes des termes employés dans NicheRS.",
      sections: {
        terms: {
          title: "Termes",
          blocks: [
            {
              dl: [
                [
                  "Idonéité relative de l'habitat",
                  "Score MaxEnt de 0 à 1 par cellule, relatif à la zone d'étude ; pas une probabilité de présence.",
                ],
                [
                  "Cloglog",
                  "Transformation monotone ramenant la sortie brute de MaxEnt entre 0 et 1 (Phillips et al. 2017) ; pas une calibration.",
                ],
                [
                  "MaxEnt",
                  "Méthode de modélisation présence-fond (Phillips et al. 2006) ; NicheRS en utilise un portage testé contre maxent.jar 3.4.4.",
                ],
                [
                  "Occurrence / observation",
                  "Une donnée GBIF de l'espèce ; observée, non prédite.",
                ],
                [
                  "Cellule de présence",
                  "Cellule de la grille contenant au moins une occurrence conservée ; unité utilisée par le modèle et les règles de preuve.",
                ],
                [
                  "Fond (background)",
                  "10 000 cellules terrestres tirées au hasard dans la zone d'étude, décrivant les conditions disponibles.",
                ],
                ["Zone d'étude", "Zone où le modèle est ajusté et projeté."],
                [
                  "Classes de variables (features)",
                  "Formes de réponse permises à MaxEnt : L linéaire, Q quadratique, H charnière, P produit.",
                ],
                [
                  "Multiplicateur de régularisation (β)",
                  "Pénalité sur la complexité du modèle ; plus il est élevé, plus les réponses sont lisses.",
                ],
                [
                  "Validation croisée par blocs spatiaux",
                  "Validation où chacun des 4 blocs géographiques est prédit par un modèle entraîné sur les trois autres.",
                ],
                [
                  "Pli (fold)",
                  "Partie des données mise de côté lors de la validation croisée.",
                ],
                [
                  "Indice de Boyce continu (CBI)",
                  "De −1 à 1 : capacité des prédictions à classer les présences mises de côté par rapport au fond.",
                ],
                [
                  "OR10",
                  "Part des présences mises de côté sous le seuil du 10e percentile d'apprentissage ; attendu ≈ 10 %.",
                ],
                [
                  "ORmtp",
                  "Part des présences mises de côté sous le minimum des présences d'apprentissage ; attendu 0 %.",
                ],
                [
                  "AUC présence-fond",
                  "Discrimination des présences par rapport au fond ; dépend de la zone d'étude ; indicateur secondaire.",
                ],
                [
                  "Règle de seuil",
                  "MTP, P10 ou max SSS : manière de choisir la valeur d'idonéité qui délimite les conditions favorables.",
                ],
                [
                  "Surface de conditions favorables",
                  "Somme des surfaces exactes des cellules au-dessus d'un seuil ; pas la surface occupée.",
                ],
                [
                  "MESS / extrapolation",
                  "Similarité environnementale aux conditions d'apprentissage ; des valeurs négatives signalent une extrapolation.",
                ],
                [
                  "Incertitude (écart-type entre plis)",
                  "Écart-type par cellule de l'idonéité entre les modèles des plis.",
                ],
                [
                  "Niveau de preuve",
                  "*Insufficient*, *Exploratory*, *Indicative* ou *Supported*, issu de règles fixes (ADR-0009).",
                ],
                [
                  "Colinéarité",
                  "Corrélation entre prédicteurs ; |r| > 0,7 entraîne l'abandon d'une variable.",
                ],
                [
                  "CHELSA v2.1",
                  "Climatologies mondiales 1981–2010 à 30″, licence CC0 (Karger et al. 2017).",
                ],
                [
                  "Proportion d'occupation du sol",
                  "Part (0–1) d'une cellule couverte par un groupe de classes ESA WorldCover 2021.",
                ],
                [
                  "DOI du téléchargement GBIF",
                  "Identifiant permanent du jeu exact d'occurrences GBIF utilisé ; à citer.",
                ],
                [
                  "Manifeste d'exécution",
                  "Fichier JSON consignant entrées, versions, graine, réglages et empreintes des fichiers d'une étude.",
                ],
                [
                  "Espèce modélisée",
                  "Unité du quota : une espèce pour laquelle un modèle est ajusté.",
                ],
                [
                  "Démonstration synthétique",
                  "Jeu de données artificiel intégré pour apprendre l'outil ; jamais des données réelles.",
                ],
              ],
            },
          ],
        },
      },
    },

    responsible: {
      title: "Limites et usage responsable",
      summary:
        "Responsabilité de l'interprétation, citation dans un rapport client, attribution et limites connues.",
      sections: {
        responsibility: {
          title: "Responsabilité",
          blocks: [
            {
              note: "NicheRS est un outil d'aide à la décision. **L'interprétation des résultats et leur usage dans un livrable relèvent du professionnel qui signe le rapport.**",
              tone: "warn",
            },
            {
              p: "Le logiciel applique des règles documentées. Il ne connaît ni le site, ni l'écologie de l'espèce au-delà des occurrences, ni le contexte réglementaire.",
            },
          ],
        },
        citing: {
          title: "Citer un résultat dans un rapport client",
          blocks: [
            {
              ol: [
                "Nommez le logiciel et sa version (dans `report.md`) et la méthode : MaxEnt (Phillips et al. 2006, 2017), portage NicheRS de MaxEnt 3.4.4.",
                "Adaptez le texte de méthodes de `report.md` : données, filtres, exclusions, prédicteurs, calibration, validation.",
                "Donnez le niveau de preuve et ses motifs tels quels, y compris la phrase sur l'absence de validation indépendante.",
                "Décrivez la sortie comme « idonéité relative de l'habitat (cloglog, 0–1), qui n'est pas une probabilité de présence ».",
                "Donnez chaque surface avec sa règle de seuil et la fourchette selon les règles.",
                "Montrez les hachures d'extrapolation et la carte d'incertitude, ou indiquez leurs valeurs.",
                "Citez le DOI du téléchargement GBIF et CHELSA ; archivez `manifest.json` (et l'identifiant de l'étude) avec le livrable.",
              ],
            },
          ],
        },
        attribution: {
          title: "Attribution : GBIF et CHELSA",
          blocks: [
            {
              ul: [
                "GBIF : *GBIF.org (date) GBIF Occurrence Download* suivi du DOI de l'étude. Les occurrences CC BY sont attribuées par cette citation.",
                "CHELSA : Karger D.N. et al. (2017) Climatologies at high resolution for the earth's land surface areas. *Scientific Data* 4:170122, doi:10.1038/sdata.2017.122 ; données CHELSA v2.1, doi:10.16904/envidat.228.",
                "Masque terrestre : Natural Earth (domaine public).",
              ],
            },
          ],
        },
        avoid: {
          title: "À ne jamais faire",
          blocks: [
            {
              ul: [
                "N'écrivez jamais « probabilité de présence », un pourcentage de confiance, « aire de répartition » ni « surface occupée » à propos de ces résultats.",
                "Ne retirez jamais les hachures d'extrapolation d'une carte publiée.",
                "Ne comparez jamais les scores d'idonéité ni l'AUC entre espèces ou entre zones d'étude.",
                "N'utilisez jamais une idonéité faible comme preuve d'absence dans une étude d'impact.",
                "Ne fondez jamais une décision réglementaire sur une seule carte *Exploratory*.",
                "N'utilisez jamais les résultats de la démonstration synthétique, ni des résultats incluant des occurrences CC BY-NC, dans un livrable facturé.",
              ],
            },
          ],
        },
        limits: {
          title: "Limites connues de cette version",
          blocks: [
            {
              ul: [
                "Espèces terrestres uniquement ; climat 1981–2010 par défaut (relief, occupation du sol 2021 et distance à l'eau en option ; pas de sols ; projections sous climat futur en option).",
                "Pas de correction du biais d'échantillonnage par défaut (le fond par groupe cible est une option Expert qui nécessite le miroir GBIF régional) ; validation indépendante uniquement avec les données présence–absence de votre organisation (option Expert, [Vos données de terrain](#field-data)) : sans elles, la preuve vient de la validation croisée des mêmes occurrences.",
                "MaxEnt par défaut ; une forêt aléatoire sous-échantillonnée et un ensemble MaxEnt + forêt aléatoire sont des options expertes en attente de revue ([Algorithmes de modélisation](#algorithms)).",
                "Les cellules côtières dépendent d'un masque terrestre précis à environ 0,5–1 km.",
              ],
            },
          ],
        },
      },
    },
  },
};
