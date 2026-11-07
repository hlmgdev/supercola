#!/bin/bash
# Testa o .deb em uma sessão KDE de verdade, com teclas e cliques reais. Rode na raiz do
# repositório, depois de gerar o pacote e com a VM criada: nada roda na sua sessão.
set -u
N=${1:-debian13-kde}
DEB=$(ls -t dist/supercola_*_amd64.deb | head -1)
FOTOS=$PWD/dist/shots; mkdir -p "$FOTOS"
falhas=0; total=0
SUPER_V=268435542

v(){ bash tests/vm/vm.sh "$@"; }
ok(){ total=$((total+1)); echo "OK    $1"; }
ko(){ total=$((total+1)); falhas=$((falhas+1)); echo "FALHA $1"; }
checa(){ if [ "$2" = "$3" ]; then ok "$1"; else ko "$1: esperado [$3], obtido [$2]"; fi; }
contem(){ case "$2" in *"$3"*) ok "$1";; *) ko "$1: [$3] não está em [$2]";; esac; }
campo(){ v sessao "$N" supercola-toggle status 2>/dev/null | tr ' ' '\n' | sed -n "s/^$1=//p"; }
foto(){ v foto "$N" "$FOTOS/kde-$1.png"; }
tecla(){ v tecla "$N" "$1"; sleep "${2:-0.15}"; }
digitar(){ for t in "$@"; do tecla "$t"; done; }
dono(){ v ssh "$N" "na-sessao busctl --user call org.kde.kglobalaccel /kglobalaccel \
  org.kde.KGlobalAccel getGlobalShortcutsByKey i $SUPER_V" </dev/null; }
# O tipo explícito importa: sem ele, o wl-copy desta VM anuncia formatos ilegíveis.
copiar(){
  v ssh "$N" "nohup na-sessao sh -c 'printf %s \"$1\" | wl-copy --type \"${2:-text/plain;charset=utf-8}\"' \
    >/dev/null 2>&1 </dev/null & sleep 1" </dev/null
}
fim_copia(){ v ssh "$N" 'pkill wl-copy; true' </dev/null; }
abrir(){ # comando em background na sessão
  v ssh "$N" "nohup na-sessao $1 >/dev/null 2>&1 </dev/null & sleep ${2:-5}" </dev/null
}
fechar_apps(){ v ssh "$N" 'pkill kate; pkill konsole; sleep 0.5; true' </dev/null; }
reiniciar(){
  v ssh "$N" 'sudo systemctl reboot' </dev/null 2>/dev/null
  sleep 15
  for _ in $(seq 40); do
    [ "$(v ssh "$N" 'pgrep -c supercola' </dev/null 2>/dev/null)" -ge 1 ] 2>/dev/null && break
    sleep 3
  done
  sleep 8
}

echo "== pacote: $DEB"
v ligar "$N"
v copiar "$N" "$DEB" /tmp/sc.deb
# Cada rodada começa do zero, como um usuário novo.
v ssh "$N" 'na-sessao supercola-toggle quit >/dev/null 2>&1
  sudo apt-get purge -y -q supercola >/dev/null 2>&1
  sudo rm -f /usr/share/applications/dev.supercola.*.desktop
  rm -rf ~/.local/share/supercola
  sed -i "/^\[services\]\[dev.supercola/,/^$/d; /^\[dev.supercola/,/^$/d" ~/.config/kglobalshortcutsrc
  sudo apt-get install -y -q /tmp/sc.deb >/dev/null 2>&1
  kwriteconfig6 --file kscreenlockerrc --group Daemon --key Autolock false
  kwriteconfig6 --file kscreenlockerrc --group Daemon --key LockOnResume false
  dpkg-query -W -f="\${Package} \${Version}\n" supercola' </dev/null
reiniciar

echo "== instalação"
checa "o autostart iniciou o daemon" "$(v ssh "$N" 'pgrep -xc supercola' </dev/null)" 1
doc=$(v sessao "$N" supercola --doctor 2>&1)
contem "doctor: colar pelo fake_input" "$doc" 'OK      colar sozinho: org_kde_kwin_fake_input'
contem "doctor: script do KWin" "$doc" 'OK      detectar terminais: script do KWin'
contem "doctor: painel" "$doc" 'OK      painel flutuante'
contem "doctor: clipboard" "$doc" 'OK      monitorar o clipboard'
contem "doctor: daemon" "$doc" 'daemon: rodando'
case "$doc" in *AUSENTE*) ko "doctor sem itens ausentes";; *) ok "doctor sem itens ausentes";; esac

