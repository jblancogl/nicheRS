# NicheRS expliqué depuis le début

**Comment le lire.** Si vous avez une minute : « En une page ». Si vous en avez cinq : le §1, le parcours du §2.4, les résultats réels du §5.7 et les questions fréquentes du §10. Si vous voulez comprendre la science : §3 à §5. Si vous voulez savoir où en est le projet : §7 et §8.

---

## En une page

- **Ce qu'il fait.** À partir des lieux où une espèce a été observée (GBIF) et du climat de chaque kilomètre carré (CHELSA), NicheRS cartographie les endroits où l'on retrouve des conditions semblables.
- **Ce qu'il ne fait pas.** Il ne dit pas où *se trouve* l'espèce et ne donne pas de probabilité : il donne une **idonéité relative** (de 0 à 1) et une **surface de conditions favorables**, sous forme de fourchette.
- **Comment s'y fier.** Chaque résultat est testé en cachant des régions entières et en vérifiant si le modèle les retrouve. Le bilan se résume en un **niveau de preuve** (*Insufficient*, *Exploratory*, *Indicative*, *Supported* et, avec des données de terrain, *Validated*) avec ses motifs, jamais en un pourcentage inventé.
- **Le moteur.** MaxEnt, la méthode de référence, réécrit en Rust et vérifié contre l'original : 16 cas sur 16 identiques. Une étude d'une espèce sur toute la France prend quelques secondes.
- **Sur données réelles.** Les espèces à niche climatique marquée (chêne vert) sortent *Supported* ; les généralistes (hérisson, vipère) sortent *Exploratory* quand seul le climat est utilisé. C'est la réponse honnête, pas une défaillance.
- **Science v1 (achevé le 20 octobre).** Ajoutés comme options expert, en attente de revue par des écologues : correction du biais d'observation, tests contre le hasard, relief, végétation et eau, une seconde méthode (Random Forest), projections au climat futur, données de terrain propres avec validation indépendante (niveau *Validated*) et sorties d'aide à la décision : où prospecter, ce que touche un projet, zones riches en espèces et comparaison d'études.
- **Ce qui manque.** Aucune des nouvelles méthodes n'a encore été utilisée de bout en bout sur une espèce réelle pour en tirer une vraie conclusion. Avant de les utiliser dans un rapport client, il faut la revue des écologues de Natural Solutions et des essais sur des espèces qu'ils connaissent.

---

## 1. La question en une phrase

> **« Dans quels endroits de cette zone trouve-t-on les conditions que cette espèce apprécie, et dans quelle mesure pouvons-nous nous fier à la réponse ? »**

Un exemple réel : un bureau d'études en écologie doit évaluer l'impact d'un parc éolien en Bretagne. Il doit savoir où le lézard à deux raies (*Lacerta bilineata*) pourrait être présent, même si personne ne l'a cherché sur chaque parcelle. Il n'y a ni le temps ni l'argent pour parcourir chaque kilomètre carré ; on utilise donc un **modèle de distribution d'espèces** (en anglais *Species Distribution Model*, SDM) : à partir des sites où l'espèce *a bien* été vue et du climat de toute la zone, le modèle estime quels autres sites présentent des conditions semblables.

NicheRS est un outil (une application web) qui fait cela de bout en bout, pour des personnes qui ne sont pas spécialistes de la modélisation.

---

## 2. Les idées de base

### 2.1 La niche écologique

Chaque espèce tolère une gamme de conditions : températures, pluie, saisonnalité, altitude, type de végétation… Un chêne vert supporte des étés secs et chauds ; un hêtre a besoin d'humidité. Cet ensemble de conditions dans lesquelles une espèce peut vivre s'appelle la **niche écologique**. (D'où le nom : *Niche* + *RS*, pour le langage de programmation Rust.)

L'idée centrale des SDM est simple :

1. On regarde **quelles conditions** règnent aux endroits où l'espèce a été observée.
2. On cherche sur la carte **d'autres endroits aux conditions semblables**.

### 2.2 Pourquoi il ne suffit pas de « regarder la carte des observations »

Parce que les observations disent où les gens ont regardé, pas où vit l'espèce :

- Il y a beaucoup plus de données près des villes, des routes et des réserves naturelles qu'en pleine nature.
- Une zone sans observations peut être une zone sans l'espèce… ou une zone que personne n'a visitée.
- Il y a des erreurs : animaux de zoo, plantes de jardin, fossiles, coordonnées mal saisies.

Le modèle essaie de séparer « là où les conditions lui plaisent » de « là où l'on a regardé ». Il n'y parvient pas complètement, et c'est pourquoi une grande partie du travail consiste à **mesurer et communiquer honnêtement** dans quelle mesure il y parvient, bien ou mal.

### 2.3 La carte comme un quadrillage

Tout se fait sur une **grille de cellules** d'environ 1 km de côté (exactement 30 secondes d'arc, la grille native des données climatiques). La France entière représente environ 930 000 cellules. Pour chaque cellule, on connaît ses conditions (température moyenne, pluie…) et, à la fin, le modèle lui attribue un score.

### 2.4 Le parcours d'une étude, en un coup d'œil

```
 1. Espèce + zone           « Lézard à deux raies, en Bretagne »
        │
 2. Observations (GBIF)     des milliers de données ──► nettoyage ──► cellules de présence
        │                                            (motif de chaque exclusion, visible)
 3. Environnement (CHELSA…) climat de chaque cellule (+ relief, végétation, eau : en option)
        │                   masque terrestre · filtre des variables redondantes
 4. Fond                    10 000 cellules tirées au hasard pour comparer
        │
 5. Modèle (MaxEnt)         25 configurations testées, examen par blocs géographiques
        │                   ──► choix selon des règles fixes ──► modèle final
 6. Évaluation              notes de l'examen, incertitude, extrapolation
        │
 7. Résultat                carte d'idonéité relative · surface favorable (fourchette)
                            niveau de preuve + motifs · rapport · manifeste
```

Chaque flèche laisse une trace : ce qui a été retiré, ce qui a été choisi et pourquoi. C'est cette trace qui permet à un écologue de revoir l'étude et à n'importe qui de la refaire.

---

## 3. Les données que nous utilisons

### 3.1 Où l'espèce a été vue : GBIF

Le **GBIF** (*Global Biodiversity Information Facility*) est la grande base de données mondiale d'observations d'espèces : musées, programmes scientifiques, applications de science participative comme iNaturalist ou Observation.org… Chaque donnée dit « cette espèce, à ces coordonnées, à cette date ». Ce sont des données de **présence seule** : elles disent où l'espèce a été vue, jamais où on l'a cherchée sans la trouver.

