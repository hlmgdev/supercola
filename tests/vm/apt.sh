#!/bin/bash
# Testa o repositório apt em uma VM, com uma chave descartável e o repositório servido por este
# computador, que a VM enxerga em 10.0.2.2. Rode na raiz do repositório, depois de gerar o
# pacote e com a VM criada: nada roda na sua sessão.
#   bash tests/vm/apt.sh [ubuntu2404-gnome|ubuntu2604-gnome|debian13-kde|debian13-gnome]
set -u
N=${1:-ubuntu2404-gnome}
DEB=$(ls -t dist/supercola_*_amd64.deb | head -1)
VERSAO=$(dpkg-deb -f "$DEB" Version)
PORTA=8790
URL=http://10.0.2.2:$PORTA
falhas=0; total=0

v(){ bash tests/vm/vm.sh "$@"; }
ok(){ total=$((total+1)); echo "OK    $1"; }
ko(){ total=$((total+1)); falhas=$((falhas+1)); echo "FALHA $1"; }
checa(){ if [ "$2" = "$3" ]; then ok "$1"; else ko "$1: esperado [$3], obtido [$2]"; fi; }
contem(){ case "$2" in *"$3"*) ok "$1";; *) ko "$1: [$3] não está em [$2]";; esac; }
vm(){ v ssh "$N" "$1" </dev/null; }

T=$(mktemp -d)
SERVIDOR=
fim(){
  [ -n "$SERVIDOR" ] && kill "$SERVIDOR" 2>/dev/null
  for k in a b; do gpgconf --homedir "$T/$k" --kill gpg-agent 2>/dev/null; done
  rm -rf "$T"
}
trap fim EXIT

# Duas chaves descartáveis: "a" faz o papel da verdadeira, "b" é de um estranho.
for k in a b; do
  mkdir -m700 "$T/$k"
  GNUPGHOME=$T/$k gpg --batch --quiet --passphrase '' \
    --quick-generate-key "Teste $k <teste-$k@supercola.invalid>" ed25519 sign 1d 2>/dev/null
  GNUPGHOME=$T/$k gpg --armor --export > "$T/$k.asc"
  GNUPGHOME=$T/$k gpg --batch --quiet --check-trustdb 2>/dev/null
done

# O pacote de verdade, só com a chave trocada pela de teste. Com versão, muda também a versão.
reempacotar(){ # destino [versão]
  rm -rf "$T/arvore"
  dpkg-deb -R "$DEB" "$T/arvore"
  gpg --dearmor < "$T/a.asc" > "$T/arvore/etc/apt/keyrings/supercola-archive-keyring.gpg"
  [ -n "${2:-}" ] && sed -i "s/^Version: .*/Version: $2/" "$T/arvore/DEBIAN/control"
  dpkg-deb --root-owner-group -b "$T/arvore" "$1" >/dev/null
  rm -rf "$T/arvore"
}
indexar(){ GNUPGHOME=$T/$1 bash packaging/apt/indexar.sh "$T/repo" "$T/$1.asc" >/dev/null; }
POOL=$T/repo/pool/main/s/supercola

echo "== pacote: $DEB"
mkdir -p "$T/deb" "$T/repo"; touch "$T/repo/.nojekyll"
reempacotar "$T/deb/supercola_${VERSAO}_amd64.deb"
# O primeiro índice sai pelo mesmo caminho de uma publicação de verdade.
if ! GNUPGHOME=$T/a SUPERCOLA_APT_KEY=$T/a.asc bash packaging/publish-apt.sh \
     "$T/deb/supercola_${VERSAO}_amd64.deb" "$T/repo" > "$T/publicar.log" 2>&1; then
  cat "$T/publicar.log"; exit 1
fi
python3 -I -m http.server "$PORTA" --bind 127.0.0.1 --directory "$T/repo" >/dev/null 2>&1 &
SERVIDOR=$!
sleep 1

v ligar "$N"
echo "apt: $(vm 'apt-get --version | head -1')"
checa "a VM alcança o repositório" "$(vm "timeout 5 bash -c 'exec 3<>/dev/tcp/10.0.2.2/$PORTA' && echo sim")" sim
v copiar "$N" "$T/deb/supercola_${VERSAO}_amd64.deb" /tmp/sc.deb
# Cada rodada começa do zero, como um usuário novo, e aponta o repositório para este computador.
vm "na-sessao supercola-toggle quit >/dev/null 2>&1
  sudo apt-get purge -y -q supercola >/dev/null 2>&1
  sudo apt-get install -y -q /tmp/sc.deb >/dev/null 2>&1
  sudo sed -i 's|https://hlmgdev.github.io/supercola-apt|$URL|' /etc/apt/sources.list.d/supercola.sources
  sudo sed -i 's/\"hlmgdev.github.io\"/\"10.0.2.2\"/' /etc/apt/preferences.d/supercola.pref"

