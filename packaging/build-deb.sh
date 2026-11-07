#!/bin/bash
# Gera dist/supercola_<versão>_<arq>.deb. Rode no Pop!_OS 24.04 ou no Ubuntu 24.04, que definem
# as versões mínimas do pacote, com o gtk4-layer-shell instalado em /usr/local.
set -euo pipefail

VERSAO=$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)
ARQ=$(dpkg --print-architecture)
# Chave pública que o apt usa para conferir o repositório. Os testes passam uma descartável.
CHAVE=${SUPERCOLA_APT_KEY:-packaging/apt/supercola-archive-keyring.asc}
[ -s "$CHAVE" ] || { echo "falta a chave pública do repositório: $CHAVE" >&2; exit 1; }
RAIZ=$(mktemp -d)
trap 'rm -rf "$RAIZ"' EXIT

# A biblioteca embutida fica fora do caminho padrão, e só o supercola a enxerga.
# `--locked` garante que o pacote usa exatamente as dependências do Cargo.lock.
RUSTFLAGS="-C link-arg=-Wl,-rpath,/usr/lib/supercola" \
    cargo build --locked --release -p supercola -p supercola-toggle

install -Dm755 target/release/supercola "$RAIZ/usr/bin/supercola"
install -Dm755 target/release/supercola-toggle "$RAIZ/usr/bin/supercola-toggle"

LIB=$(readlink -f /usr/local/lib/libgtk4-layer-shell.so.0)
install -Dm644 "$LIB" "$RAIZ/usr/lib/supercola/libgtk4-layer-shell.so.0"
strip --strip-unneeded "$RAIZ/usr/lib/supercola/libgtk4-layer-shell.so.0"

install -Dm644 data/kwin/focus.js "$RAIZ/usr/share/supercola/kwin/focus.js"
EXTENSOES="$RAIZ/usr/share/gnome-shell/extensions"
install -d "$EXTENSOES"
cp -r data/gnome-shell/. "$EXTENSOES/"
find "$EXTENSOES" -type d -exec chmod 755 {} + -o -type f -exec chmod 644 {} +
install -d "$RAIZ/usr/share/supercola/icons"
install -m644 data/icons/*.svg "$RAIZ/usr/share/supercola/icons/"
install -Dm644 data/fonts/selawk.ttf "$RAIZ/usr/share/fonts/truetype/supercola/selawk.ttf"
install -m644 data/fonts/selawkl.ttf data/fonts/selawksb.ttf "$RAIZ/usr/share/fonts/truetype/supercola/"
install -Dm644 packaging/deb/supercola.desktop "$RAIZ/usr/share/applications/dev.supercola.SuperCola.desktop"
install -m644 packaging/deb/dev.supercola.*.desktop "$RAIZ/usr/share/applications/"
install -Dm644 packaging/deb/supercola-autostart.desktop "$RAIZ/etc/xdg/autostart/supercola.desktop"
install -Dm644 data/supercola.svg "$RAIZ/usr/share/icons/hicolor/scalable/apps/supercola.svg"

# Repositório de atualizações. Os três arquivos ficam em /etc como conffiles: o `remove` os
# mantém funcionando juntos, e só o `purge` os apaga.
install -d -m755 "$RAIZ/etc/apt/keyrings" "$RAIZ/etc/apt/sources.list.d" "$RAIZ/etc/apt/preferences.d"
gpg --dearmor < "$CHAVE" > "$RAIZ/etc/apt/keyrings/supercola-archive-keyring.gpg"
chmod 644 "$RAIZ/etc/apt/keyrings/supercola-archive-keyring.gpg"
install -m644 packaging/apt/supercola.sources "$RAIZ/etc/apt/sources.list.d/supercola.sources"
install -m644 packaging/apt/supercola.pref "$RAIZ/etc/apt/preferences.d/supercola.pref"

DOC="$RAIZ/usr/share/doc/supercola"
install -d "$DOC"
install -m644 data/licenses/* LICENSE README.md CHANGELOG.md "$DOC/"

# Dependências calculadas a partir dos binários, sem a biblioteca embutida.
mkdir -p "$RAIZ/debian"
: > "$RAIZ/debian/control"
DEPS=$(cd "$RAIZ" && LD_LIBRARY_PATH=/usr/local/lib dpkg-shlibdeps -O \
        --ignore-missing-info usr/bin/supercola usr/bin/supercola-toggle usr/lib/supercola/libgtk4-layer-shell.so.0 2>/dev/null \
      | sed -n 's/^shlibs:Depends=//p')
rm -rf "$RAIZ/debian"

install -d "$RAIZ/DEBIAN"
TAMANHO=$(du -sk --exclude=DEBIAN "$RAIZ" | cut -f1)
sed -e "s/@VERSION@/$VERSAO/" -e "s/@ARCH@/$ARQ/" -e "s/@SIZE@/$TAMANHO/" -e "s|@DEPENDS@|$DEPS|" \
    packaging/deb/control > "$RAIZ/DEBIAN/control"
printf '%s\n' /etc/xdg/autostart/supercola.desktop \
    /etc/apt/keyrings/supercola-archive-keyring.gpg \
    /etc/apt/sources.list.d/supercola.sources \
    /etc/apt/preferences.d/supercola.pref > "$RAIZ/DEBIAN/conffiles"

mkdir -p dist
SAIDA="dist/supercola_${VERSAO}_${ARQ}.deb"
dpkg-deb --root-owner-group --build "$RAIZ" "$SAIDA" >/dev/null
echo "$SAIDA"
dpkg-deb -f "$SAIDA" Depends
du -h "$SAIDA"
