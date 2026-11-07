#!/bin/bash
# Testa o .deb em uma sessão GNOME de verdade, com teclas e cliques reais. Rode na raiz do
# repositório, depois de gerar o pacote e com a VM criada: nada roda na sua sessão.
#
# Qualquer queda do GNOME Shell é falha: ela encerra a sessão do usuário e desliga as
# extensões dele.
set -u
N=${1:-ubuntu2404-gnome}
DEB=$(ls -t dist/supercola_*_amd64.deb | head -1)
FOTOS=$PWD/dist/shots; mkdir -p "$FOTOS"
UUID=supercola@hlmgdev.github.io
falhas=0; total=0

v(){ timeout 60 bash tests/vm/vm.sh "$@"; }
ok(){ total=$((total+1)); echo "OK    $1"; }
ko(){ total=$((total+1)); falhas=$((falhas+1)); echo "FALHA $1"; }
checa(){ if [ "$2" = "$3" ]; then ok "$1"; else ko "$1: esperado [$3], obtido [$2]"; fi; }
contem(){ case "$2" in *"$3"*) ok "$1";; *) ko "$1: [$3] não está em [$2]";; esac; }
campo(){ v sessao "$N" supercola-toggle status 2>/dev/null | tr ' ' '\n' | sed -n "s/^$1=//p"; }
foto(){ v foto "$N" "$FOTOS/gnome-$N-$1.png"; }
tecla(){ v tecla "$N" "$1"; sleep "${2:-0.15}"; }
digitar(){ for t in "$@"; do tecla "$t"; done; }
gs(){ v ssh "$N" "na-sessao gsettings $*" </dev/null 2>/dev/null; }
abrir(){ v ssh "$N" "nohup na-sessao $1 >/dev/null 2>&1 </dev/null & sleep ${2:-5}" </dev/null; }
fechar_apps(){ v ssh "$N" 'pkill gnome-text-edit; pkill gnome-terminal; pkill -f '[g]nome-initial-setup'; sleep 0.5; true' </dev/null; }

reiniciar(){
  bash tests/vm/vm.sh desligar "$N"
  for _ in $(seq 40); do
    kill -0 "$(cat "$HOME/.local/share/supercola-vms/$N/pid" 2>/dev/null)" 2>/dev/null || break
    sleep 2
  done
  bash tests/vm/vm.sh ligar "$N"
  for _ in $(seq 40); do
    v ssh "$N" 'na-sessao printenv WAYLAND_DISPLAY' </dev/null 2>/dev/null | grep -q wayland && break
    sleep 3
  done
  sleep 8
  # O Mutter pode não assumir o vídeo do QEMU ao iniciar: a tela fica presa no console,
  # sem nenhum quadro. Trocar de terminal refaz a configuração. Só acontece nesta VM.
  v ssh "$N" 'T=$(sudo fgconsole); sudo chvt 6; sleep 1; sudo chvt $T; sleep 2' </dev/null
  sleep 6
}

sem_queda(){
  local q
  q=$(v ssh "$N" 'journalctl --user -b --no-pager 2>/dev/null | grep -c -E "GNOME Shell crashed|assertion failed"; ls /var/crash 2>/dev/null | grep -c gnome-shell; true' </dev/null | paste -sd+ | bc)
  checa "$1: o Shell não caiu" "$q" 0
  checa "$1: extensões seguem ligadas" "$(gs get org.gnome.shell disable-user-extensions)" false
}

# Copia pelo próprio editor: o wl-copy precisa de foco no GNOME.
copiar_no_editor(){
  digitar "$@"; sleep 0.3; tecla ctrl-a 0.3; tecla ctrl-c 1.2; tecla ctrl-end 0.3
}

