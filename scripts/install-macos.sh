#!/bin/bash
# Instala ou atualiza o Lume no macOS, sem o aviso "A Apple não pôde verificar…".
#
# O aviso aparece porque o navegador marca o arquivo baixado e o Lume não tem a assinatura paga
# da Apple (só a ad-hoc). Baixado por aqui, o arquivo não é marcado. A garantia é outra: o .dmg
# vem do release oficial no GitHub e a soma SHA-256 tem que bater com a do SHA256SUMS.txt do
# mesmo release.
#
# O repositório é privado, então o download usa o `gh` (GitHub CLI) já logado:
#   gh api repos/phs-santos/Lume/contents/scripts/install-macos.sh -H "Accept: application/vnd.github.raw" | bash
#
# LUME_VERSAO escolhe uma versão (ex.: v0.1.0; padrão: a mais recente publicada).
# LUME_DESTINO troca a pasta (padrão: /Applications, ou ~/Applications sem permissão).

set -euo pipefail

REPO="phs-santos/Lume"

erro() {
    echo "Erro: $*" >&2
    exit 1
}

[ "$(uname -s)" = "Darwin" ] || erro "este instalador é só para macOS."
command -v gh >/dev/null || erro "precisa do GitHub CLI: brew install gh && gh auth login"
gh auth status >/dev/null 2>&1 || erro "entre no GitHub CLI primeiro: gh auth login"

# Chip Apple, inclusive quando o Terminal roda pelo Rosetta.
if [ "$(uname -m)" = "arm64" ] || [ "$(sysctl -n sysctl.proc_translated 2>/dev/null || echo 0)" = "1" ]; then
    arch="aarch64"
else
    arch="x64"
fi

if pgrep -xq Lume; then
    erro "o Lume está aberto. Feche (⌘Q) e rode o comando de novo."
fi

tmp="$(mktemp -d)"
trap 'hdiutil detach -quiet "$tmp/mnt" 2>/dev/null || true; rm -rf "$tmp"' EXIT

versao="${LUME_VERSAO:-}"
if [ -z "$versao" ]; then
    versao="$(gh release view --repo "$REPO" --json tagName -q .tagName 2>/dev/null)" || erro "não achei nenhum release publicado."
fi

echo "Baixando o Lume $versao ($arch)…"
gh release download "$versao" --repo "$REPO" --dir "$tmp" --pattern "*_${arch}.dmg" --pattern "SHA256SUMS.txt" \
    || erro "o download falhou (o release $versao tem o .dmg de $arch e o SHA256SUMS.txt?)."
dmg="$(ls "$tmp"/*_"$arch".dmg | head -1)"
nome="$(basename "$dmg")"

echo "Conferindo o arquivo…"
esperado="$(awk -v f="$nome" '$2 == f { print $1 }' "$tmp/SHA256SUMS.txt")"
obtido="$(shasum -a 256 "$dmg" | cut -d' ' -f1)"
[ -n "$esperado" ] || erro "o SHA256SUMS.txt não lista $nome."
[ "$obtido" = "$esperado" ] || erro "o arquivo não confere com o release (SHA-256 diferente). Nada foi instalado."

mkdir "$tmp/mnt"
hdiutil attach -quiet -nobrowse -readonly -mountpoint "$tmp/mnt" "$dmg" || erro "não consegui abrir o .dmg."
[ -d "$tmp/mnt/Lume.app" ] || erro "o .dmg não tem o Lume.app."

destino="${LUME_DESTINO:-/Applications}"
if [ -z "${LUME_DESTINO:-}" ] && [ ! -w "$destino" ]; then
    destino="$HOME/Applications"
fi
mkdir -p "$destino"

# Troca só o app; o cofre e os livros ficam na pasta de dados e continuam.
rm -rf "$destino/Lume.app"
ditto "$tmp/mnt/Lume.app" "$destino/Lume.app"
xattr -dr com.apple.quarantine "$destino/Lume.app" 2>/dev/null || true

echo "Pronto: Lume $versao instalado em $destino/Lume.app"
if [ -z "${LUME_NAO_ABRIR:-}" ]; then
    open "$destino/Lume.app"
fi
