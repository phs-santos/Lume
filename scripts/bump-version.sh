#!/bin/bash
# Troca a versão do Lume em todos os lugares de uma vez.
#
#   scripts/bump-version.sh patch        0.1.0 -> 0.1.1 (só correções)
#   scripts/bump-version.sh minor        0.1.0 -> 0.2.0 (coisa nova)
#   scripts/bump-version.sh major        0.1.0 -> 1.0.0 (mudança grande)
#   scripts/bump-version.sh 0.3.0-beta.1 versão exata (com hífen vira pré-lançamento)
#
# Escreve a versão nova na saída. Não faz commit nem tag.

set -euo pipefail
cd "$(dirname "$0")/.."

atual="$(node -p "require('./src-tauri/tauri.conf.json').version")"
IFS=. read -r ma mi pa <<<"${atual%%-*}"

case "${1:-}" in
    patch) nova="$ma.$mi.$((pa + 1))" ;;
    minor) nova="$ma.$((mi + 1)).0" ;;
    major) nova="$((ma + 1)).0.0" ;;
    [0-9]*.[0-9]*.[0-9]*) nova="$1" ;;
    *) echo "uso: $0 patch|minor|major|X.Y.Z" >&2; exit 1 ;;
esac

[[ "$nova" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.]+)?$ ]] || { echo "versão inválida: $nova" >&2; exit 1; }
git rev-parse -q --verify "refs/tags/v$nova" >/dev/null && { echo "a tag v$nova já existe" >&2; exit 1; }

# tauri.conf.json, package.json e a versão do próprio projeto no package-lock.json
node -e '
  const fs = require("fs"); const v = process.argv[1];
  // troca só o "version" de cima, sem reformatar o resto do arquivo
  for (const f of ["src-tauri/tauri.conf.json", "package.json"]) {
    const s = fs.readFileSync(f, "utf8");
    fs.writeFileSync(f, s.replace(/"version": "[^"]+"/, `"version": "${v}"`));
  }
  const f = "package-lock.json"; const j = JSON.parse(fs.readFileSync(f, "utf8"));
  j.version = v; if (j.packages && j.packages[""]) j.packages[""].version = v;
  fs.writeFileSync(f, JSON.stringify(j, null, 2) + "\n");' "$nova"
# Cargo.toml e o Cargo.lock do pacote lume
sed -i '' -E "s/^version = \"[^\"]+\"/version = \"$nova\"/" src-tauri/Cargo.toml
awk -v v="$nova" 'prev == "name = \"lume\"" && /^version = / { $0 = "version = \"" v "\"" } { print; prev = $0 }' \
    src-tauri/Cargo.lock > src-tauri/Cargo.lock.tmp && mv src-tauri/Cargo.lock.tmp src-tauri/Cargo.lock

echo "$nova"
