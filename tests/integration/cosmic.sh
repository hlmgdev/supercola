#!/bin/bash
# Testa o daemon real na sessão COSMIC em uso. Rode a partir da raiz do repositório.
#
# Dados, configuração e socket ficam em uma pasta temporária, então nada do que é seu muda,
# exceto o clipboard. Durante cerca de 1 minuto, não use o teclado nem o mouse.
set -u
RAIZ=$PWD
case "${XDG_CURRENT_DESKTOP:-}" in *COSMIC*) ;; *) echo "rode em uma sessão COSMIC"; exit 2;; esac
T=$(mktemp -d)
L=$T/logs
mkdir -p "$L" "$T/run" "$T/config/supercola" "$T/data" "$T/cache"
chmod 700 "$T/run"
# Caminho absoluto: o socket do compositor continua no XDG_RUNTIME_DIR de verdade.
case "$WAYLAND_DISPLAY" in /*) ;; *) export WAYLAND_DISPLAY=$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY;; esac
export XDG_RUNTIME_DIR=$T/run XDG_CONFIG_HOME=$T/config XDG_DATA_HOME=$T/data XDG_CACHE_HOME=$T/cache
export SUPERCOLA_DATA_DIR=$RAIZ/data DBUS_SESSION_BUS_ADDRESS=disabled: GDK_BACKEND=wayland
export PATH=$RAIZ/target/release:$PATH
CFG=$XDG_CONFIG_HOME/supercola/config.toml
SPIKE=supercola-spike
trap 'kill $(jobs -p) 2>/dev/null; rm -rf "$T"' EXIT

falhas=0; total=0
ok(){ total=$((total+1)); echo "OK    $1"; }
ko(){ total=$((total+1)); falhas=$((falhas+1)); echo "FALHA $1"; }
checa(){ if [ "$2" = "$3" ]; then ok "$1"; else ko "$1: esperado [$3], obtido [$2]"; fi; }

campo(){ supercola-toggle status | tr ' ' '\n' | sed -n "s/^$1=//p"; }
esperar(){ for _ in $(seq 40); do [ "$(campo "$1")" = "$2" ] && return 0; sleep 0.1; done; return 1; }
iniciar(){
  supercola >> "$L/supercola.log" 2>&1 &
  SUPERCOLA=$!
  for _ in $(seq 60); do supercola-toggle status >/dev/null 2>&1 && return; sleep 0.1; done
  echo "daemon não respondeu"; cat "$L/supercola.log"; exit 1
}
parar(){ supercola-toggle quit >/dev/null 2>&1; wait "$SUPERCOLA" 2>/dev/null; }
copiar(){ printf '%s' "$1" | wl-copy; sleep 0.3; }
tecla(){ "$SPIKE" tecla "$1"; sleep 0.15; }
alvo(){ # opções de ambiente do alvo
  env "$@" "$SPIKE" target > "$L/alvo" 2>&1 & ALVO=$!
  for _ in $(seq 50); do
    grep 'FOCO-ALVO' "$L/alvo" 2>/dev/null | tail -1 | grep -q 'ativo=true' && { sleep 0.2; return 0; }
    sleep 0.1
  done
  echo "a janela-alvo não recebeu o foco"; exit 1
}
fim_alvo(){ kill "$ALVO" 2>/dev/null; wait "$ALVO" 2>/dev/null; sleep 0.2; }
abrir(){ supercola-toggle "$@"; esperar visible true; sleep 0.3; }
ultimo(){ grep -E '^ALVO' "$L/alvo" | tail -1 | sed 's/^ALVO //'; }
hash_de(){ python3 -c "import sys;d=sys.stdin.buffer.read();h=0xcbf29ce484222325
for b in d: h=((h^b)*0x100000001b3)&0xffffffffffffffff
print('len=%d hash=%016x'%(len(d),h))"; }
descanso(){ sleep 0.8; }
grande(){ python3 -c "print(('0123456789abcdef'*20000)[:$1],end='')"; }

printf 'language = "pt"\n' > "$CFG"
iniciar

echo "== histórico"
# O daemon também guarda o que já estava no clipboard ao iniciar.
base=$(campo items)
copiar 'um'; copiar 'dois'; copiar 'um'
checa "lê o clipboard e não duplica" "$(campo items)" "$((base+2))"
checa "grava por data-control" "$(campo clipboard)" ext-data-control

echo "== colar em app nativo"
alvo GDK_BACKEND=wayland
checa "detecta o app em foco" "$(campo focus)" supercola-spike
copiar 'texto com acentuação: ção é ü'
abrir; tecla Return; descanso
checa "Enter cola o item do topo" "$(ultimo)" "$(printf '%s' 'texto com acentuação: ção é ü' | hash_de)"
grep -q 'TECLA ctrl+v' "$L/alvo" && ok "o app recebe Ctrl+V" || ko "o app não recebeu Ctrl+V"
checa "painel fecha ao colar" "$(campo visible)" false
fim_alvo

echo "== item antigo"
alvo GDK_BACKEND=wayland
copiar 'antigo'; copiar 'novo'
abrir; tecla Down; tecla Return; descanso
checa "seta e Enter colam o segundo item" "$(ultimo)" "$(printf antigo | hash_de)"
checa "o clipboard fica com ele" "$(wl-paste -n)" antigo
fim_alvo

echo "== texto grande"
alvo GDK_BACKEND=wayland
grande 300000 | wl-copy; sleep 0.5
copiar 'mais novo'
abrir; tecla Down; tecla Return; sleep 1.5
checa "300 KB colam inteiros" "$(ultimo)" "$(grande 300000 | hash_de)"
fim_alvo

echo "== app XWayland"
alvo GDK_BACKEND=x11
copiar 'para o xwayland'
abrir; tecla Return; descanso
checa "cola em app XWayland" "$(ultimo)" "$(printf 'para o xwayland' | hash_de)"
fim_alvo

echo "== depois de abrir, o painel libera o foco exclusivo"
alvo GDK_BACKEND=wayland
copiar 'com foco aberto'
abrir; sleep 0.5
checa "o painel segue aberto" "$(campo visible)" true
tecla Return; descanso
checa "o teclado continua no painel" "$(ultimo)" "$(printf 'com foco aberto' | hash_de)"
abrir; sleep 0.5
"$SPIKE" target > "$L/alvo2" 2>&1 & OUTRA=$!
esperar visible false
checa "perder o foco fecha o painel" "$(campo visible)" false
kill "$OUTRA" 2>/dev/null; wait "$OUTRA" 2>/dev/null
fim_alvo

echo "== emoji e clipboard anterior"
alvo GDK_BACKEND=wayland
copiar 'anterior'
n=$(campo items)
abrir emoji; tecla c; tecla o; tecla r; sleep 0.5; tecla Return; descanso
emoji=$(ultimo)
[ -n "$emoji" ] && ok "emoji colado ($emoji)" || ko "o emoji não chegou"
sleep 0.6
checa "emoji fica fora do histórico" "$(campo items)" "$n"
checa "clipboard anterior volta" "$(wl-paste -n)" anterior
antes=$(grep -c '^ALVO' "$L/alvo")
"$SPIKE" paste; sleep 0.6
depois=$(grep -c '^ALVO' "$L/alvo")
tamanho=$(ultimo | sed -E 's/^len=([0-9]+).*/\1/')
[ "$depois" -gt "$antes" ] && [ "$tamanho" -ge 8 ] \
  && ok "o app em foco também recebe o clipboard anterior" \
  || ko "o app em foco não recebeu o clipboard anterior ($(ultimo))"