Avant de les utiliser, NicheRS les **nettoie** selon des règles fixes (décidées dans l'ADR-0009), et indique toujours combien de données il retire et pourquoi :

| Exclu | Pourquoi |
| --- | --- |
| Fossiles, animaux en captivité, plantes cultivées | Ils ne reflètent pas où vit l'espèce à l'état sauvage. |
| Données sous licence « non commerciale » (CC BY-NC) | Elles ne peuvent pas être utilisées dans un rapport d'études facturé. |
| Données sans date ou antérieures à 1981 | Le climat utilisé est celui de 1981–2010 ; une donnée de 1900 peut relever d'un autre climat. |
| Coordonnées dont l'incertitude est \> 1 km | Si le point peut se trouver à 5 km, on ne sait pas dans quelle cellule il tombe. |
| Coordonnées « floutées » par le fournisseur sans précision indiquée (nouveau, ADR-0011) | Les espèces sensibles (par ex. les rapaces nicheurs) sont publiées avec une position volontairement déplacée pour les protéger. Si l'on ne sait pas de combien, le point trompe le modèle. |
| Points en mer ou dans de grands lacs | Nous travaillons sur des espèces terrestres. |
| Doublons dans la même cellule | Dix photos du même lézard au même endroit ne font pas dix lézards. On en garde une par cellule. |

Une remarque sur le rang taxonomique : si l'utilisateur choisit un **genre** au lieu d'une **espèce** (par ex. « Quercus » au lieu de « Quercus ilex »), on mélangerait plusieurs espèces aux niches différentes. L'outil le signale et limite le niveau de confiance (nous l'avons ajouté le 16 octobre après avoir constaté le cas sur une étude réelle).

### 3.2 À quoi ressemble l'environnement : le climat CHELSA

Pour décrire les conditions de chaque cellule, nous utilisons **CHELSA v2.1**, un ensemble de cartes climatiques mondiales à 1 km. Par défaut, sept « variables bioclimatiques » sont utilisées :

- la température moyenne annuelle (BIO1), sa saisonnalité (BIO4), la maximale du mois le plus chaud (BIO5), la minimale du mois le plus froid (BIO6) ;
- les précipitations annuelles (BIO12), les précipitations du mois le plus sec (BIO14) et leur saisonnalité (BIO15).

Autrement dit : la chaleur, l'eau, leur variation au fil de l'année et leurs extrêmes.

Pourquoi CHELSA et non WorldClim, le plus connu ? Parce que **WorldClim interdit l'usage commercial** sans autorisation, alors que CHELSA est dans le domaine public (CC0). Natural Solutions est une entreprise : toutes les données par défaut doivent pouvoir être utilisées dans un travail rémunéré (ADR-0006).

Deux détails découverts en chemin :

- **CHELSA a des valeurs en mer** (par exemple 13,65 °C au milieu du golfe de Gascogne). Si l'on ne masque pas la mer, le modèle comparerait l'espèce à des conditions marines et paraîtrait meilleur qu'il ne l'est. C'est pourquoi nous appliquons un **masque terrestre** (Natural Earth) — ADR-0010.
- Certaines variables sont très **corrélées** entre elles (s'il fait chaud l'été, il fait généralement chaud en moyenne). Donner au modèle deux variables qui disent la même chose l'embrouille. Un filtre est appliqué : si deux variables se ressemblent trop (corrélation \> 0,7), on garde la première.

### 3.3 Au-delà du climat : relief, végétation et eau (en option)

À 1 km, le climat dit si une région convient, mais pas si l'on y trouve de la forêt, des champs ou de la ville. C'est pourquoi beaucoup d'espèces généralistes sortaient *Exploratory*. Depuis la phase 4 de Science v1 (ADR-0015), on peut ajouter, en mode Expert, douze variables supplémentaires, toutes sous licence commerciale :

| Groupe | Variables | Source |
| --- | --- | --- |
| Relief | altitude, pente, rugosité | Copernicus DEM (90 m) |
| Occupation du sol | proportion de chaque cellule couverte par des arbres, des arbustes, des herbacées, des cultures, du bâti, du sol nu, de l'eau, des zones humides | ESA WorldCover 2021 (10 m) |
| Eau | distance à l'eau la plus proche | ESA WorldCover 2021 |

On utilise des **proportions** (« cette cellule est à 60 % de la forêt et à 30 % des cultures ») plutôt qu'une seule classe (« cette cellule est une forêt »), car à 1 km presque toutes les cellules sont des mélanges. Toutes sont calculées sur la même grille que le climat, sans la déformer.

Par défaut, **aucune n'est sélectionnée** : une étude normale reste uniquement climatique. Et chacune a ses avertissements : l'occupation du sol est une photo de 2021 alors que les observations remontent jusqu'à 1981 ; le « bâti » peut refléter où vivent les observateurs plutôt que l'espèce ; le modèle d'élévation mesure la surface (cimes des arbres, toits), pas le sol.

### 3.4 Le « fond » : à quoi nous comparons

Comme on sait seulement où l'espèce *est* et non où elle *n'est pas*, le modèle compare les cellules avec observations à un échantillon de **10 000 cellules tirées au hasard** dans la zone d'étude, appelé **fond** (*background*). La question que se pose le modèle n'est pas « l'espèce est-elle ici ? », mais « en quoi les sites où l'espèce a été vue diffèrent-ils des sites disponibles en général ? ».

---

## 4. Le modèle : MaxEnt

### 4.1 L'idée, sans formules

**MaxEnt** (*Maximum Entropy*, Phillips et al., 2006) est la méthode la plus utilisée au monde pour ce problème. Sa logique :

> « Parmi toutes les façons de répartir l'espèce sur la carte qui sont cohérentes avec ce que nous avons observé (par exemple, que la température moyenne là où elle a été vue est d'environ 12 °C), choisir la plus étalée, celle qui suppose le moins. »

C'est une manière prudente de ne pas inventer des préférences que les données ne montrent pas. Pour décrire les préférences, MaxEnt peut utiliser des formes simples (linéaires : « plus il fait chaud, mieux c'est ») ou plus complexes (« elle aime entre 10 et 14 °C, ni plus ni moins »). Plus on autorise de complexité, plus le modèle colle aux données… et plus il risque de mémoriser le bruit au lieu d'apprendre la règle (**surajustement**). Un paramètre appelé **régularisation** (β) pénalise la complexité.

### 4.2 Pourquoi nous avons refait MaxEnt en Rust

Le programme de référence est `maxent.jar`, écrit en Java, gratuit et figé depuis 2020. Les écologues lui font confiance. Mais il exige Java, il est difficile à intégrer dans un service web et il est lent quand on enchaîne des centaines d'ajustements.

Nous avons décidé de **réécrire (porter) MaxEnt en Rust**, mais à une condition stricte : il doit donner **exactement les mêmes résultats** que `maxent.jar`. Pas des résultats « proches » : les mêmes. Cela a obligé à copier jusqu'aux manies internes du programme original (l'ordre dans lequel il additionne, sa façon de départager les ex æquo, des calculs qu'il réutilise même s'ils sont un peu périmés), car MaxEnt s'arrête avant d'atteindre l'optimum mathématique, et seul le fait de reproduire son chemin exact mène au même point (ADR-0001).

