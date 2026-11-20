#!/usr/bin/env bash
# The report of a regeneration (tools/proj-db/update.sh), as the Markdown body of an issue: what
# changed in the database, what the test suite says against the binaries of the new release, and
# what is left to do by hand (docs/PROJ-UPSTREAM.md). The workflow proj-upstream.yml posts it.
#
# Usage: tools/proj-db/report.sh <tag> <current> <reference dir> <update status> <tests log> [branch]
#   <update status>  exit status of update.sh (0 replaced or same, 3 layout changed, other failed)
#   <tests log>      output of `cargo test --no-fail-fast`, or "-" when the tests did not run
#   [branch]         the branch the new database was pushed to, if any
set -uo pipefail

tag=$1 current=$2 dir=$3 update_status=$4 tests_log=$5 branch=${6:-}
root=$(cd "$(dirname "$0")/../.." && pwd)
repo=${GITHUB_REPOSITORY:-RandazzoMaxime/proj-rs}
server=${GITHUB_SERVER_URL:-https://github.com}

echo "PROJ **$tag** est publiée ; \`data/proj.db\` vient de PROJ **$current**."
echo
echo "- Version : https://github.com/OSGeo/PROJ/releases/tag/$tag"
echo "- Ce qui change dans PROJ : https://github.com/OSGeo/PROJ/blob/$tag/NEWS.md"
echo "- Différences de sources : https://github.com/OSGeo/PROJ/compare/$current...$tag"
[ -n "${GITHUB_RUN_ID:-}" ] && echo "- Exécution du workflow : $server/$repo/actions/runs/$GITHUB_RUN_ID"
echo

echo "## 1. Base de données"
echo
case $update_status in
  0)
    if [ -n "$branch" ]; then
      echo "Régénérée par \`tools/proj-db/update.sh $tag\` et poussée sur la branche \`$branch\`."
      echo "Ouvrir la PR : $server/$repo/compare/main...$branch?expand=1"
    else
      echo "Régénérée par \`tools/proj-db/update.sh $tag\` (rien n'est poussé)."
    fi ;;
  3)
    echo "**Bloqué** : la version majeure du schéma (\`DATABASE.LAYOUT.VERSION.MAJOR\`) a changé."
    echo "proj-rs lit l'ancien schéma : la base n'est pas remplacée. Il faut d'abord porter le code"
    echo "de lecture de PROJ (\`src/iso19111/factory.rs\`, \`src/projdb.rs\`)." ;;
  *)
    echo "**Échec** de la régénération (statut $update_status) : voir le journal de l'exécution." ;;
esac
echo
[ -s "$dir/proj-db-diff.md" ] && cat "$dir/proj-db-diff.md" && echo

echo "## 2. Suite de tests contre PROJ $tag"
echo
if [ "$tests_log" = "-" ] || [ ! -s "$tests_log" ]; then
  echo "Non lancée."
else
  awk '
    /^ *Running / { binary = $2 }
    /^test result:/ { passed += $4; failed += $6; ignored += $8; lines++ }
    / \.\.\. FAILED$/ { sub(/^test /, ""); sub(/ \.\.\. FAILED$/, ""); failures[++n] = binary " :: " $0 }
    /^error: could not compile/ { broken[++m] = $0 }
    END {
      printf "%d lignes `test result` : %d réussis, %d en échec, %d ignorés.\n", lines, passed, failed, ignored
      if (m > 0) { print ""; print "**Ne compile pas :**"; for (i = 1; i <= m; i++) print "- " broken[i] }
      if (n > 0) {
        print ""; print "Tests en échec (" n ") :"; print ""
        for (i = 1; i <= n && i <= 100; i++) print "- `" failures[i] "`"
        if (n > 100) print "- … et " n - 100 " autres (journal de l’exécution)"
      }
    }' "$tests_log"
  # The same cause often fails hundreds of tests: the most frequent messages come first.
  messages=$(grep -h -A2 "panicked at" "$tests_log" | grep -v -e "panicked at" -e '^--$' -e '^stack backtrace' \
    -e '^note: ' -e '^ *$' | sed -E 's/[0-9]+\.[0-9]{4,}/N/g' | cut -c1-200 | sort | uniq -c | sort -rn | head -5)
  if [ -n "$messages" ]; then
    echo
    echo "Messages d'échec les plus fréquents (nombre d'occurrences) :"
    echo
    echo '```'
    echo "$messages"
    echo '```'
  fi
  echo
  echo "Un échec n'est pas forcément un bug : un test qui fige une sortie de PROJ 9.9.0 (liste de"
  echo "candidats, texte de \`projinfo\`) change légitimement quand l'EPSG change. Le comparer à la"
  echo "sortie de PROJ $tag lui-même avant de corriger quoi que ce soit."
fi
echo

echo "## 3. Suite \`gie\` de PROJ"
echo
gie_new=$dir/src/test/gie
if [ -d "$gie_new" ]; then
  changed=$(for file in "$gie_new"/*.gie; do
    name=$(basename "$file")
    if [ ! -f "$root/tests/gie/$name" ]; then echo "- \`$name\` (nouveau)"
    elif ! cmp -s "$file" "$root/tests/gie/$name"; then echo "- \`$name\` (modifié)"; fi
  done)
  if [ -n "$changed" ]; then
    echo "Fichiers de \`test/gie\` qui diffèrent de \`tests/gie/\` (à revendoriser dans le lot de portage) :"
    echo
    echo "$changed"
  else
    echo "Aucun fichier \`.gie\` ne change."
  fi
else
  echo "Sources de PROJ $tag absentes : non comparé."
fi
echo

echo "## 4. Ce qui reste à faire à la main"
echo
echo "Procédure complète : $server/$repo/blob/main/docs/PROJ-UPSTREAM.md"
echo
echo "- [ ] Ouvrir la PR de la base, relire les tests en échec (vrai bug, ou valeur figée de PROJ $current)."
echo "- [ ] Lire le NEWS de PROJ $tag : un correctif de calcul ou de sélection se porte dans un lot à part."
echo "- [ ] Revendoriser les fichiers \`.gie\` modifiés et mettre à jour \`tests/gie/passing.txt\`."
echo "- [ ] Déplacer la référence : \`tools/build-reference.sh\`, \`.github/workflows/ci.yml\` (conda, clés de cache),"
echo "      les macros de version de \`capi/include/proj.h\`, les constantes \`RELEASE\` des outils."
