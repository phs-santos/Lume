#!/bin/bash
# Instala ou atualiza o Lume no macOS, sem o aviso "A Apple não pôde verificar…".
#
# O aviso aparece porque o navegador marca o arquivo baixado e o Lume não tem a assinatura paga
# da Apple (só a ad-hoc). Baixado por aqui, o arquivo não é marcado. A garantia é outra: o .dmg
# vem do release oficial no GitHub e a soma SHA-256 tem que bater com a do SHA256SUMS.txt do
# mesmo release.
#
#   curl -fsSL https://raw.githubusercontent.com/phs-santos/Lume/main/scripts/install-macos.sh | bash
#
# LUME_VERSAO escolhe uma versão (ex.: v0.1.0; padrão: a mais recente publicada).
# LUME_DESTINO troca a pasta (padrão: /Applications, ou ~/Applications sem permissão).

set -euo pipefail

REPO="phs-santos/Lume"

erro() {
    echo "Erro: $*" >&2
    exit 1
}

[ "$(uname -s)" = "Darwin" ] || erro "este instalador é só para macOS. Os outros sistemas estão em https://github.com/$REPO/releases/latest"

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

if [ -n "${LUME_VERSAO:-}" ]; then
    base="https://github.com/$REPO/releases/download/$LUME_VERSAO"
else
    base="https://github.com/$REPO/releases/latest/download"
fi

echo "Procurando a versão…"
curl -fsSL "$base/SHA256SUMS.txt" -o "$tmp/SHA256SUMS.txt" || erro "não deu para falar com o GitHub (ou ainda não há release publicado). Confira a internet."
# o nome do .dmg (com a versão) vem da própria lista de somas
nome="$(awk -v a="_${arch}.dmg" 'substr($2, length($2) - length(a) + 1) == a { print $2; exit }' "$tmp/SHA256SUMS.txt")"
esperado="$(awk -v f="$nome" '$2 == f { print $1 }' "$tmp/SHA256SUMS.txt")"
[ -n "$nome" ] && [ -n "$esperado" ] || erro "o release não tem o .dmg para $arch."
versao="$(echo "$nome" | sed -E 's/^Lume_(.*)_[^_]+\.dmg$/\1/')"
dmg="$tmp/$nome"

echo "Baixando o Lume $versao ($arch)…"
curl -fL --progress-bar "$base/$nome" -o "$dmg" || erro "o download falhou."

echo "Conferindo o arquivo…"
obtido="$(shasum -a 256 "$dmg" | cut -d' ' -f1)"
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