Résultat mesuré : **16 cas de test sur 16 identiques** à `maxent.jar` 3.4.4, dont 5 espèces réelles, avec des écarts de l'ordre de 0,00000000005. Chaque modification du moteur est revérifiée par rapport à l'original (`scripts/compare_maxent.py`) ; si le résultat n'est pas de 100 %, elle n'est pas intégrée.

### 4.3 Calibration automatique : choisir la meilleure configuration

Il n'existe pas de configuration de MaxEnt bonne pour toutes les espèces. NicheRS teste **25 configurations** (5 niveaux de complexité × 5 niveaux de régularisation) et en choisit une selon des critères fixes, de façon transparente :

1. celle qui **se trompe le moins sur des données qu'elle n'a pas vues** (taux d'omission, voir §5) ;
2. en cas d'égalité, celle qui **classe** le mieux les sites (indice de Boyce) ;
3. en cas d'égalité, la plus **simple**.

Ce critère favorise des modèles prudents et « lisses », qui fonctionnent mieux lorsqu'on les applique à de nouvelles zones, au prix d'une précision apparente un peu moindre sur leurs propres données. C'est un choix délibéré.

### 4.4 Ce que signifie le nombre sur la carte

Le modèle attribue à chaque cellule une valeur entre 0 et 1. **Ce n'est pas une probabilité de présence.** C'est une **idonéité relative** : 0,8 veut dire « des conditions plus proches de celles des observations qu'une cellule à 0,3 », et non « 80 % de chances de trouver l'espèce ». Avec des données de présence seule, il est impossible de calculer une vraie probabilité, car on ne sait pas combien de fois l'espèce a été cherchée sans succès.

Cette règle est écrite (ADR-0007) et non négociable : aucun texte, aucune carte ni aucun rapport de NicheRS ne mentionne une « probabilité » ni un « pourcentage de confiance » inventé.

---

## 5. Comment savoir si le modèle est bon ?

C'est la partie la plus délicate sur le plan scientifique, et celle où nous avons mis le plus d'efforts.

### 5.1 Interroger sur des questions inédites : la validation croisée

Si vous évaluez un élève avec les mêmes questions que celles qu'il a révisées, il aura 20/20 même s'il n'a rien compris. Il en va de même pour les modèles. C'est pourquoi on **cache une partie des données**, on entraîne le modèle sur le reste et on regarde s'il voit juste sur la partie cachée. On recommence plusieurs fois en changeant la partie cachée : c'est la **validation croisée**.

### 5.2 Cacher par zones, pas au hasard : les blocs spatiaux

Si l'on cache des points au hasard, chaque point caché a un voisin presque identique dans les données d'entraînement (à 500 m, avec le même climat), et l'examen est trop facile. C'est pourquoi le territoire est divisé en **4 blocs géographiques** (en coupant à la médiane de la latitude et de la longitude) et l'on cache un bloc entier à chaque fois. On vérifie ainsi si le modèle sait **se transposer à une autre zone**, ce qui est justement ce qu'on lui demande en pratique.

### 5.3 Les notes de l'examen

- **Taux d'omission au 10e percentile (OR10).** On fixe un seuil qui laisse de côté les 10 % d'observations d'entraînement ayant les plus mauvais scores, et on regarde quelle part des observations *cachées* tombe en dessous. Si le modèle n'est pas surajusté, elle devrait être ≈ 0,10. Si elle vaut 0,40, le modèle ne reconnaît pas de nouveaux sites où l'espèce est pourtant présente.
- **Indice de Boyce.** Il vérifie si, dans les zones auxquelles le modèle donne un score élevé, il y a réellement plus d'observations que le hasard n'en produirait. Il va de −1 à 1 : 0 signifie « comme jouer à pile ou face », une valeur proche de 1 signifie « très cohérent ».
- **AUC.** C'est la métrique la plus célèbre, mais avec des données de présence seule elle mesure surtout **la taille de la zone d'étude**, pas la qualité du modèle. Nous l'affichons dans le détail, mais nous ne l'utilisons pour rien décider.

### 5.4 Incertitude et extrapolation

- **Carte d'incertitude :** dans quelle mesure la prédiction d'une cellule varie entre les modèles de la validation croisée. Si elle varie beaucoup, ce résultat dépend des données utilisées.
- **Extrapolation (MESS) :** zones dont le climat est **plus extrême que toutes les données utilisées** (par ex. des sommets plus froids que n'importe quel point observé). Là, le modèle devine. Elles sont signalées par des hachures sur la carte, activées par défaut, et le pourcentage de surface concerné est indiqué.

### 5.5 Seuils et surface favorable

Pour dire « il y a X km² de conditions favorables », il faut fixer une coupure : au-dessus, « favorable ». Il n'existe pas de coupure unique correcte ; nous donnons donc une **fourchette selon trois règles** connues (la plus inclusive, celle des 10 % — utilisée par défaut — et celle qui équilibre réussites et erreurs). Les surfaces sont calculées avec la formule exacte de la surface de chaque cellule sur l'ellipsoïde terrestre. On dit toujours « surface de conditions favorables », jamais « surface occupée par l'espèce ».

### 5.6 Le niveau de preuve : le résumé honnête

Au lieu d'un pourcentage de confiance, chaque résultat porte un **niveau de preuve** calculé selon des règles écrites (ADR-0009), avec la liste des motifs qui le justifient :

| Niveau | Ce qu'il signifie | Conditions (toutes) |
| --- | --- | --- |
| **Insufficient** (insuffisant) | Aucun modèle n'est ajusté. | Moins de 10 cellules de présence. |
| **Exploratory** (exploratoire) | Sert à orienter, pas à décider. | Il y a un modèle, mais au moins un critère d'« Indicative » n'est pas satisfait. |
| **Indicative** (indicatif) | Utilisable avec prudence. | ≥ 25 cellules ; Boyce ≥ 0,40 ; OR10 ≤ 0,25 ; ≤ 25 % de la zone extrapolée. |
| **Supported** (étayé) | Le mieux que l'on puisse dire avec ces données. | ≥ 50 cellules ; Boyce ≥ 0,70 ; OR10 ≤ 0,15 ; ≤ 10 % extrapolée ; validation par blocs spatiaux. |
| **Validated** (validé) | La carte voit juste sur des données de terrain qu'elle n'a jamais vues. | Tout ce qu'exige « Supported », plus des données de terrain propres **mises de côté** pour la validation : ≥ 30 sites de présence et ≥ 30 d'absence ; AUC présence–absence ≥ 0,75 ; TSS ≥ 0,40. |

Le niveau **Validated** (phase 7, ADR-0108) est le seul qui ne dépend pas uniquement des données ayant servi à entraîner le modèle : il exige de confronter la carte à des données de terrain **indépendantes** de l'organisation (sites où l'on a cherché l'espèce et où on l'a trouvée ou non), expliquées au §8. Sans ces données, le résultat l'indique explicitement : « pas de données de validation indépendantes : la preuve vient de la validation croisée des mêmes occurrences, potentiellement biaisées ».

Conséquence attendue : beaucoup d'espèces aux données opportunistes sortiront *Exploratory* ou *Indicative*. Par exemple, le chêne vert (*Quercus ilex*) en France sort *Supported* ; la vipère aspic (*Vipera aspis*), une espèce généraliste, sort *Exploratory*. Ce n'est pas une défaillance de l'outil : c'est la réponse honnête.

### 5.7 Ce que dix espèces réelles nous ont appris

Le 15 octobre, dix espèces ont été modélisées sur toute la France métropolitaine à 1 km, avec le climat seul et les réglages par défaut (`docs/benchmarks/g6-france-batch.md`). Au plus 20 000 données par espèce ont été utilisées, tirées de façon répartie sur le territoire :

| Espèce | Niveau | Boyce | OR10 | Surface de conditions favorables (seuil à 10 %) |
| --- | --- | --- | --- | --- |
| Chêne vert (*Quercus ilex*) | *Supported* | 0,74 | 0,12 | 110 166 km² |
| Écureuil roux (*Sciurus vulgaris*) | *Supported* | 0,84 | 0,11 | 372 868 km² |
| Lucane cerf-volant (*Lucanus cervus*) | *Supported* | 0,73 | 0,14 | 378 774 km² |
| Hêtre (*Fagus sylvatica*) | *Indicative* | 0,90 | 0,19 | 330 204 km² |
| Crapaud épineux (*Bufo spinosus*) | *Indicative* | 0,62 | 0,06 | 289 513 km² |
| Grenouille agile (*Rana dalmatina*) | *Indicative* | 0,60 | 0,11 | 290 522 km² |
| Salamandre tachetée (*Salamandra salamandra*) | *Exploratory* | 0,32 | 0,09 | 428 067 km² |
| Hérisson d'Europe (*Erinaceus europaeus*) | *Exploratory* | 0,27 | 0,06 | 408 759 km² |
| Vipère aspic (*Vipera aspis*) | *Exploratory* | 0,23 | 0,10 | 469 518 km² |
| Lézard à deux raies (*Lacerta bilineata*) | *Exploratory* | 0,17 | 0,05 | 431 716 km² |

Trois leçons :

1. **Les espèces à niche climatique marquée s'en sortent bien ; les généralistes, non.** Le chêne vert est méditerranéen et le climat l'explique. Le hérisson ou la vipère vivent presque partout en France : le climat à 1 km ne permet pas de dire où ils sont, et l'outil ne le cache pas. Il leur faut d'autres variables (occupation du sol, §3.3) ou des données de terrain.
2. **Une seule bonne note ne suffit pas.** Le hêtre a le meilleur indice de Boyce (0,90), mais son OR10 (0,19) montre que, lorsqu'on cache une région entière, le modèle laisse de côté trop d'observations : il a du mal à se transposer à des climats qu'il n'a pas vus. C'est pourquoi il s'arrête à *Indicative*.
3. **La quantité de données change le résultat.** En passant de 20 000 données à toutes les données disponibles (avec les téléchargements GBIF), l'indice de Boyce du hérisson est passé de 0,27 à 0,80 et celui du lézard à deux raies de 0,17 à 0,70 (`docs/benchmarks/gbif-batch-downloads.md`). Plus de données, c'est aussi plus de biais d'observation : c'est pourquoi les avertissements sur le biais restent affichés.

Les surfaces sont des surfaces de **conditions favorables**, pas de présence : que le lézard à deux raies dispose de 431 716 km² de conditions favorables ne veut pas dire qu'il occupe toute cette surface.

---

## 6. Ce que livre NicheRS

À la fin d'une étude, l'utilisateur obtient :

- la **carte d'idonéité relative**, avec des couches d'incertitude et d'extrapolation ;
- la **fourchette de surface favorable** ;
- le **niveau de preuve** avec ses motifs et tous les avertissements (les avertissements ne peuvent jamais être masqués) ;
- l'**importance de chaque variable** et les **courbes de réponse** (« à partir de quelle température elle s'y plaît moins ») ;
- un **rapport de méthodes** imprimable, avec des références bibliographiques ;
- des exports (GeoTIFF, GeoJSON, le modèle au format `.lambdas` de MaxEnt) ;
- un **manifeste d'exécution** : un fichier qui consigne exactement quelles données (avec leurs sommes de contrôle et le DOI du téléchargement GBIF), quelles versions, quelle graine aléatoire et quels réglages ont été utilisés. Grâce à lui, n'importe qui peut refaire l'étude et obtenir le même résultat, bit à bit.

