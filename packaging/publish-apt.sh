#!/bin/bash
# Publica um .deb no repositório apt e assina tudo. Rode depois dos testes e antes da Release:
#   bash packaging/publish-apt.sh dist/supercola_<versão>_amd64.deb ../supercola-apt
# O segundo argumento é um clone de hlmgdev/supercola-apt. O script pede a senha da chave e
# não faz commit nem push: confira o resultado e envie você mesmo.
set -euo pipefail
umask 022

[ $# -eq 2 ] || { sed -n '2,5p' "$0" | sed 's/^# \?//'; exit 2; }
DEB=$(realpath "$1")
APT=$(realpath "$2")
RAIZ=$(cd "$(dirname "$0")/.." && pwd)
CHAVE=$(realpath "${SUPERCOLA_APT_KEY:-$RAIZ/packaging/apt/supercola-archive-keyring.asc}")
[ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"

falha(){ echo "recusado: $*" >&2; exit 1; }
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

[ -f "$DEB" ] || falha "não achei $DEB"
[ -s "$CHAVE" ] || falha "não achei a chave pública $CHAVE"
[ -f "$APT/.nojekyll" ] || falha "$APT não parece um clone do supercola-apt"

echo "== pacote"
campo(){ dpkg-deb -f "$DEB" "$1"; }
VERSAO=$(campo Version)
ESPERADA=$(grep -m1 '^version' "$RAIZ/Cargo.toml" | cut -d'"' -f2)
[ "$(campo Package)" = supercola ] || falha "o pacote se chama $(campo Package)"
[ "$(campo Architecture)" = amd64 ] || falha "arquitetura $(campo Architecture)"
[ "$VERSAO" = "$ESPERADA" ] || falha "o pacote é $VERSAO, mas o Cargo.toml diz $ESPERADA"
[ "$(basename "$DEB")" = "supercola_${VERSAO}_amd64.deb" ] || falha "nome inesperado: $(basename "$DEB")"
case "$(campo Maintainer)" in *.invalid*|"") falha "mantenedor inválido: $(campo Maintainer)" ;; esac

# Nada pode rodar como root na instalação: só os arquivos de controle comuns.
while read -r f; do
  case "$f" in ''|control|conffiles|md5sums) ;; *) falha "o pacote traz $f, que rodaria como root" ;; esac
done < <(dpkg-deb --ctrl-tarfile "$DEB" | tar -t | sed 's|^\./||')

LISTA=$(dpkg-deb --fsys-tarfile "$DEB" | tar -tv --numeric-owner)
SUID=$(awk '$1 ~ /[sS]/ { print $NF }' <<< "$LISTA")
[ -z "$SUID" ] || falha "arquivos com setuid ou setgid: $SUID"
DONOS=$(awk '$2 != "0/0" { print $NF }' <<< "$LISTA")
[ -z "$DONOS" ] || falha "arquivos que não são de root: $DONOS"

extrair(){ dpkg-deb --fsys-tarfile "$DEB" | tar -xO "./$1"; }
gpg --dearmor < "$CHAVE" > "$TMP/chave.gpg"
cmp -s <(extrair etc/apt/keyrings/supercola-archive-keyring.gpg) "$TMP/chave.gpg" \
  || falha "a chave dentro do pacote não é $CHAVE"
cmp -s <(extrair etc/apt/sources.list.d/supercola.sources) "$RAIZ/packaging/apt/supercola.sources" \
  || falha "o supercola.sources do pacote difere de packaging/apt"
cmp -s <(extrair etc/apt/preferences.d/supercola.pref) "$RAIZ/packaging/apt/supercola.pref" \
  || falha "o supercola.pref do pacote difere de packaging/apt"
CONFS=$(dpkg-deb --ctrl-tarfile "$DEB" | tar -xO ./conffiles)
for c in /etc/apt/keyrings/supercola-archive-keyring.gpg /etc/apt/sources.list.d/supercola.sources \
         /etc/apt/preferences.d/supercola.pref; do
  grep -qx "$c" <<< "$CONFS" || falha "$c não está em conffiles"
