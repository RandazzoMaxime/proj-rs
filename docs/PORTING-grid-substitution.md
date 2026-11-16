# PORTING: substitution des noms de grille PROJ

Module : `src/iso19111/proj_grid_substitution.rs`, port de
`SingleOperation::substitutePROJAlternativeGridNames` (`src/iso19111/operation/singleoperation.cpp:2402`)
et de `PointMotionOperation::substitutePROJAlternativeGridNames` (`singleoperation.cpp:5591`),
appelés par `createCoordinateOperation` quand `usePROJAlternativeGridNames` est vrai
(`factory.cpp:6766`, `6776`, `6953`).

Une opération de grille du registre dont le fichier a une alternative dans `grid_alternatives`
(`lookForGridAlternative`) devient une nouvelle opération :
`createSimilarPropertiesOperation` (`singleoperation.cpp:2305`) garde le nom, les remarques et les
domaines, écrit les identifiants `DERIVED_FROM(<autorité>)` et ne garde ni la version ni les
époques ; `createSimilarPropertiesMethod` (`singleoperation.cpp:2335`) ne garde du méthode que le
nom et l'identifiant.

## Branches portées

| Branche de PROJ | Méthodes EPSG | Paramètres de la nouvelle opération |
|---|---|---|
| horizontale (`2411-2530`) | 9613, 9614, 9615, 1074, 1075 | le seul fichier GTiff |
| géoïde (`2540-2577`) | `Geographic3D to GravityRelatedHeight` | le seul fichier |
| géocentrique IGN (`2599-2636`) | 1087 | le seul fichier |
| vitesse géographique 3D, vitesse verticale (`2638-2714`) | 1114, 1113 | le seul fichier `Point motion velocity grid file` |
| VERTCON et grilles verticales régulières (`2716-2777`) | 9658, 1071, 1080, 1084, 1085, 1101, 1112, 1126, 1129 | le seul fichier |
| table `gridTransformations` (`2779-2856`) | 1079 (déformation NZ), 1137, 1138, 1145 (TIN JSON) | le seul fichier |
| position vector ou translations avec vitesses NEU (`2858-2917`) | 1143, 1144 | tous les paramètres d'origine, le fichier de vitesse remplacé sur place |
| translations par grille et vitesses NEU (`2919-2992`) | 1142 | tous les paramètres d'origine, les deux fichiers remplacés sur place ; les deux doivent avoir une alternative |
| mouvement de point (`5591-5629`) | 1070, 1141, 1139 | le seul fichier |

## Ordre des paramètres de `other_transformation`

PROJ ajoute le paramètre de fichier de grille d'une ligne `other_transformation` **après** les
paramètres numériques (`factory.cpp:6872`), alors que `grid_transformation` le met avant
(`factory.cpp:6664-6683`). Le port écrivait le fichier en tête pour les deux tables ; la différence
se voit sur les 7 opérations de méthode 1143.

## Écarts volontaires

- Les sens inverses de `grid_alternatives` (`inverse_direction = 1`) ne sont pas portés pour les
  branches de vitesse, du géoïde, de 1087 et de la table `gridTransformations` : PROJ lève
  `UnsupportedOperationException` ou construit l'inverse, et aucune ligne de `proj.db` 9.9.0 n'en
  a un pour ces grilles. L'appelant garde alors le renommage de fichier.
- Les formats `NTv1`, `NTv2` et `CTable2` de la branche horizontale ne sont pas portés (aucune
  ligne de `proj.db` ne les a).

## Tests

- `tests/unit_op_grid_substitution_velocity.rs` : les méthodes 1142, 1143, 1144, 1079, 1113 et
  1114, valeurs de `projinfo` 9.9.0 ; échouent sans la substitution et sans l'ordre ci-dessus.
- `tests/unit_op_nadcon5_gtiff_export.rs` : branche horizontale NADCON5.
- Corpus `tests/parity/export` (voir `docs/PARITY.md`, « Exports d'objets »).