---

## 7. Comment nous en sommes arrivés là

### Phase 0 — Se documenter avant de programmer (15 oct. 2026)

Avant d'écrire du code, un état de l'art a été réalisé (14 documents dans `docs/research/`) : quelles méthodes fonctionnent, quels outils existent (MaxEnt, biomod2, ENMeval, Wallace…), quelles données peuvent être utilisées légalement, quel langage choisir. Principales conclusions :

- MaxEnt reste une référence solide ; le second moteur devrait être un *Random Forest* équilibré (voir phase 1b).
- Les principaux risques **ne sont pas techniques** : ce sont le biais des données, le fait que les gens lisent l'idonéité comme une probabilité, l'extrapolation silencieuse et les licences des données.
- Des « portes » (*gates*) mesurables ont été posées : si le moteur en Rust ne reproduisait pas `maxent.jar`, le plan B était d'utiliser `maxent.jar` lui-même.

Le jour même, le portage de MaxEnt a été démontré (11/11 cas synthétiques identiques ; puis 16/16 avec des espèces réelles).

### MVP — Le produit minimum complet (15–16 oct.)

Toute la chaîne a été construite : téléchargement GBIF et CHELSA, nettoyage, modèle, validation, cartes, rapport, lots de nombreuses espèces, tâches qui reprennent en cas de coupure de courant. Les choix scientifiques par défaut (nettoyage, sélection, niveaux de preuve) ont été fixés dans l'ADR-0009. Les 14 critères de « terminé » du MVP ont été remplis (`docs/mvp-status.md`).

Mesures réelles : une étude complète d'une espèce sur toute la France à 1 km prend **5–12 s** sur un ordinateur portable à 8 cœurs. Un lot de 10 espèces est passé de **22,5 min** (API de recherche GBIF) à **3 min 10 s** sur le serveur une fois les données GBIF préparées (le premier téléchargement dépend de la file d'attente du GBIF). Pour 100 espèces, il n'y a qu'une extrapolation (\~25 min de modélisation), **pas une mesure**.

### Un produit utilisable par d'autres (16–19 oct.)

Ce qui a transformé le prototype en un outil qu'une équipe peut utiliser, sans changer la science :

