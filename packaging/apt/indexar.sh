#!/bin/bash
# Gera os índices do repositório apt e os assina.
#   bash packaging/apt/indexar.sh PASTA CHAVE_PUBLICA.asc
# A chave privada correspondente precisa estar no GnuPG de quem roda. O publish-apt.sh chama
# este script depois das conferências; os testes o usam direto para montar repositórios falsos.
set -euo pipefail
umask 022

PASTA=$(realpath "$1")
CHAVE=$(realpath "$2")
[ -d "$PASTA/pool" ] || { echo "sem pool/ em $PASTA" >&2; exit 1; }
FPR=$(gpg --show-keys --with-colons "$CHAVE" | awk -F: '$1 == "fpr" && !f { print $10; f = 1 }')
[ -n "$FPR" ] || { echo "chave pública ilegível: $CHAVE" >&2; exit 1; }
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
gpg --dearmor < "$CHAVE" > "$TMP/chave.gpg"

cd "$PASTA"
D=dists/stable
B=$D/main/binary-amd64
# Apaga o que restou da última assinatura, para não entrar no Release novo.
rm -f "$D/Release" "$D/InRelease" "$D/Release.gpg"
mkdir -p "$B"

# Só SHA-256 e SHA-512: o apt atual ignora MD5 e SHA-1.
HASHES=(-o APT::FTPArchive::MD5=false -o APT::FTPArchive::SHA1=false
        -o APT::FTPArchive::SHA256=true -o APT::FTPArchive::SHA512=true)
apt-ftparchive "${HASHES[@]}" packages pool > "$B/Packages"
gzip -9nkf "$B/Packages"
xz -9kf "$B/Packages"

R=APT::FTPArchive::Release
apt-ftparchive "${HASHES[@]}" \
    -o "$R::Origin=Super Cola" -o "$R::Label=Super Cola" \
    -o "$R::Suite=stable" -o "$R::Codename=stable" \
    -o "$R::Architectures=amd64" -o "$R::Components=main" \
    -o "$R::Description=Atualizações do Super Cola" \
    release "$D" > "$TMP/Release"
mv "$TMP/Release" "$D/Release"

gpg --local-user "$FPR" --digest-algo SHA512 --clearsign -o "$D/InRelease" "$D/Release"
gpg --local-user "$FPR" --digest-algo SHA512 --armor --detach-sign -o "$D/Release.gpg" "$D/Release"

# Confere as duas assinaturas só com a chave pública, como o apt vai fazer.
gpgv --keyring "$TMP/chave.gpg" "$D/InRelease" 2>/dev/null
gpgv --keyring "$TMP/chave.gpg" "$D/Release.gpg" "$D/Release" 2>/dev/null
echo "índices assinados por $FPR"
