# Portage `cct`

Binaire inchangé : `cct` (`src/bin/cct.rs`).

Référence lue avant le code : PROJ 9.9.0 `src/apps/cct.cpp` et
`test/cli/test_cct.yaml` (cas `-d`, `-z`, `-t`, `-o`, pipeline `+proj`).

Aucun nouveau module Rust. Fichiers :

- `src/bin/cct.rs` (étendu)
- `tests/cli.rs` (stdout YAML, dont `-o`)
- `PORT-NOTES-cct.md` (ce fichier)

`src/lib.rs` et `tests/gie/passing.txt` ne sont pas modifiés.

`-d` fixe les décimales de x, y et z (10 et 4 sinon : angles plus larges,
distances en `%13`). Le temps reste `%12.4f`. Une ordonnée absente vaut
l'infini. `-z` et `-t` remplissent les colonnes 3 et 4 sans les décaler.
`-o` écrit les mêmes lignes dans un fichier ; stdout reste vide.
Un pipeline `+proj=unitconvert` réaffiche les radians en degrés, et laisse
une sortie déjà en degrés telle quelle.