fim_alvo

echo "== NumLock depois de colar"
LED=$(ls /sys/class/leds/*numlock/brightness 2>/dev/null | head -1)
if [ -n "$LED" ]; then
  inicial=$(cat "$LED")
  alvo GDK_BACKEND=wayland
  [ "$inicial" = 0 ] && tecla Num_Lock
  sleep 0.3
  checa "NumLock ligado antes de colar" "$(cat "$LED")" 1
  copiar 'com numlock'
  abrir; tecla Return; descanso
  checa "colou com o NumLock ligado" "$(ultimo)" "$(printf 'com numlock' | hash_de)"
  checa "NumLock continua ligado" "$(cat "$LED")" 1
  [ "$inicial" = 0 ] && tecla Num_Lock
  fim_alvo
else
  echo "sem LED de NumLock para conferir"
fi

echo "== terminal"
parar
printf 'language = "pt"\nterminals = ["supercola-spike"]\n' > "$CFG"
iniciar
alvo GDK_BACKEND=wayland
copiar 'em terminal'
abrir; tecla Return; descanso
grep -q 'TECLA ctrl+shift+v' "$L/alvo" && ok "terminal recebe Ctrl+Shift+V" || ko "terminal não recebeu Ctrl+Shift+V"
fim_alvo

echo "== app ignorado"
parar
printf 'language = "pt"\nexcluded_apps = ["supercola-spike"]\n' > "$CFG"
iniciar
copiar 'antes'
n=$(campo items)
alvo GDK_BACKEND=wayland
copiar 'copiado no app ignorado'
checa "app ignorado fica fora" "$(campo items)" "$n"
fim_alvo

echo "== cópia comum, a opção alternativa"
parar
printf 'language = "pt"\nwrite_backend = "gdk"\n' > "$CFG"
iniciar
alvo GDK_BACKEND=wayland
copiar 'por cópia comum'; copiar 'mais novo'
abrir; tecla Down; tecla Return; descanso
checa "cola por cópia comum" "$(ultimo)" "$(printf 'por cópia comum' | hash_de)"
fim_alvo

parar
echo "--- log do daemon"
grep -vE 'keyvals|^$|Locale|fallback|Theme|dconf' "$L/supercola.log" | sort | uniq -c | sort -rn | head -10
echo "total=$total falhas=$falhas"
exit "$falhas"