echo "== diálogo do Super+V: recusar"
foto dialogo
contem "antes de responder, Super+V segue com o Klipper" "$(dono)" 'show-on-mouse-pos'
tecla esc 1.5
contem "recusar mantém o Klipper" "$(dono)" 'show-on-mouse-pos'
checa "a recusa não pergunta de novo" \
  "$(v ssh "$N" 'test -f ~/.local/share/supercola/shortcut-done && echo sim' </dev/null)" sim
checa "recusar não cria backup" \
  "$(v ssh "$N" 'test -e ~/.local/share/supercola/atalho-substituido && echo sim || echo nao' </dev/null)" nao
tecla meta_l-semicolon 2
checa "Super+; abre o painel" "$(campo visible)" true
checa "Super+; abre o início" "$(campo tab)" home
foto painel-inicio
tecla esc 1
checa "Esc fecha o painel" "$(campo visible)" false

echo "== diálogo do Super+V: aceitar"
v ssh "$N" 'na-sessao supercola-toggle quit; rm -f ~/.local/share/supercola/shortcut-done; sleep 1' </dev/null
abrir supercola 4
foto dialogo-2
tecla ret 1.5
contem "aceitar passa Super+V para o Super Cola" "$(dono)" 'dev.supercola.Toggle.desktop'
case "$(dono)" in *show-on-mouse-pos*) ko "o Klipper perdeu Super+V";; *) ok "o Klipper perdeu Super+V";; esac

echo "== colar no Kate"
v ssh "$N" 'rm -f /tmp/a.txt; touch /tmp/a.txt' </dev/null
abrir 'kate -n /tmp/a.txt' 6
checa "foco no Kate" "$(campo focus)" org.kde.kate
# O histórico já começa com o que estava no clipboard, por isso a conta é relativa.
antes=$(campo items)
copiar 'colagem-no-kate'
checa "o item entrou no histórico" "$(campo items)" "$((antes+1))"
tecla meta_l-v 2
checa "Super+V abre o histórico" "$(campo tab)" history
checa "o painel abriu" "$(campo visible)" true
checa "o foco segue no app de origem" "$(campo focus)" org.kde.kate
foto painel-historico
tecla ret 1.5
checa "escolher fecha o painel" "$(campo visible)" false
# A primeira letra logo depois de colar não pode se perder.
digitar x y
tecla ctrl-s 1
checa "colou e digitou depois" "$(v ssh "$N" 'cat /tmp/a.txt' </dev/null)" 'colagem-no-katexy'
fim_copia

echo "== copiar no Kate, com Ctrl+C real"
antes=$(campo items)
digitar ctrl-a; tecla ctrl-c 1.5
checa "a cópia real entrou no histórico" "$(campo items)" "$((antes+1))"

echo "== colar no terminal"
fechar_apps
v ssh "$N" 'rm -f /tmp/b.txt' </dev/null
abrir "konsole -e sh -c 'cat > /tmp/b.txt'" 5
checa "foco no Konsole" "$(campo focus)" org.kde.konsole
copiar 'colagem-no-terminal'
tecla meta_l-v 2
tecla ret 1.5
digitar q; tecla ret; tecla ctrl-d 1
checa "o terminal recebeu com Ctrl+Shift+V" \
  "$(v ssh "$N" 'tr -d "\n" < /tmp/b.txt' </dev/null)" 'colagem-no-terminalq'
fim_copia
fechar_apps

echo "== imagem"
v copiar "$N" tests/vm/png.py /tmp/png.py
v ssh "$N" 'python3 /tmp/png.py /tmp/t.png' </dev/null
hash_png=$(v ssh "$N" 'sha256sum < /tmp/t.png | cut -d" " -f1' </dev/null)
v ssh "$N" 'rm -f /tmp/d.txt; touch /tmp/d.txt' </dev/null
abrir 'kate -n /tmp/d.txt' 5
antes=$(campo items)
v ssh "$N" "nohup na-sessao sh -c 'wl-copy --type image/png < /tmp/t.png' >/dev/null 2>&1 </dev/null & sleep 2" </dev/null
checa "a imagem entrou no histórico" "$(campo items)" "$((antes+1))"
# O wl-copy segue vivo: sem dono, o Klipper devolve o texto dele ao topo do histórico.
tecla meta_l-v 2
foto imagem
tecla ret 2
checa "a imagem volta intacta" \
  "$(v ssh "$N" 'timeout 5 na-sessao wl-paste --type image/png </dev/null | sha256sum | cut -d" " -f1' </dev/null)" "$hash_png"