done
echo "ok: supercola $VERSAO, sem scripts de instalação, chave e origem conferidas"

echo "== chave"
FPR=$(gpg --show-keys --with-colons "$CHAVE" | awk -F: '$1 == "fpr" { print $10; exit }')
gpg --list-secret-keys "$FPR" >/dev/null 2>&1 || falha "a chave privada $FPR não está neste GnuPG"
VENCE=$(gpg --show-keys --with-colons "$CHAVE" | awk -F: '$1 == "pub" { print $7; exit }')
if [ -n "$VENCE" ]; then
  AGORA=$(date +%s)
  [ "$VENCE" -gt "$AGORA" ] || falha "a chave venceu"
  DIAS=$(( (VENCE - AGORA) / 86400 ))
  [ "$DIAS" -ge 180 ] || echo "atenção: a chave vence em $DIAS dias; estenda e distribua antes"
fi
# Quem baixa confere a chave pelo README, então ele precisa mostrar a chave de verdade.
# Os testes usam uma chave descartável, que não está no README.
if [ -z "${SUPERCOLA_APT_KEY:-}" ]; then
  grep -qF "$FPR" "$RAIZ/README.md" || falha "o README não mostra a impressão digital $FPR"
  ! grep -q '@IMPRESSAO_DIGITAL@\|@VENCIMENTO@' "$RAIZ/README.md" || falha "o README ainda tem marcadores"
fi
echo "ok: $FPR"

echo "== dependências"
(cd "$RAIZ" && cargo audit) || falha "o cargo audit achou falha conhecida nas dependências"

echo "== repositório"
HASH=$(sha256sum "$DEB" | cut -d' ' -f1)
REGISTRO=$APT/publicadas.txt
touch "$REGISTRO"
# O registro guarda cada versão já publicada, mesmo depois que o arquivo sai do pool.
ANTERIOR=$(awk -v v="$VERSAO" '$1 == v { print $2 }' "$REGISTRO")
if [ -n "$ANTERIOR" ]; then
  [ "$ANTERIOR" = "$HASH" ] || falha "a versão $VERSAO já foi publicada com outro conteúdo"
  echo "a versão $VERSAO já estava publicada com este mesmo arquivo"
else
  ULTIMA=$(awk 'NF { v = $1 } END { print v }' "$REGISTRO")
  if [ -n "$ULTIMA" ]; then
    dpkg --compare-versions "$VERSAO" gt "$ULTIMA" || falha "$VERSAO não é maior que $ULTIMA"
  fi
  echo "$VERSAO $HASH" >> "$REGISTRO"
fi

POOL=$APT/pool/main/s/supercola
mkdir -p "$POOL"
install -m644 "$DEB" "$POOL/"
# Mantém só as 3 versões mais recentes no pool. O registro está em ordem de publicação.
MANTER=$(awk 'NF { print $1 }' "$REGISTRO" | tail -3)
for f in "$POOL"/supercola_*_amd64.deb; do
  v=$(dpkg-deb -f "$f" Version)
  grep -qx "$v" <<< "$MANTER" || { rm -f "$f"; echo "saiu do pool: $v"; }
done

echo "== assinatura (o GnuPG vai pedir a senha)"
bash "$RAIZ/packaging/apt/indexar.sh" "$APT" "$CHAVE"

SOMAS=$(dirname "$DEB")/SHA256SUMS
(cd "$(dirname "$DEB")" && sha256sum "$(basename "$DEB")") > "$SOMAS"
gpg --local-user "$FPR" --digest-algo SHA512 --armor --detach-sign --yes -o "$SOMAS.asc" "$SOMAS"
gpgv --keyring "$TMP/chave.gpg" "$SOMAS.asc" "$SOMAS" 2>/dev/null

echo
echo "Pronto. Falta você:"
echo "  1. conferir e enviar: cd $APT && git add -A && git status, depois commit e push"
echo "  2. anexar à Release v$VERSAO: $DEB, $SOMAS e $SOMAS.asc"
