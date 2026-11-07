#!/bin/bash
# Teste de ponta a ponta do daemon real num sway headless. Não prova o COSMIC.
set -u
. "$(dirname "$0")/sway-env.sh"
RAIZ=$PWD
export HOME=$XDG_RUNTIME_DIR/home SUPERCOLA_DATA_DIR=$RAIZ/data
export XDG_CONFIG_HOME=$HOME/.config XDG_DATA_HOME=$HOME/.local/share XDG_CACHE_HOME=$HOME/.cache
export PATH=$RAIZ/target/release:$PATH
mkdir -p "$HOME/.fonts" "$XDG_CONFIG_HOME/supercola"
cp data/fonts/*.ttf "$HOME/.fonts/"; fc-cache -f >/dev/null 2>&1
CFG=$XDG_CONFIG_HOME/supercola/config.toml
FOTOS=$RAIZ/dist/shots; mkdir -p "$FOTOS"
# Em 1920 × 1080, o cartão de 360 × 400 fica em (1548, 668).
PX=1548; PY=668
MAIS_X=$((PX+321)); LINHA1_Y=$((PY+157)); PINO_Y=$((PY+195))
TEXTO_X=$((PX+226)); EXCLUIR_X=$((PX+306)); ACOES_Y=$((PY+176))
LIMPAR_X=$((PX+301)); LIMPAR_Y=$((PY+113))
GIF_X=$((PX+68)); GIF_Y=$((PY+240))
L=$XDG_RUNTIME_DIR/logs; mkdir -p "$L"
falhas=0; total=0
ok(){ total=$((total+1)); echo "OK    $1"; }
ko(){ total=$((total+1)); falhas=$((falhas+1)); echo "FALHA $1"; }
checa(){ if [ "$2" = "$3" ]; then ok "$1"; else ko "$1: esperado [$3], obtido [$2]"; fi; }

campo(){ supercola-toggle status | tr ' ' '\n' | sed -n "s/^$1=//p"; }
esperar(){ # campo valor
  for _ in $(seq 40); do [ "$(campo "$1")" = "$2" ] && return 0; sleep 0.1; done; return 1
}
iniciar(){
  env "$@" supercola >> "$L/supercola.log" 2>&1 &
  SUPERCOLA=$!
  for _ in $(seq 60); do supercola-toggle status >/dev/null 2>&1 && return; sleep 0.1; done
  echo "daemon não respondeu"; cat "$L/supercola.log"; exit 1
}
parar(){ supercola-toggle quit >/dev/null 2>&1; wait $SUPERCOLA 2>/dev/null; }
copiar(){ printf '%s' "$1" | wl-copy; sleep 0.25; }
clicar(){ timeout 5 target/release/supercola-spike click "$1" "$2"; }
alvo(){ target/release/supercola-spike target > "$L/alvo" 2>&1 & ALVO=$!; sleep 1.5; }
fim_alvo(){ kill $ALVO 2>/dev/null; wait $ALVO 2>/dev/null; }
abrir(){ supercola-toggle "$@"; esperar visible true; sleep 2.5; }
ultimo(){ grep -E '^ALVO' "$L/alvo" | tail -1 | sed 's/^ALVO //'; }
hash_de(){ python3 -c "import sys;d=sys.stdin.buffer.read();h=0xcbf29ce484222325
for b in d: h=((h^b)*0x100000001b3)&0xffffffffffffffff
print('len=%d hash=%016x'%(len(d),h))"; }
descanso(){ sleep 1.2; }
# O mapa de teclas do wtype chega ao app logo antes da tecla: sem a pausa, ela se perde.
tecla(){ timeout 4 wtype -s 300 -k "$1" -s 100; }

printf 'language = "pt"\n' > "$CFG"
iniciar

echo "== histórico"
copiar 'um'; copiar 'dois'; copiar 'um'
checa "recópia não duplica" "$(campo items)" 2
for i in $(seq 1 30); do copiar "item $i"; done
checa "limite de 25" "$(campo items)" 25

echo "== privacidade"
antes=$(campo items)
printf 'senha' | wl-copy --type x-kde-passwordManagerHint; sleep 0.4
copiar 'item 30'
checa "conteúdo secreto fica fora" "$(campo items)" "$antes"

echo "== painel"
abrir
checa "abre" "$(campo visible)" true
checa "abre na área de transferência" "$(campo tab)" history
checa "busca oculta nessa aba" "$(campo search)" false
tecla Escape; esperar visible false
checa "Esc fecha" "$(campo visible)" false
abrir
clicar 200 200; esperar visible false
checa "clique fora fecha" "$(campo visible)" false
abrir; supercola-toggle >/dev/null; esperar visible false
checa "atalho alterna" "$(campo visible)" false

echo "== colar"
alvo
copiar 'texto com acentuação: ção é ü'
esperado=$(printf '%s' 'texto com acentuação: ção é ü' | hash_de)
abrir; tecla Return; descanso
checa "Enter cola o item do topo" "$(ultimo)" "$esperado"
checa "painel fecha ao colar" "$(campo visible)" false
fim_alvo

echo "== item antigo sobe"
alvo
copiar 'antigo'; copiar 'novo'
abrir; tecla Down; sleep 0.2; tecla Return; descanso
checa "reabre no topo e seta + Enter cola o segundo item" "$(ultimo)" "$(printf antigo | hash_de)"
checa "clipboard ficou com ele" "$(wl-paste -n)" antigo
fim_alvo

echo "== excluir com teclado"
copiar 'para apagar'
n=$(campo items)
abrir; tecla Delete; sleep 0.4
checa "Delete remove o item" "$(campo items)" "$((n-1))"
tecla Escape; esperar visible false

echo "== emoji"
alvo
copiar 'anterior'
n=$(campo items)
abrir emoji; timeout 4 wtype -s 300 'coracao'; sleep 0.6; tecla Return; descanso
final=$(ultimo)
if [ -n "$final" ]; then ok "emoji colado ($final)"; else ko "emoji não chegou"; fi
sleep 0.6
checa "emoji fica fora do histórico" "$(campo items)" "$n"
checa "clipboard anterior volta" "$(wl-paste -n)" anterior
fim_alvo

echo "== kaomoji e símbolos"
alvo
abrir kaomoji; tecla Right; sleep 0.2; tecla Return; descanso
checa "kaomoji cola o selecionado com a seta" "$(ultimo)" "$(printf ':-(' | hash_de)"
fim_alvo

echo "== imagem"
parar
iniciar
alvo
n=$(campo items)
grim -g "0,0 64x64" "$L/x.png"; wl-copy --type image/png < "$L/x.png"; sleep 0.5
checa "imagem entra no histórico" "$(campo items)" "$((n+1))"
printf 'outra' | wl-copy; sleep 0.3
abrir; tecla Down; sleep 0.2; tecla Return; descanso
checa "imagem volta ao clipboard igual" "$(wl-paste -n --type image/png | hash_de)" "$(hash_de < "$L/x.png")"
fim_alvo

echo "== terminal recebe Ctrl+Shift+V"
parar
printf 'language = "pt"\nterminals = ["supercola-spike"]\n' > "$CFG"
iniciar
alvo
copiar 'em terminal'
abrir; tecla Return; descanso
grep -q 'TECLA ctrl+shift+v' "$L/alvo" && ok "Ctrl+Shift+V em terminal" || { ko "terminal"; cat "$L/alvo"; }
fim_alvo

echo "== app ignorado"
parar
printf 'language = "pt"\nexcluded_apps = ["supercola-spike"]\n' > "$CFG"
iniciar
copiar 'antes'; n=$(campo items)
alvo
checa "foco detectado" "$(campo focus)" supercola-spike
copiar 'copiado no app ignorado'
checa "app ignorado fica fora" "$(campo items)" "$n"
fim_alvo

echo "== histórico desligado"
parar
printf 'language = "pt"\nhistory_enabled = false\n' > "$CFG"
iniciar
copiar 'nada'
checa "desligado não guarda" "$(campo items)" 0

echo "== gravação por cópia comum"
parar
printf 'language = "pt"\nwrite_backend = "gdk"\n' > "$CFG"
iniciar
alvo
copiar 'por cópia comum'; copiar 'mais novo'
abrir; tecla Down; sleep 0.2; tecla Return; descanso
checa "cola item antigo por cópia comum" "$(ultimo)" "$(printf 'por cópia comum' | hash_de)"
checa "cópia comum gravou no clipboard" "$(wl-paste -n)" 'por cópia comum'
fim_alvo

echo "== fixados sobrevivem ao reinício"
parar
printf 'language = "pt"\n' > "$CFG"
iniciar
copiar 'comum'; copiar 'fixo'
abrir
clicar "$MAIS_X" "$PINO_Y"; sleep 0.5
checa "fixar pelo botão" "$(campo pinned)" 1
tecla Escape; esperar visible false
parar
iniciar
checa "só o fixado volta" "$(campo items)/$(campo pinned)" 1/1
modo=$(stat -c %a "$XDG_DATA_HOME/supercola/pinned.db")
checa "banco privado" "$modo" 600
copiar 'depois do reinício'
abrir; tecla Down; sleep 0.2; tecla Return; descanso
checa "fixado pode ser colado" "$(wl-paste -n)" fixo

echo "== menu do item, busca e limpar"
parar
printf 'language = "pt"\n' > "$CFG"
iniciar
alvo
copiar 'alfa'; copiar 'beta'; copiar 'gama'
abrir
clicar "$MAIS_X" "$LINHA1_Y"; sleep 1
grim -g "1400,500 520x580" "$FOTOS/menu.png"
clicar "$TEXTO_X" "$ACOES_Y"; sleep 1
checa "⋯: colar como texto" "$(ultimo)" "$(printf gama | hash_de)"
copiar 'delta'
n=$(campo items)
abrir
clicar "$MAIS_X" "$LINHA1_Y"; sleep 1
clicar "$EXCLUIR_X" "$ACOES_Y"; sleep 0.8
checa "⋯: excluir" "$(campo items)" "$((n-1))"
timeout 4 wtype -s 300 'beta'; sleep 1
checa "digitar revela a busca" "$(campo search)" true
grim -g "1400,500 520x580" "$FOTOS/busca.png"
tecla Return; descanso
checa "busca + Enter cola o resultado" "$(wl-paste -n)" beta
fim_alvo
abrir
checa "a busca volta a ficar oculta" "$(campo search)" false
clicar "$LIMPAR_X" "$LIMPAR_Y"; sleep 0.6
checa "limpar tudo preserva só os fixados" "$(campo items)" "$(campo pinned)"
tecla Escape; esperar visible false

echo "== início"
alvo
abrir home
checa "abre no início" "$(campo tab)" home
checa "busca visível no início" "$(campo search)" true
tecla Return; descanso
checa "Enter cola o emoji usado por último" "$(ultimo)" "$final"
fim_alvo
abrir home
timeout 4 wtype -s 300 'coracao'; sleep 0.8
checa "digitar no início leva aos emojis" "$(campo tab)" emoji
tecla Escape; esperar visible false

echo "== perto do ponteiro"
parar
printf 'language = "pt"\nposition = "near-pointer"\n' > "$CFG"
iniciar
copiar 'x'
timeout 9 target/release/supercola-spike hover 400 600 7000 & PONTEIRO=$!
sleep 0.6
supercola-toggle; esperar visible true; sleep 3
grim -g "400,200 360x400" "$FOTOS/ponteiro.png"
kill $PONTEIRO 2>/dev/null; wait $PONTEIRO 2>/dev/null
supercola-toggle hide; esperar visible false
checa "perto do ponteiro abre e fecha" "$(campo visible)" false

echo "== configurações"
supercola-toggle settings; sleep 2.5
grim "$FOTOS/config.png"
ok "configurações abrem"

echo "== GIF com servidor simulado"
parar
mkdir -p "$L/www/api/v1/k/gifs" "$L/www/f"
python3 - "$L/www/f/a.gif" <<'PY'
import base64,sys
open(sys.argv[1],'wb').write(base64.b64decode('R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7'))
PY
cat > "$L/www/api/v1/k/gifs/resp.json" <<'JS'
{"result":true,"data":{"data":[{"id":1,"title":"oi","file":{
 "md":{"gif":{"url":"https://mock/f/a.gif"}},"sm":{"gif":{"url":"https://mock/f/a.gif"}}}}]}}
JS
cp "$L/www/api/v1/k/gifs/resp.json" "$L/www/api/v1/k/gifs/trending"
cp "$L/www/api/v1/k/gifs/resp.json" "$L/www/api/v1/k/gifs/search"
(cd "$L/www" && exec python3 -m http.server 18080 --bind 127.0.0.1 >/dev/null 2>&1) & WWW=$!
sleep 0.6
printf 'language = "pt"\ngif_provider = "klipy"\ngif_api_key = "k"\n' > "$CFG"
iniciar SUPERCOLA_GIF_TEST_BASE=http://127.0.0.1:18080
alvo
copiar 'antes do gif'
n=$(campo items)
abrir gif; sleep 1
clicar "$GIF_X" "$GIF_Y"; sleep 2
checa "GIF cola a URL em texto" "$(ultimo)" "$(printf 'https://mock/f/a.gif' | hash_de)"
checa "GIF fica fora do histórico" "$(campo items)" "$n"
checa "clipboard anterior volta" "$(wl-paste -n)" 'antes do gif'
fim_alvo
kill $WWW 2>/dev/null

echo "== atalho do COSMIC"
parar
rm -f "$XDG_DATA_HOME/supercola/shortcut-done"
C="$XDG_CONFIG_HOME/cosmic/com.system76.CosmicSettings.Shortcuts/v1"
mkdir -p "$C"
printf '{\n    (modifiers: [Super], key: "t"): Spawn("foot"),\n}\n' > "$C/custom"
iniciar XDG_CURRENT_DESKTOP=pop:COSMIC
grep -q 'key: "v"): Spawn("supercola-toggle")' "$C/custom" && ok "registra Super+V" || ko "registra Super+V"
grep -q 'key: "semicolon"): Spawn("supercola-toggle home")' "$C/custom" && ok "registra Super+;" || ko "registra Super+;"
grep -q 'Spawn("foot")' "$C/custom" && ok "preserva atalho existente" || ko "preserva"
[ -f "$C/custom.supercola-backup" ] && ok "faz backup" || ko "backup"
supercola --install-shortcut >/dev/null
checa "repetir não duplica" "$(grep -c supercola-toggle "$C/custom")" 2

echo "== autostart pelo atalho"
parar
supercola-toggle status >/dev/null 2>&1 && ko "daemon deveria estar parado" || ok "status não inicia o daemon"
timeout 8 supercola-toggle >/dev/null 2>&1
sleep 0.5
supercola-toggle status >/dev/null 2>&1 && ok "atalho sobe o daemon" || ko "atalho não subiu o daemon"
supercola --doctor > "$L/doctor" 2>&1; grep -q 'daemon: rodando' "$L/doctor" && ok doctor || { ko doctor; cat "$L/doctor"; }
supercola-toggle quit >/dev/null 2>&1

echo "--- log do daemon"
grep -vE 'keyvals|^$|Locale|fallback' "$L/supercola.log" | sort | uniq -c | sort -rn | head -12
echo "total=$total falhas=$falhas"
kill $(jobs -p) 2>/dev/null
exit $falhas