- comptes, organisations, projets et quotas ; console d'administration ;
- interface et aide en **anglais, français et espagnol** ;
- deux parcours pour créer une étude : **Rapide** (trois étapes, sans connaissances en modélisation) et **Expert** ; une lecture simplifiée des résultats pour les non-spécialistes ;
- zones d'étude par nom (communes, départements, espaces protégés officiels) ;
- un **miroir GBIF régional** pour la France et l'Espagne : les données arrivent en quelques secondes au lieu d'attendre la file d'attente du GBIF (ADR-0114). Ce miroir conserve en outre le nombre d'observations de chaque groupe d'espèces dans chaque cellule — ce qui sert ensuite à corriger le biais (§8) ;
- confidentialité (pas de noms d'observateurs en cache), déploiements sans interruption, rapports d'erreurs.

### Revue scientifique (20 oct.)

Le 20 octobre, le code a été passé en revue et l'on a constaté que **le cœur scientifique n'avait pas changé depuis le MVP**, alors que ses faiblesses pour un usage professionnel étaient précisément scientifiques :

1. le **biais d'échantillonnage** n'est pas corrigé (on modélise en partie l'endroit où les gens regardent) ;
2. seul le **climat** est utilisé (ni relief, ni végétation, ni eau) ;
3. il n'y a pas de **validation indépendante** ;
4. la sortie est une carte par espèce, pas une réponse à la **décision du client** (où échantillonner ? ce projet a-t-il un impact ?).

De là est né le programme **Science v1** (`docs/science-v1-plan.md`), dans lequel nous nous trouvons aujourd'hui.

---

## 8. Science v1 : ce qui a été ajouté

### Règles du programme

- **Les valeurs par défaut ne changent pas.** Chaque nouvelle méthode est **optionnelle** (mode Expert). Une étude avec les réglages par défaut donne des résultats **identiques bit à bit** à ceux d'avant ; nous l'avons vérifié en comparant les fichiers.
- **On ne touche pas au moteur MaxEnt.** L'équivalence avec `maxent.jar` reste à 100 %.
- Chaque nouvelle méthode a un ADR au statut **« Proposé — en attente de revue par les écologues de Natural Solutions »**. En faire une valeur par défaut sera une décision ultérieure, après cette revue.

### Phases

| Phase | De quoi il s'agit | État |
| --- | --- | --- |
| 1a | Briques d'évaluation : filtrage spatial par distance, blocs en grille, autocorrélation spatiale, domaine d'applicabilité, modèles nuls, métriques de présence–absence | Terminée |
| 1b | Second moteur : *Random Forest* équilibré | Terminée |
| 1c | Données : marqueurs de coordonnées floutées, grilles d'effort d'échantillonnage, groupe taxonomique, climat futur CHELSA CMIP6 | Terminée |
| 1d | Prédicteurs : relief (Copernicus DEM), occupation du sol (ESA WorldCover), distance à l'eau, changements de projection | Terminée |
| 2 | **Biais et nettoyage** : coordonnées floutées, fond par groupe cible, couche d'effort, filtrage spatial | Terminée (ADR-0011, ADR-0013) |
| 3 | **Évaluation** : modèles nuls, blocs selon l'autocorrélation, domaine d'applicabilité | Terminée (ADR-0014) |
| 4 | **Prédicteurs** : relief, occupation du sol et eau comme variables sélectionnables (§3.3) | Terminée (ADR-0015) |
| 5 | **Climat futur** : projections, cartes d'évolution et accord entre modèles climatiques | Terminée (ADR-0016) |
| 6 | **Second moteur dans l'application** : Random Forest et ensemble MaxEnt + RF | Terminée (ADR-0017) |
| 7 | **Données de terrain propres** : import, validation indépendante et niveau *Validated* | Terminée (ADR-0108) |
| 8 | **Aide à la décision** : priorité de prospection, emprise de projet, plusieurs espèces, comparaison d'études | Terminée (ADR-0018) |
| 9 | Documentation : état du MVP, synthèse, feuille de route, note en français pour les écologues | Terminée |

### Les nouvelles méthodes, expliquées

**Modèles nuls (phase 3).** Le modèle fait-il mieux que le hasard ? On entraîne le même modèle, avec les mêmes réglages, sur des points placés **au hasard** (20, 50 ou 100 fois) et on compare : si le modèle réel ne dépasse pas les modèles aléatoires dans au moins 95 % des cas, le niveau de preuve descend à *Exploratory*. C'est comme vérifier qu'une voyante devine mieux que quelqu'un qui répond au hasard.

**Blocs selon l'autocorrélation (phase 3).** Les 4 blocs à la médiane ont une taille arbitraire. Le plus juste est que les blocs soient au moins aussi grands que la distance à laquelle le climat « se ressemble à lui-même » (l'**autocorrélation spatiale**). Désormais, la taille peut être calculée à partir des données climatiques elles-mêmes (variogramme) et le territoire est découpé en une grille de blocs de cette taille.

**Domaine d'applicabilité (AOA, phase 3).** L'extrapolation MESS examine chaque variable séparément. L'AOA examine aussi les **combinaisons nouvelles** : un site peut avoir une température normale et une pluviométrie normale, mais une combinaison des deux qui n'apparaît dans aucune donnée (par exemple, de la chaleur avec beaucoup de pluie). Il s'affiche comme une couche supplémentaire de la carte.

**Coordonnées floutées (phase 2, ADR-0011).** Les espèces sensibles sont publiées avec une position volontairement déplacée (iNaturalist jusqu'à \~27 km, PatriNat 5 km). Auparavant, si la donnée n'indiquait pas son incertitude, elle était utilisée comme si elle était exacte. Désormais, si le fournisseur indique qu'il l'a floutée sans dire de combien (ou en indiquant plus de 1 km), elle est exclue. Limite : seule l'API de recherche GBIF fournit cette information ; les téléchargements en masse et le miroir régional ne la fournissent pas, et l'outil le signale. Testé sur des données réelles : dans une étude de la tortue d'Hermann (*Testudo hermanni*) en Provence, 339 données sur 1 509 étaient marquées comme floutées ; l'outil les a comptées, a averti et a expliqué le motif de chaque exclusion, sans changer le nombre de présences ni le niveau de preuve dans ce cas.

**Fond par groupe cible (phase 2, ADR-0013).** C'est la correction du biais d'échantillonnage la plus recommandée dans la littérature (Phillips et al., 2009). Au lieu de comparer les observations à des cellules tirées au hasard, on les compare à des cellules **où d'autres espèces du même groupe ont été observées** (par ex. d'autres oiseaux si l'on étudie un oiseau). Ainsi, présences et fond ont le même biais (« les gens se promènent près des villes ») et ce biais s'annule en partie. Les données proviennent des grilles d'effort du miroir régional. S'il n'y a pas assez de données, on revient au fond aléatoire **avec un avertissement** : une étude n'échoue jamais parce que la correction n'est pas disponible.

**Filtrage spatial par distance (phase 2).** S'il y a 200 observations dans un même parc et 5 dans le reste de la région, le modèle apprend « le parc ». Le filtrage ne garde que des observations séparées d'au moins une distance donnée (par ex. 5 km).

**Random Forest équilibré (phases 1b et 6, ADR-0017).** Une seconde méthode, très différente de MaxEnt : des milliers d'« arbres de décision » (« si la température \> 12 °C et la pluie \< 800 mm, alors… ») qui votent. La version par défaut fonctionne mal avec des données de présence seule ; la version **équilibrée** (chaque arbre voit autant de points de fond que de points de présence) figure parmi les meilleures méthodes dans les comparaisons publiées (Valavi et al., 2022). Comme il n'existe pas de programme de référence que l'on puisse copier bit à bit, elle est validée avec des espèces synthétiques dont on connaît la niche réelle : le modèle retrouve les variables importantes et les bons optimums. Disposer de deux méthodes différentes permet aussi de voir **où elles divergent**, ce qui est une information utile.

Depuis la phase 6, le mode Expert permet de choisir l'algorithme : **MaxEnt** (recommandé, par défaut), **Random Forest** ou un **ensemble** des deux, qui fait la moyenne de leurs classements des cellules. Les trois passent exactement le même examen (mêmes blocs, mêmes notes, modèles nuls, extrapolation, niveau de preuve), ils sont donc comparables. Au lieu des courbes de réponse, le Random Forest affiche la *dépendance partielle*, qui se lit de la même façon (« comment l'idonéité change quand la température varie »). Sur la démo synthétique, avec des blocs spatiaux, l'indice de Boyce a été de 0,53 pour MaxEnt, 0,47 pour le Random Forest et 0,52 pour l'ensemble ; l'étude a pris 5,0 s, 2,7 s et 7,9 s. Ce n'est qu'un essai sur données synthétiques : **aucune comparaison n'a encore été faite sur des espèces réelles**. On sait déjà que les arbres extrapolent moins bien que MaxEnt hors des conditions observées, et que le résultat du Random Forest varie un peu selon la graine aléatoire (Boyce 0,51 contre 0,47 avec deux graines sur la démo), ce que l'outil ne montre pas encore à l'utilisateur.

**Relief, végétation et eau (phase 4, ADR-0015).** Expliqué au §3.3. Testé sur un découpage réel de la Provence (environ 1 000 km²) : préparer les couches prend environ 12 s la première fois et quelques centièmes de seconde ensuite, grâce au cache. Ce qui **n'a pas encore été mesuré**, c'est dans quelle mesure les modèles d'espèces réelles s'améliorent quand on ajoute l'occupation du sol ; c'est la prochaine vérification à faire.

**Climat futur (phase 5, ADR-0016).** Le modèle ajusté est appliqué au climat futur de CHELSA CMIP6 : trois périodes (2011–2040, 2041–2070, 2071–2100), trois scénarios d'émissions (de faibles à très fortes) et jusqu'à cinq modèles climatiques différents ; il est recommandé d'utiliser les cinq, car l'écart entre eux fait partie de la réponse. Seul le climat change : le relief, la végétation et l'eau restent tels qu'aujourd'hui, et l'outil le signale. Cela fonctionne avec les trois algorithmes. Pour chaque scénario, on obtient :

- une **carte d'évolution** à quatre classes : favorable aujourd'hui et demain, **nouvellement favorable**, **qui cesse d'être favorable** et défavorable dans les deux cas, avec le même seuil qu'aujourd'hui (celui à 10 %) pour que la comparaison soit juste. Sur la carte, les zones où la majorité des modèles climatiques ne sont pas d'accord apparaissent en pâle ;
- **combien de modèles climatiques sont d'accord** dans chaque cellule, et la surface sur laquelle **tous** le sont : si les cinq disent qu'une zone est perdue, la conclusion est plus solide que si un seul le dit ;
- la **dispersion entre modèles** et l'**extrapolation future** (hachurée sur la carte) : les zones dont le climat futur ne ressemble à rien de ce que le modèle a vu. Au-delà de 10 % de la zone, un avertissement s'affiche ;
- la surface de chaque classe, avec la fourchette entre modèles climatiques, dans un onglet « Futur » des résultats et dans le rapport.

Coût mesuré : sur la démo, projeter un scénario avec cinq modèles climatiques ajoute 0,2 s à une étude avec MaxEnt et 2,2 s avec l'ensemble. Lire les couches futures de CHELSA pour un découpage réel de la Provence a pris 1,8 s. Ce qui **n'a pas encore été fait** : projeter une espèce réelle de bout en bout (l'essai réel a utilisé une espèce virtuelle sur des couches réelles), une étude à l'échelle nationale, et confirmer auprès de CHELSA la licence de ses couches futures (l'attribution est donnée de toute façon). Le niveau de preuve porte sur le modèle actuel ; **il ne couvre pas les projections**.

**Données de terrain propres et validation indépendante (phase 7, ADR-0108).** Les bureaux d'études ont leurs propres inventaires, souvent en Lambert-93 et souvent avec des **absences** : des sites visités où l'espèce n'a pas été vue. On peut désormais les importer sous forme de fichier CSV. Le navigateur le lit et reconnaît les séparateurs, les virgules décimales, les dates et une liste fermée de systèmes de coordonnées (WGS84, Lambert-93, LAEA Europe et UTM). Les données appartiennent à l'organisation : aucune autre organisation ne peut les voir, et ni le manifeste ni les rapports ne contiennent leurs coordonnées.

Pour chaque espèce, on choisit comment les utiliser :

- **GBIF + présences propres** pour l'entraînement ;
- **présences propres seules**, sans interroger GBIF ;
- **GBIF pour l'entraînement et données propres mises de côté pour la vérification**. C'est l'option qui ouvre l'accès au niveau *Validated*.

La vérification confronte la carte aux sites de terrain : obtient-elle un score plus élevé là où l'espèce a été trouvée que là où elle ne l'a pas été ? On calcule l'**AUC présence–absence** (qui a ici du sens, puisqu'il y a de vraies absences), la **sensibilité** (la part des présences reconnues), la **spécificité** (la part des absences reconnues) et le **TSS**, qui combine les deux. Les seuils sont fixés sur les données d'entraînement, jamais ajustés sur les données de vérification. Une règle empêche de tricher : un site de vérification situé dans une cellule déjà utilisée pour l'entraînement est écarté, car le modèle connaît déjà la réponse (c'est le cas, par exemple, d'observations du SINP également publiées dans GBIF).

Testé avec un inventaire **synthétique** de l'espèce de la démo, en Lambert-93 : il restait 105 sites de présence et 193 d'absence, et 2 ont été écartés par la règle ci-dessus. Résultat : AUC 0,84 et TSS 0,53, mais le niveau est resté *Indicative*, car le modèle ne remplissait pas tous les critères de *Supported* : apporter des données de terrain ne donne pas le niveau d'office. **Aucun essai n'a encore été fait avec un inventaire réel de client** (il est interdit d'en inclure dans le dépôt). Le fait qu'une absence puisse être une espèce présente mais non vue n'est pas non plus modélisé.

**Aide à la décision (phase 8, ADR-0018).** Une carte par espèce ne répond pas aux questions du client. Un nouvel onglet des résultats, « Aide à la décision », calcule à la demande quatre sorties à partir d'une étude terminée, sans réajuster aucun modèle ni toucher à ses fichiers :

- **Priorité de prospection** : les zones qui méritent une visite parce qu'elles sont favorables et peu échantillonnées (d'après l'effort du groupe cible ou, à défaut, la distance à la donnée connue la plus proche). Des zones candidates classées sont proposées. Cela ne dit pas où est l'espèce.
- **Emprise de projet** : on importe ou on dessine le contour d'un projet (parc éolien, route) et on obtient, avec des surfaces exactes, la surface favorable touchée pour chaque espèce, la part de l'emprise que cela représente, et si cette zone est en extrapolation.
- **Plusieurs espèces** : combien d'espèces trouvent des conditions favorables dans chaque cellule et où elles se concentrent (les « zones à enjeu », les 10 % de cellules qui en comptent le plus). Seules les espèces au niveau *Indicative* ou supérieur sont prises en compte, avec un avertissement : additionner des cartes tend à surestimer.
- **Comparer avec…** : deux études de la même espèce sur la même zone (par exemple avec et sans correction du biais), avec une carte des différences et des mesures de ressemblance.

Toutes portent l'avertissement qu'elles **ne remplacent ni un inventaire de terrain ni une étude d'impact**, et qu'une faible idonéité n'est pas une preuve d'absence. Chaque analyse conserve sa propre trace, liée à celle de l'étude. Testé avec deux espèces virtuelles de la démo ; **pas testé sur des espèces réelles**.

### Après Science v1

Le programme est achevé, mais **aucune nouvelle option n'a été validée**. L'étape suivante n'est pas de programmer davantage, mais :

1. **La revue par les écologues de Natural Solutions** de chaque ADR proposée (et de l'ADR-0009, qui n'a pas été revue non plus) : quel groupe d'espèces prendre comme référence du biais, les seuils des niveaux de preuve, les critères de *Validated*, le calcul de la priorité de prospection… La note the design notes rassemble ces questions.
2. **Des essais sur des espèces réelles** que les écologues connaissent : tous les chiffres de Science v1 viennent de la démo synthétique, de tests synthétiques ou de petits découpages de la Provence.
3. **La confirmation de deux licences** : les couches futures de CHELSA et le modèle d'élévation Copernicus.
4. Seulement ensuite, faire de ce qui est approuvé un réglage par défaut, par une nouvelle ADR.

---

## 9. Comment nous travaillons pour ne pas nous tromper

Un tel outil peut produire des cartes belles et fausses. Voici les règles qui l'empêchent (`AGENTS.md`) :

- **La justesse scientifique passe avant tout**, y compris la vitesse et le confort d'utilisation.
- **Chaque décision scientifique est écrite** dans un ADR, avec sa justification bibliographique et son coût connu. Aucune valeur par défaut ne change « sans le dire ».
- **Le nouveau reste optionnel tant que des écologues ne l'ont pas revu.** Et l'on vérifie que, sans rien activer de nouveau, les résultats restent identiques, fichier par fichier.
- **Le moteur est comparé à l'original** à chaque modification ; on n'assouplit jamais une tolérance pour faire passer un test.
- **Aucun avertissement n'est masqué** pour simplifier l'interface (qualité des données, extrapolation, échantillons trop petits, biais).
- **Les données de démonstration sont synthétiques et le disent** ; elles ne sont jamais présentées comme réelles.
- **On n'annonce pas de vitesses sans les mesurer**, et chaque phase se termine par une liste de ce qui a été vérifié et de ce qui ne l'a **pas** été.

---

## 10. Questions fréquentes

**La carte dit-elle où se trouve l'espèce ?** Non. Elle dit où les conditions ressemblent à celles des sites où l'espèce a été vue. L'espèce peut être absente d'une zone favorable (elle n'y est pas arrivée, la chasse, la concurrence) et apparaître occasionnellement dans une zone défavorable.

**Pourquoi ne donne-t-elle pas un pourcentage de fiabilité ?** Parce que tout pourcentage serait inventé : avec des données de présence seule, il n'existe aucune manière honnête de le calculer. À la place, le niveau de preuve indique quels critères sont satisfaits et lesquels ne le sont pas.

**Mon espèce sort « Exploratory ». L'étude est-elle mauvaise ?** Pas nécessairement. Cela signifie généralement peu de données, une espèce généraliste que le climat explique mal, ou une grande part de zone extrapolée. Les motifs s'affichent à côté du niveau. Ajouter l'occupation du sol, corriger le biais ou apporter des données de terrain peut l'améliorer ; forcer le résultat, non.

**Pourquoi ne pas utiliser directement** **`maxent.jar` ?** Parce qu'il exige Java, s'intègre mal dans un service web et est plus lent quand on enchaîne des centaines d'ajustements. Nous continuons à l'utiliser comme **arbitre** : le moteur de NicheRS doit donner exactement les mêmes chiffres que lui.

**MaxEnt ou Random Forest ?** Par défaut, MaxEnt : c'est la méthode vérifiée contre la référence et celle qui se comporte le mieux dans des zones nouvelles. Le Random Forest et l'ensemble sont des options pour experts, utiles pour voir si deux méthodes très différentes arrivent à la même conclusion ; si elles divergent, c'est un signe qu'il faut rester prudent.

**La carte future dit-elle où vivra l'espèce en 2070 ?** Non. Elle dit où se trouveront des conditions climatiques semblables à celles d'aujourd'hui là où l'espèce est observée, selon chaque modèle climatique. Elle ne sait pas si l'espèce pourra y arriver, ni comment évolueront l'occupation du sol ou les autres espèces. Et là où le climat futur ne ressemble à rien de connu (zones hachurées), le modèle devine.

**Qu'est-ce que j'y gagne à apporter mes propres données de terrain ?** Deux choses. Davantage de présences pour l'entraînement, souvent de meilleure qualité que celles de GBIF. Et surtout, si on les met de côté pour la vérification, un test indépendant de la carte, seul moyen d'amener un résultat au niveau *Validated*. Les absences (sites visités où l'espèce n'a pas été trouvée) sont ce qu'il y a de plus précieux.

**Puis-je déjà utiliser les nouvelles options dans un rapport client ?** Pas encore. Elles sont implémentées et vérifiées techniquement, mais ni revues par des écologues ni testées sur des espèces réelles. Les réglages par défaut (ceux du MVP) sont ceux que l'on peut utiliser aujourd'hui.

**Puis-je utiliser les résultats dans un rapport pour un client ?** Les données par défaut sont sous licence commerciale et les données « non commerciales » sont exclues. Le rapport inclut les méthodes, les sources et les références. Pour les espèces sensibles, il faut consulter le référentiel régional avant de publier des points (le rapport le rappelle).

**Pourquoi tant d'options sont-elles « en attente de revue » ?** Parce que ce sont des méthodes reconnues dans la littérature, mais leur réglage précis (seuils, groupe d'espèces de référence, distances) doit être validé par un écologue de Natural Solutions avant d'en faire une valeur par défaut.

---

## 11. Ce que NicheRS ne fait pas (encore) — limites honnêtes

- Uniquement les espèces **terrestres** ; le climat est la moyenne 1981–2010.
- Par défaut, les modèles utilisent **uniquement le climat** ; relief, végétation et eau sont optionnels et leur effet sur des espèces réelles n'a pas encore été mesuré.
- La correction du biais par groupe cible n'a été testée que hors ligne (données de test et démonstration synthétique), pas avec le vrai miroir régional.
- Le Random Forest et l'ensemble n'ont été testés que sur la démo synthétique, pas sur des espèces réelles.
- Les projections au climat futur n'ont pas encore été testées de bout en bout sur une espèce réelle, n'incluent ni dispersion ni changement d'occupation du sol, et la licence des couches futures de CHELSA reste à confirmer.
- **Aucune méthode nouvelle de Science v1 n'a été utilisée de bout en bout sur une espèce réelle.** Seule exception : la règle des coordonnées floutées.
- Les sorties d'aide à la décision n'ont été testées que sur des espèces virtuelles, et leurs avertissements ne s'affichent qu'en anglais dans l'interface.
- La validation indépendante et le niveau *Validated* n'ont été testés qu'avec des données de terrain synthétiques ; l'import n'accepte que le CSV (pas les exports GeoNature, Excel, GeoPackage ni Shapefile).
- Les corrections de biais sont optionnelles et **en attente de revue** par les écologues de Natural Solutions, tout comme les seuils des niveaux de preuve (0,40 / 0,70 pour Boyce, etc.), qui sont des décisions raisonnées mais discutables.
- Les vitesses citées ont été obtenues sur des machines précises et ne s'extrapolent pas sans mesure.
- Le résultat est toujours une **idonéité relative** : il ne dit pas où se trouve l'espèce, mais où les conditions ressemblent à celles des sites où elle a été vue.

---

## 12. Glossaire

| Terme | Signification |
| --- | --- |
| **SDM** | Modèle de distribution d'espèces : estime où se trouvent des conditions favorables à une espèce. |
| **Niche écologique** | Ensemble des conditions environnementales dans lesquelles une espèce peut vivre. |
| **GBIF** | Base de données mondiale d'observations d'espèces. |
| **Présence / présence seule** | Observation de l'espèce en un lieu. « Présence seule » : on ne sait pas où l'espèce a été cherchée sans succès. |
| **Présence–absence** | Données de terrain où l'on note aussi si l'espèce *n'était pas* là. Elles permettent une validation indépendante. |
| **Sensibilité / spécificité** | La part des présences / des absences de terrain que la carte reconnaît. |
| **TSS** | Sensibilité + spécificité − 1. 0, c'est le hasard ; 1, c'est parfait. |
| **Validated** (validé) | Niveau de preuve le plus élevé : la carte réussit une vérification sur des données de terrain indépendantes. |
| **Priorité de prospection** | Zones favorables et peu échantillonnées, où il vaut la peine de chercher en premier. |
| **Emprise de projet** | Contour d'un projet (parc éolien, route…) croisé avec les cartes. |
| **Zone à enjeu** | Zone où de nombreuses espèces trouvent en même temps des conditions favorables. |
| **CHELSA** | Cartes climatiques mondiales à 1 km, dans le domaine public. |
| **Scénario d'émissions (SSP)** | Hypothèse sur la quantité de gaz à effet de serre émise ; de faible (SSP1-2.6) à très forte (SSP5-8.5). |
| **Modèle climatique (GCM)** | Simulation du climat de la Terre ; des modèles différents donnent des futurs un peu différents, et cet écart fait partie de l'incertitude. |
| **Copernicus DEM / ESA WorldCover** | Cartes européennes du relief (90 m) et de l'occupation du sol (10 m), utilisables commercialement. |
| **Occupation du sol en proportions** | Part de chaque cellule occupée par chaque type de couverture (forêt, cultures…). |
| **Variable bioclimatique (BIO1…)** | Résumé du climat utile pour la biologie (température moyenne, pluie du mois le plus sec…). |
| **Fond (background)** | Échantillon de cellules de la zone auquel on compare les présences. |
| **MaxEnt** | Méthode de modélisation de référence pour les données de présence seule. |
| **Régularisation (β)** | Pénalisation de la complexité du modèle pour éviter le surajustement. |
| **Surajustement** | Le modèle mémorise les données (bruit compris) au lieu d'apprendre la règle générale. |
| **Idonéité relative** | Score de 0 à 1 indiquant à quel point les conditions sont favorables. Ce n'est pas une probabilité. |
| **Validation croisée** | Cacher une partie des données, entraîner le modèle sur le reste et vérifier s'il voit juste sur la partie cachée. |
| **Blocs spatiaux** | Cacher des zones géographiques entières, pour que l'examen ne soit pas trop facile. |
| **OR10** | Proportion d'observations cachées que le modèle laisse de côté avec le seuil des 10 %. Idéal ≈ 0,10. |
| **Indice de Boyce** | Mesure si les zones bien notées ont plus d'observations que ce qu'on attendrait du hasard. De −1 à 1. |
| **AUC** | Métrique classique ; en présence seule, elle dépend surtout de la taille de la zone. Purement informative. |
| **MESS** | Détecte les zones où une variable prend des valeurs hors de la gamme des données (extrapolation). |
| **AOA** | Domaine d'applicabilité : détecte aussi les combinaisons de variables jamais vues. |
| **Modèle nul** | Modèle entraîné sur des points aléatoires, pour vérifier que le modèle réel fait mieux que le hasard. |
| **Autocorrélation spatiale** | Ce qui est proche se ressemble : deux sites distants de 1 km ont presque le même climat. |
| **Biais d'échantillonnage** | Il y a plus de données là où les gens regardent davantage, pas là où l'espèce est plus présente. |
| **Fond par groupe cible** | Fond tiré là où des espèces proches ont été vues, pour qu'il ait le même biais que les présences. |
| **Filtrage spatial (thinning)** | Retirer les observations trop proches les unes des autres. |
| **Random Forest** | Méthode fondée sur de nombreux arbres de décision qui votent ; second moteur de NicheRS. |
| **Niveau de preuve** | Résumé par règles (Insufficient / Exploratory / Indicative / Supported) du degré de confiance à accorder au résultat. |
| **Manifeste d'exécution** | Enregistrement complet des données, des versions et des réglages permettant de refaire l'étude. |
| **ADR** | *Architecture Decision Record* : document qui consigne par écrit une décision, ses raisons et son coût. |

## Pour en savoir plus

- Vue d'ensemble : `docs/executive-summary.md`
- Choix scientifiques par défaut : `docs/architecture/architecture-decisions/ADR-0009-ecological-defaults-and-evidence-levels.md`
- Comment la confiance est communiquée : `ADR-0007-output-semantics-and-confidence.md`
- Programme Science v1 : `docs/science-v1-plan.md` (synthèse en tête) et les ADR 0011, 0013, 0014, 0015, 0016, 0017, 0018 et 0108
- Note pour les écologues de Natural Solutions : the design notes
- État de l'art et bibliographie : `docs/research/01-sdm-state-of-the-art.md`, `docs/research/14-references.md`
- Équivalence avec MaxEnt : `docs/benchmarks/phase3-results.md`
