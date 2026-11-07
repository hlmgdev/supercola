#!/bin/bash
# VMs para testar em sessões reais de GNOME e KDE. Rodam sem sudo, ligadas só em 127.0.0.1.
#
#   bash tests/vm/vm.sh lista
#   bash tests/vm/vm.sh criar NOME            baixa a imagem, confere o hash e instala tudo
#   bash tests/vm/vm.sh ligar NOME
#   bash tests/vm/vm.sh desligar NOME
#   bash tests/vm/vm.sh ssh NOME [comando]
#   bash tests/vm/vm.sh sessao NOME comando   roda na sessão gráfica do usuário
#   bash tests/vm/vm.sh copiar NOME ORIGEM [DESTINO]
#   bash tests/vm/vm.sh foto NOME ARQUIVO.png
#   bash tests/vm/vm.sh tecla NOME meta_l-v   teclado de verdade, com nomes do QEMU
#   bash tests/vm/vm.sh clique NOME X Y       em 1920 × 1080
#   bash tests/vm/vm.sh tela NOME             abre a tela, para testar à mão
#
# As VMs ficam em $SUPERCOLA_VMS. Usuário e senha: dev.
set -euo pipefail

BASE=${SUPERCOLA_VMS:-$HOME/.local/share/supercola-vms}
DEBIAN=https://cloud.debian.org/images/cloud/trixie/latest
UBUNTU=https://cloud-images.ubuntu.com

# nome: porta ssh, tela VNC, versão, pacotes, sessão
definir(){
  case "$1" in
    debian13-kde)
      PORTA=2221 TELA=21 SISTEMA=debian IMAGEM=debian-13-generic-amd64.qcow2 URL=$DEBIAN
      SOMAS=SHA512SUMS DM=sddm
      PACOTES="kde-plasma-desktop konsole kate kwin-wayland sddm" ;;
    ubuntu2404-gnome)
      PORTA=2222 TELA=22 SISTEMA=ubuntu IMAGEM=noble-server-cloudimg-amd64.img
      URL=$UBUNTU/noble/current SOMAS=SHA256SUMS DM=gdm
      PACOTES="ubuntu-desktop-minimal linux-generic gnome-terminal" ;;
    debian13-gnome)
      PORTA=2223 TELA=23 SISTEMA=debian IMAGEM=debian-13-generic-amd64.qcow2 URL=$DEBIAN
      SOMAS=SHA512SUMS DM=gdm
      PACOTES="gnome-core gdm3 gnome-terminal" ;;
    ubuntu2604-gnome)
      PORTA=2224 TELA=24 SISTEMA=ubuntu IMAGEM=resolute-server-cloudimg-amd64.img
      URL=$UBUNTU/resolute/current SOMAS=SHA256SUMS DM=gdm
      PACOTES="ubuntu-desktop-minimal linux-generic gnome-terminal" ;;
    *) echo "VM desconhecida: $1" >&2; exit 2 ;;
  esac
  PACOTES="$PACOTES wl-clipboard dbus-user-session xwayland x11-apps kbd libnotify-bin"
  D=$BASE/$1
}

chave(){
  [ -f "$BASE/chave" ] || ssh-keygen -q -t ed25519 -N '' -C supercola-vm -f "$BASE/chave"
}

# Cada imagem fica na própria pasta e só é aceita com o hash publicado.
baixar(){
  local pasta=$BASE/imagens/$SISTEMA-${IMAGEM%%.*}
  mkdir -p "$pasta"
  if [ ! -f "$pasta/ok" ]; then
    curl -fL --retry 3 -o "$pasta/$IMAGEM" "$URL/$IMAGEM"
    curl -fsSL -o "$pasta/$SOMAS" "$URL/$SOMAS"
    local esperado
    esperado=$(grep -E "[ *]$IMAGEM\$" "$pasta/$SOMAS" | cut -d' ' -f1)
    [ -n "$esperado" ] || { echo "hash não publicado para $IMAGEM" >&2; exit 1; }
    local programa=sha256sum; [ "$SOMAS" = SHA512SUMS ] && programa=sha512sum
    echo "$esperado  $pasta/$IMAGEM" | $programa -c - >/dev/null
    touch "$pasta/ok"
  fi
  ORIGEM=$pasta/$IMAGEM
}