echo "== pacote: $DEB"
bash tests/vm/vm.sh ligar "$N"
v copiar "$N" "$DEB" /home/dev/sc.deb
# Cada rodada começa do zero, como um usuário novo.
v ssh "$N" 'na-sessao supercola-toggle quit >/dev/null 2>&1
  sudo apt-get purge -y -q supercola >/dev/null 2>&1
  rm -rf ~/.local/share/supercola
  # Uma sessão antiga do editor pode reabrir texto de outra rodada no lugar do arquivo novo.
  pkill gnome-text-edit; rm -rf ~/.local/state/gnome-text-editor
  sudo rm -f /var/crash/*
  sudo apt-get install -y -q kbd >/dev/null 2>&1
  sudo systemctl mask systemd-networkd-wait-online.service 2>/dev/null
  echo yes > ~/.config/gnome-initial-setup-done
  na-sessao gsettings set org.gnome.desktop.session idle-delay 0
  na-sessao gsettings set org.gnome.desktop.screensaver lock-enabled false
  na-sessao gsettings reset org.gnome.shell enabled-extensions
  na-sessao gsettings reset org.gnome.shell disable-user-extensions
  na-sessao gsettings reset org.gnome.shell.keybindings toggle-message-tray
  na-sessao gsettings reset org.gnome.settings-daemon.plugins.media-keys custom-keybindings
  na-sessao gsettings set org.gnome.TextEditor restore-session false' </dev/null
echo "GNOME: $(v ssh "$N" 'na-sessao gnome-shell --version' </dev/null)"
# A sessão precisa nascer sem o pacote: um Shell que já viu a extensão a liga na hora.
reiniciar

echo "== instalado com a sessão aberta: a extensão só carrega na próxima"
fechar_apps
v ssh "$N" 'sudo apt-get install -y -q /home/dev/sc.deb >/dev/null 2>&1
  dpkg-query -W -f="\${Package} \${Version}\n" supercola' </dev/null
abrir supercola 5
foto aviso-sessao
contem "o daemon habilitou a extensão" "$(gs get org.gnome.shell enabled-extensions)" "$UUID"
# O GNOME pode ligar a extensão na hora ou só na próxima sessão: os dois caminhos valem,
# desde que o doctor diga a verdade.
doc=$(v sessao "$N" supercola --doctor 2>&1)
case "$doc" in
  *'saia e entre'*|*'extensão do GNOME: ativa'*) ok "o doctor informa o estado da extensão";;
  *) ko "o doctor informa o estado da extensão: [$doc]";;
esac
checa "sem extensão, não marca o primeiro uso" \
  "$(v ssh "$N" 'test -f ~/.local/share/supercola/shortcut-done && echo sim || echo nao' </dev/null)" nao

echo "== nova sessão: a extensão liga sozinha, sem pedir outra"
# Como quem instala e reinicia: o Shell já conhece a extensão, e o autostart a habilita.
gs reset org.gnome.shell enabled-extensions
reiniciar
fechar_apps
checa "o autostart iniciou o daemon" "$(v ssh "$N" 'pgrep -xc supercola' </dev/null)" 1
contem "o daemon habilitou a extensão" "$(gs get org.gnome.shell enabled-extensions)" "$UUID"
doc=$(v sessao "$N" supercola --doctor 2>&1)
contem "doctor: extensão ativa" "$doc" 'OK      extensão do GNOME: ativa'
contem "doctor: daemon" "$doc" 'daemon: rodando'
case "$doc" in *AUSENTE*) ko "doctor sem itens ausentes";; *) ok "doctor sem itens ausentes";; esac
foto dialogo
contem "antes de responder, as notificações seguem em Super+V" \
  "$(gs get org.gnome.shell.keybindings toggle-message-tray)" '<Super>v'
tecla esc 1.5
contem "recusar mantém as notificações" \
  "$(gs get org.gnome.shell.keybindings toggle-message-tray)" '<Super>v'
checa "a recusa não pergunta de novo" \
  "$(v ssh "$N" 'test -f ~/.local/share/supercola/shortcut-done && echo sim' </dev/null)" sim
tecla meta_l-semicolon 2
checa "Super+; abre o painel" "$(campo visible)" true
checa "Super+; abre o início" "$(campo tab)" home
tecla esc 1
checa "Esc fecha o painel" "$(campo visible)" false

echo "== diálogo do Super+V: aceitar"
v ssh "$N" 'na-sessao supercola-toggle quit; rm -f ~/.local/share/supercola/shortcut-done; sleep 1' </dev/null
abrir supercola 4
tecla ret 1.5
checa "aceitar tira Super+V das notificações" \
  "$(gs get org.gnome.shell.keybindings toggle-message-tray)" "['<Super>m']"

echo "== colar no editor"
v ssh "$N" 'rm -f /tmp/a.txt; touch /tmp/a.txt' </dev/null
abrir 'gnome-text-editor --new-window /tmp/a.txt' 5
checa "foco no editor" "$(campo focus)" org.gnome.TextEditor
antes=$(campo items)
copiar_no_editor c o p i a
checa "o Ctrl+C entrou no histórico" "$(campo items)" "$((antes+1))"
tecla meta_l-v 2
checa "Super+V abre o histórico" "$(campo tab)" history
checa "o painel abriu" "$(campo visible)" true
checa "o foco segue no app de origem" "$(campo focus)" org.gnome.TextEditor
foto painel
tecla ret 1.5
checa "escolher fecha o painel" "$(campo visible)" false
digitar x y
tecla ctrl-s 1
checa "colou e digitou depois" "$(v ssh "$N" 'cat /tmp/a.txt' </dev/null)" 'copiacopiaxy'

echo "== colar no terminal"
fechar_apps
v ssh "$N" 'rm -f /tmp/b.txt' </dev/null
abrir "gnome-terminal --wait -- sh -c 'cat > /tmp/b.txt'" 4
checa "foco no terminal" "$(campo focus)" org.gnome.Terminal
foto terminal
tecla meta_l-v 2
tecla ret 1.5
foto terminal-colado
digitar q; tecla ret; tecla ctrl-d 1
checa "o terminal recebeu com Ctrl+Shift+V" \
  "$(v ssh "$N" 'tr -d "\n" < /tmp/b.txt' </dev/null)" 'copiaq'
fechar_apps
sem_queda "depois das colagens"

echo "== emoji devolve o clipboard"
v ssh "$N" 'rm -f /tmp/c.txt; touch /tmp/c.txt' </dev/null
abrir 'gnome-text-editor --new-window /tmp/c.txt' 5
antes=$(campo items)
# Só nomes de formatos, para explicar uma falha aqui. O conteúdo não passa por este log.
v ssh "$N" '(nohup na-sessao timeout 25 dbus-monitor --session "destination=dev.supercola.Daemon" \
  "destination=org.gnome.Shell,path=/dev/supercola/Shell" >/tmp/formatos.log 2>&1 </dev/null &); sleep 1; true' </dev/null
tecla meta_l-semicolon 2
v sessao "$N" supercola-toggle emoji >/dev/null 2>&1; sleep 2
foto emoji
tecla ret 2
tecla ctrl-v 1
tecla ctrl-s 1
conteudo=$(v ssh "$N" 'cat /tmp/c.txt' </dev/null)
contem "o clipboard anterior voltou" "$conteudo" 'copia'
depois=$(campo items)
checa "o emoji não entrou no histórico" "$depois" "$antes"
[ "$depois" = "$antes" ] || v ssh "$N" "grep -E 'member=(SelectionChanged|ReadSelection|SetSelection|Paste)|string \"[a-zA-Z/_.-]+(;charset=utf-8)?\"' /tmp/formatos.log | grep -v -E 'string \"(org\.|version|dev\.supercola)' | sed -E 's/^.*member=/-- /; s/^ +//'" </dev/null
fechar_apps

echo "== imagem"
v copiar "$N" tests/vm/png.py /tmp/png.py
v ssh "$N" 'python3 /tmp/png.py /tmp/t.png' </dev/null
hash_png=$(v ssh "$N" 'sha256sum < /tmp/t.png | cut -d" " -f1' </dev/null)
v ssh "$N" 'rm -f /tmp/d.txt; touch /tmp/d.txt' </dev/null
abrir 'gnome-text-editor --new-window /tmp/d.txt' 5
antes=$(campo items)
v ssh "$N" "nohup na-sessao sh -c 'wl-copy --type image/png < /tmp/t.png' >/dev/null 2>&1 </dev/null & sleep 2" </dev/null
checa "a imagem entrou no histórico" "$(campo items)" "$((antes+1))"
tecla meta_l-v 2
foto imagem
tecla ret 2
checa "a imagem volta intacta" \
  "$(v ssh "$N" 'timeout 5 na-sessao wl-paste --type image/png </dev/null | sha256sum | cut -d" " -f1' </dev/null)" "$hash_png"
v ssh "$N" 'pkill wl-copy; true' </dev/null

echo "== XWayland, em um app X11 de verdade"
# O compositor só faz a ponte com o app X11 em foco: um `xclip` solto não prova nada.
fechar_apps
v ssh "$N" 'rm -f /tmp/x.txt; touch /tmp/x.txt' </dev/null
abrir 'env GDK_BACKEND=x11 gnome-text-editor --standalone /tmp/x.txt' 6
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

echo "== clique fora"
abrir 'gnome-text-editor --new-window /tmp/a.txt' 4
tecla meta_l-v 2
checa "painel aberto" "$(campo visible)" true
v clique "$N" 300 300; sleep 1.5
checa "clicar fora fecha o painel" "$(campo visible)" false
# Sem ícones na área de trabalho, como no Debian, o clique abre o menu do fundo do GNOME,
# que captura o teclado até fechar.
tecla esc 0.5
fechar_apps

echo "== aberturas em sequência"
v ssh "$N" 'for i in $(seq 20); do na-sessao supercola-toggle; sleep 0.4; done' </dev/null
sleep 2
sem_queda "depois de 20 aberturas"

echo "== restaurar"
contem "--restore-shortcuts confirma" "$(v sessao "$N" supercola --restore-shortcuts 2>&1)" 'voltou'
sleep 1
contem "Super+V voltou às notificações" \
  "$(gs get org.gnome.shell.keybindings toggle-message-tray)" '<Super>v'
contem "restaurar duas vezes é inofensivo" "$(v sessao "$N" supercola --restore-shortcuts 2>&1)" 'Nenhum atalho'
# Depois das aberturas em sequência, o painel pode ter ficado aberto, e a tecla o fecharia.
v sessao "$N" supercola-toggle hide >/dev/null 2>&1; sleep 1
tecla meta_l-semicolon 2
checa "Super+; continua funcionando" "$(campo visible)" true
tecla esc 1

# Fica por último: as teclas do teste são físicas, e no Dvorak o `;` vira S. `Super+S` abre as
# Configurações rápidas, um menu que captura o teclado até fechar.
echo "== layouts de teclado"
# O Ctrl+V precisa achar a tecla certa em cada layout. O painel abre por comando porque
# as teclas do teste são físicas: no Dvorak, o V físico é o K, e o S é o ponto e vírgula.
for layout in "br|s" "us+dvorak|semicolon"; do
  nome=${layout%|*}; salvar=${layout#*|}
  v ssh "$N" "na-sessao gsettings set org.gnome.desktop.input-sources sources \"[('xkb', '$nome')]\"; rm -f /tmp/l.txt; touch /tmp/l.txt" </dev/null
  sleep 1.5
  abrir 'gnome-text-editor --new-window /tmp/l.txt' 5
  v sessao "$N" supercola-toggle >/dev/null 2>&1; sleep 2
  tecla ret 2
  tecla "ctrl-$salvar" 1.5
  checa "colar no layout $nome" "$(v ssh "$N" 'cat /tmp/l.txt' </dev/null)" "ponte$A$B"
  fechar_apps
done
v ssh "$N" "na-sessao gsettings set org.gnome.desktop.input-sources sources \"[('xkb', 'us')]\"; sleep 1
  na-sessao gsettings reset org.gnome.desktop.input-sources sources" </dev/null
sleep 1

echo "== remoção"
v ssh "$N" 'na-sessao supercola-toggle quit; sudo apt-get purge -y -q supercola >/dev/null 2>&1; true' </dev/null
checa "a extensão saiu do sistema" \
  "$(v ssh "$N" "ls -d /usr/share/gnome-shell/extensions/$UUID 2>/dev/null | wc -l" </dev/null)" 0
sem_queda "depois da remoção"

echo "total=$total falhas=$falhas"
exit "$falhas"