echo "== XWayland, em um app X11 de verdade"
# O compositor só faz a ponte com o app X11 em foco: um `xclip` solto não prova nada.
fechar_apps
v ssh "$N" 'rm -f /tmp/x.txt; touch /tmp/x.txt' </dev/null
abrir 'env QT_QPA_PLATFORM=xcb kate -n /tmp/x.txt' 6
foto xwayland
antes=$(campo items)
# Texto novo a cada rodada: copiar o que já está no histórico só sobe o item para o topo, e
# o Klipper devolve o último texto da rodada anterior ao iniciar.
A=$(date +%S | cut -c1); B=$(date +%S | cut -c2)
digitar p o n t e "$A" "$B"; tecla ctrl-a 0.3; tecla ctrl-c 1.5; tecla ctrl-end 0.3
checa "cópia de app X11 entrou no histórico" "$(campo items)" "$((antes+1))"
v sessao "$N" supercola-toggle >/dev/null 2>&1; sleep 2
tecla ret 2
tecla ctrl-s 1.2
checa "o app X11 recebeu a colagem" "$(v ssh "$N" 'cat /tmp/x.txt' </dev/null)" "ponte$A${B}ponte$A$B"
fechar_apps

echo "== layouts de teclado"
# O Ctrl+V precisa achar a tecla certa em cada layout. O painel abre por comando porque
# as teclas do teste são físicas: no Dvorak, o V físico é o K, e o S é o ponto e vírgula.
for layout in "br||s" "us|dvorak|semicolon"; do
  mapa=${layout%%|*}; resto=${layout#*|}; variante=${resto%|*}; salvar=${resto#*|}
  v ssh "$N" "kwriteconfig6 --file kxkbrc --group Layout --key Use true
    kwriteconfig6 --file kxkbrc --group Layout --key LayoutList '$mapa'
    kwriteconfig6 --file kxkbrc --group Layout --key VariantList '$variante'
    na-sessao dbus-send --session --type=signal --reply-timeout=100 /Layouts org.kde.keyboard.reloadConfig
    rm -f /tmp/l.txt; touch /tmp/l.txt" </dev/null
  sleep 2
  abrir 'kate -n /tmp/l.txt' 5
  v sessao "$N" supercola-toggle >/dev/null 2>&1; sleep 2
  tecla ret 2
  tecla "ctrl-$salvar" 1.5
  checa "colar no layout $mapa $variante" "$(v ssh "$N" 'cat /tmp/l.txt' </dev/null)" "ponte$A$B"
  fechar_apps
done
v ssh "$N" 'kwriteconfig6 --file kxkbrc --group Layout --key LayoutList us
  kwriteconfig6 --file kxkbrc --group Layout --key VariantList ""
  na-sessao dbus-send --session --type=signal --reply-timeout=100 /Layouts org.kde.keyboard.reloadConfig' </dev/null
sleep 2

echo "== privacidade"
antes=$(campo items)
copiar 'senha-secreta' 'x-kde-passwordManagerHint'
checa "item marcado como senha fica de fora" "$(campo items)" "$antes"
fim_copia

echo "== clique fora"
tecla meta_l-v 2
checa "painel aberto" "$(campo visible)" true
v clique "$N" 400 400; sleep 1.5
checa "clicar fora fecha o painel" "$(campo visible)" false

echo "== restaurar"
saida=$(v sessao "$N" supercola --restore-shortcuts 2>&1)
contem "--restore-shortcuts confirma" "$saida" 'voltou para'
sleep 1
contem "Super+V voltou ao Klipper" "$(dono)" 'show-on-mouse-pos'
case "$(dono)" in *dev.supercola*) ko "o Super Cola largou Super+V";; *) ok "o Super Cola largou Super+V";; esac
contem "restaurar duas vezes é inofensivo" "$(v sessao "$N" supercola --restore-shortcuts 2>&1)" 'Nenhum atalho'
tecla meta_l-semicolon 2
checa "Super+; continua funcionando" "$(campo visible)" true
tecla esc 1

echo "== remoção"
v ssh "$N" 'na-sessao supercola-toggle quit; sudo apt-get purge -y -q supercola >/dev/null 2>&1; true' </dev/null
checa "o pacote saiu" "$(v ssh "$N" 'dpkg-query -W -f="\${Status}" supercola 2>/dev/null | grep -c "ok installed"' </dev/null)" 0
checa "sem .desktop sobrando" "$(v ssh "$N" 'ls /usr/share/applications/dev.supercola.* 2>/dev/null | wc -l' </dev/null)" 0
contem "Klipper intacto depois da remoção" "$(dono)" 'show-on-mouse-pos'

echo "total=$total falhas=$falhas"
exit "$falhas"