semente(){
  local s=$D/semente; mkdir -p "$s"
  printf 'instance-id: %s\nlocal-hostname: %s\n' "$1" "$1" > "$s/meta-data"
  local gerente
  if [ "$DM" = sddm ]; then
    gerente='  - path: /etc/sddm.conf.d/autologin.conf
    content: |
      [Autologin]
      User=dev
      Session=plasma'
  else
    # O Debian lê `daemon.conf`, e o Ubuntu lê `custom.conf`.
    local conf=custom.conf; [ "$SISTEMA" = debian ] && conf=daemon.conf
    gerente="  - path: /etc/gdm3/$conf
    content: |
      [daemon]
      AutomaticLoginEnable=true
      AutomaticLogin=dev
      WaylandEnable=true"
  fi
  cat > "$s/user-data" <<EOF
#cloud-config
users:
  - name: dev
    groups: [sudo, adm, input]
    shell: /bin/bash
    sudo: ALL=(ALL) NOPASSWD:ALL
    lock_passwd: false
    plain_text_passwd: dev
    ssh_authorized_keys:
      - $(cat "$BASE/chave.pub")
ssh_pwauth: false
package_update: true
packages: [$(echo "$PACOTES" | tr ' ' ',')]
write_files:
$gerente
  - path: /usr/local/bin/na-sessao
    permissions: '0755'
    content: |
      #!/bin/bash
      export XDG_RUNTIME_DIR=/run/user/\$(id -u)
      export DBUS_SESSION_BUS_ADDRESS=unix:path=\$XDG_RUNTIME_DIR/bus
      while IFS= read -r linha; do
        case "\$linha" in
          WAYLAND_DISPLAY=*|XDG_CURRENT_DESKTOP=*|DISPLAY=*|XAUTHORITY=*|XDG_SESSION_TYPE=*) export "\$linha" ;;
        esac
      done < <(systemctl --user show-environment)
      exec "\$@"
runcmd:
  - systemctl set-default graphical.target
  # Sem isso, a imagem de nuvem atrasa o login em 2 minutos à espera da rede.
  - systemctl mask systemd-networkd-wait-online.service
power_state:
  mode: reboot
  condition: true
EOF
  xorriso -as mkisofs -quiet -volid cidata -joliet -rock -o "$D/semente.iso" \
    "$s/user-data" "$s/meta-data" 2>/dev/null
}

ligar(){
  [ -S "$D/qmp.sock" ] && kill -0 "$(cat "$D/pid" 2>/dev/null)" 2>/dev/null && return 0
  rm -f "$D/qmp.sock"
  qemu-system-x86_64 -accel kvm -cpu host -smp 4 -m 6G -M q35 -name "$NOME" \
    -drive "file=$D/disco.qcow2,if=virtio" -drive "file=$D/semente.iso,media=cdrom,readonly=on" \
    -device virtio-vga,xres=1920,yres=1080 -device virtio-keyboard-pci -device virtio-tablet-pci \
    -device virtio-rng-pci -nic "user,model=virtio-net-pci,hostfwd=tcp:127.0.0.1:$PORTA-:22" \
    -display none -vnc "127.0.0.1:$TELA" -qmp "unix:$D/qmp.sock,server=on,wait=off" \
    -daemonize -pidfile "$D/pid"
}

vssh(){
  ssh -i "$BASE/chave" -p "$PORTA" -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null \
    -o LogLevel=ERROR -o ConnectTimeout=5 -o BatchMode=yes dev@127.0.0.1 "$@"
}

esperar_ssh(){
  for _ in $(seq 180); do vssh true 2>/dev/null && return 0; sleep 5; done
  echo "$NOME não respondeu" >&2; return 1
}