atualizar(){ vm 'sudo env LC_ALL=C apt-get update 2>&1'; }
# Linhas sobre o repositório de teste que não sejam um download normal: avisos e erros.
problemas(){ grep -F "10.0.2.2" <<< "$1" | grep -v -E '^(Get|Hit):'; }
versao(){ vm "dpkg-query -W -f='\${Version}' supercola 2>/dev/null"; }
candidato(){ vm "LC_ALL=C apt-cache policy $1 | sed -n 's/^ *Candidate: //p'"; }

echo "== instalação e assinatura"
checa "o pacote instalou" "$(versao)" "$VERSAO"
saida=$(atualizar)
checa "o apt aceitou a assinatura" "$(problemas "$saida")" ""
contem "o apt leu o índice" "$saida" "$URL stable InRelease"
contem "o supercola vem do repositório" "$(vm 'LC_ALL=C apt-cache policy supercola')" "$URL stable/main"

echo "== atualização"
reempacotar "$POOL/supercola_${VERSAO}+teste1_amd64.deb" "${VERSAO}+teste1"
indexar a
saida=$(atualizar)
checa "o índice da versão nova foi aceito" "$(problemas "$saida")" ""
vm 'sudo env DEBIAN_FRONTEND=noninteractive apt-get install -y -q --only-upgrade \
  -o Dpkg::Options::=--force-confdef -o Dpkg::Options::=--force-confold supercola >/dev/null 2>&1'
checa "o apt atualizou o pacote" "$(versao)" "${VERSAO}+teste1"
contem "a atualização manteve o endereço editado" \
  "$(vm 'cat /etc/apt/sources.list.d/supercola.sources')" "URIs: $URL"

echo "== pacote alheio"
mkdir -p "$T/hello/DEBIAN" "$T/repo/pool/main/h/hello"
printf 'Package: hello\nVersion: 99\nArchitecture: amd64\nMaintainer: x <x@x.invalid>\nDescription: falso\n' \
  > "$T/hello/DEBIAN/control"
dpkg-deb --root-owner-group -b "$T/hello" "$T/repo/pool/main/h/hello/hello_99_amd64.deb" >/dev/null
indexar a
atualizar >/dev/null
politica=$(vm 'LC_ALL=C apt-cache policy hello')
contem "o apt viu o hello falso" "$politica" "99 -1"
[ "$(candidato hello)" != 99 ] && ok "o hello falso não é candidato" || ko "o hello falso virou candidato"
rm -rf "$T/repo/pool/main/h"

echo "== índice adulterado"
reempacotar "$POOL/supercola_${VERSAO}+teste2_amd64.deb" "${VERSAO}+teste2"
indexar a
sed -i 's/^Description: .*/&!/' "$T/repo/dists/stable/InRelease"
saida=$(atualizar)
[ -n "$(problemas "$saida")" ] && ok "o apt avisou do índice adulterado" || ko "o apt aceitou o índice adulterado"
checa "a versão do índice adulterado não aparece" "$(candidato supercola)" "${VERSAO}+teste1"

echo "== outra chave"
indexar b
saida=$(atualizar)
[ -n "$(problemas "$saida")" ] && ok "o apt avisou da chave estranha" || ko "o apt aceitou a chave estranha"
checa "a versão assinada pela chave estranha não aparece" "$(candidato supercola)" "${VERSAO}+teste1"
rm -f "$POOL/supercola_${VERSAO}+teste2_amd64.deb"
indexar a

echo "== remoção"
vm 'sudo apt-get remove -y -q supercola >/dev/null 2>&1'
checa "o remove mantém o repositório" \
  "$(vm 'ls /etc/apt/sources.list.d/supercola.sources /etc/apt/preferences.d/supercola.pref \
    /etc/apt/keyrings/supercola-archive-keyring.gpg 2>/dev/null | wc -l')" 3
checa "o repositório segue válido depois do remove" "$(problemas "$(atualizar)")" ""
vm 'sudo apt-get purge -y -q supercola >/dev/null 2>&1'
checa "o purge apaga os três arquivos" \
  "$(vm 'ls /etc/apt/sources.list.d/supercola.sources /etc/apt/preferences.d/supercola.pref \
    /etc/apt/keyrings/supercola-archive-keyring.gpg 2>/dev/null | wc -l')" 0
checa "depois do purge, o apt nem consulta o repositório" "$(grep -c -F 10.0.2.2 <<< "$(atualizar)")" 0

echo
echo "$N: $((total - falhas)) de $total"
[ "$falhas" -eq 0 ]