qmp(){ # comando JSON
  (echo '{"execute":"qmp_capabilities"}'; sleep 0.2; echo "$1"; sleep 0.4) \
    | socat - "UNIX-CONNECT:$D/qmp.sock"
}

evento(){ # lista JSON de eventos
  qmp "{\"execute\":\"input-send-event\",\"arguments\":{\"events\":$1}}" >/dev/null
}

comando=${1:-}; shift || true
case "$comando" in
  lista)
    for n in debian13-kde ubuntu2404-gnome debian13-gnome ubuntu2604-gnome; do
      definir "$n"; estado=ausente
      if [ -f "$D/disco.qcow2" ]; then
        estado=desligada
        kill -0 "$(cat "$D/pid" 2>/dev/null)" 2>/dev/null && estado=ligada
      fi
      printf '%-18s %-10s ssh %s, vnc 127.0.0.1:%s\n' "$n" "$estado" "$PORTA" "$((5900+TELA))"
    done ;;
  criar)
    NOME=$1; definir "$NOME"; mkdir -p "$BASE" "$D"; chave; baixar
    [ -f "$D/disco.qcow2" ] || qemu-img create -q -f qcow2 -F qcow2 -b "$ORIGEM" "$D/disco.qcow2" 40G
    semente "$NOME"; ligar; esperar_ssh
    # A primeira ligação termina com um reinício: espera o cloud-init e a sessão gráfica.
    vssh 'cloud-init status --wait' >/dev/null 2>&1 || true
    sleep 20; esperar_ssh
    for _ in $(seq 60); do
      vssh 'loginctl list-sessions --no-legend | grep -q dev && na-sessao printenv WAYLAND_DISPLAY' \
        2>/dev/null | grep -q wayland && break
      sleep 5
    done
    vssh 'na-sessao sh -c "echo \$XDG_CURRENT_DESKTOP \$XDG_SESSION_TYPE \$WAYLAND_DISPLAY"' ;;
  ligar) NOME=$1; definir "$NOME"; ligar; esperar_ssh ;;
  desligar)
    NOME=$1; definir "$NOME"
    qmp '{"execute":"system_powerdown"}' >/dev/null ;;
  ssh) NOME=$1; shift; definir "$NOME"; vssh "$@" ;;
  sessao) NOME=$1; shift; definir "$NOME"; vssh "na-sessao $*" ;;
  copiar)
    NOME=$1; definir "$NOME"
    scp -q -i "$BASE/chave" -P "$PORTA" -o StrictHostKeyChecking=no \
      -o UserKnownHostsFile=/dev/null -o LogLevel=ERROR "$2" "dev@127.0.0.1:${3:-.}" ;;
  foto)
    NOME=$1; definir "$NOME"
    saida=$(realpath -m "$2")
    qmp "{\"execute\":\"screendump\",\"arguments\":{\"filename\":\"$saida\",\"format\":\"png\"}}" \
      >/dev/null ;;
  tecla)
    NOME=$1; definir "$NOME"
    teclas=$(echo "$2" | tr '-' '\n' | sed 's/.*/{"type":"qcode","data":"&"}/' | paste -sd,)
    qmp "{\"execute\":\"send-key\",\"arguments\":{\"keys\":[$teclas]}}" >/dev/null ;;
  clique)
    NOME=$1; definir "$NOME"
    x=$(($2 * 32767 / 1919)) y=$(($3 * 32767 / 1079))
    mover="{\"type\":\"abs\",\"data\":{\"axis\":\"x\",\"value\":$x}},{\"type\":\"abs\",\"data\":{\"axis\":\"y\",\"value\":$y}}"
    evento "[$mover]"; sleep 0.2
    evento '[{"type":"btn","data":{"button":"left","down":true}}]'; sleep 0.1
    evento '[{"type":"btn","data":{"button":"left","down":false}}]' ;;
  tela) NOME=$1; definir "$NOME"; exec remote-viewer "vnc://127.0.0.1:$((5900+TELA))" ;;
  *) sed -n '2,16p' "$0" | sed 's/^# \?//' ;;
esac
