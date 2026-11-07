#!/bin/bash
# Kit de validação: roda num Pop!_OS 24.04 COSMIC e gera spike-report-*.txt.
# O relatório traz protocolos, versões, tamanhos e hashes. Nunca traz conteúdo do clipboard.
#
#   bash spike/run.sh              matriz automática de colagem, cerca de 1 minuto
#   bash spike/run.sh --completo   inclui os testes que precisam de você (digitar e clicar)
#
# Durante a matriz, não use o teclado nem o mouse: as janelas de teste precisam do foco.
# O clipboard atual é substituído pelo conteúdo dos testes.
cd "$(dirname "$0")" || exit 1
# O terminal do VS Code exporta GDK_BACKEND=x11, e o painel só existe em Wayland.
export GDK_BACKEND=wayland
BIN=${BIN:-../target/release/supercola-spike}
COMPLETO=; [ "${1:-}" = "--completo" ] && COMPLETO=1
CONHECIDO=
REL="$PWD/spike-report-$(date +%Y%m%d-%H%M%S).txt"
TMP=$(mktemp -d)
trap 'kill $(jobs -p) 2>/dev/null; rm -rf "$TMP"' EXIT

diz(){ echo "$*" | tee -a "$REL"; }
sim(){ # pergunta -> grava a resposta
  local r
  read -r -p "$1 [s/n] " r
  case "$r" in [sS]*) diz "OK    $1";; *) diz "FALHA $1";; esac
}
falhas=0

diz "=== sistema"
diz "data: $(date -Is)"
diz "$(. /etc/os-release; echo "$PRETTY_NAME")"
diz "sessão: ${XDG_CURRENT_DESKTOP:-?} / ${XDG_SESSION_TYPE:-?}"
diz "cosmic-comp: $(dpkg-query -W -f='${Version}' cosmic-comp 2>/dev/null || echo '?')"
diz "gtk4: $(dpkg-query -W -f='${Version}' libgtk-4-1 2>/dev/null || echo '?')"

diz; diz "=== protocolos"
$BIN probe | tee "$TMP/probe" | grep -E '^(OK|AUSENTE)' | tee -a "$REL"
{ diz "--- todos os globais"; sed -n '/--- todos/,$p' "$TMP/probe" | tail -n +2 >> "$REL"; }

esperar_foco_do_alvo(){
  for _ in $(seq 50); do
    grep 'FOCO-ALVO' "$TMP/alvo" 2>/dev/null | tail -1 | grep -q 'ativo=true' && return 0
    sleep 0.1
  done
  return 1
}

caso(){ # sequência de teclas, nome, ambiente do alvo, opções do painel
  local modo=$1 nome="$1, $2" ambiente=$3; shift 3
  env $ambiente $BIN target > "$TMP/alvo" 2>&1 & local A=$!
  # Sem o foco na janela-alvo não há colagem: ela nunca pode cair em outro app.
  if ! esperar_foco_do_alvo; then
    diz "FALHA $nome: a janela-alvo não recebeu o foco"
    falhas=$((falhas+1))
    kill $A 2>/dev/null; wait $A 2>/dev/null
    return
  fi
  : > "$TMP/watch"
  $BIN watch > "$TMP/watch" 2>&1 & local W=$!
  sleep 0.3
  SUPERCOLA_PASTE_MODE=$modo $BIN panel --auto --paste "$@" > "$TMP/painel" 2>&1 & local P=$!
  local copiado colado
  for _ in $(seq 35); do
    copiado=$(grep -oE 'len=[0-9]+ hash=[0-9a-f]+' "$TMP/painel" | head -1)
    colado=$(grep -oE 'len=[0-9]+ hash=[0-9a-f]+' "$TMP/alvo" | tail -1)
    [ -n "$copiado" ] && [ "$copiado" = "$colado" ] && break
    sleep 0.2
  done
  sleep 0.3
  local foco tecla injecao visto
  foco=$(sed -n 's/^FOCO-PAINEL ms=//p' "$TMP/painel" | head -1)
  tecla=$(grep -m1 '^TECLA' "$TMP/alvo" | cut -d' ' -f2)
  injecao=$(grep -cE '^INJETADO' "$TMP/painel")
  visto=não; [ -n "$copiado" ] && grep -q "DADOS $copiado" "$TMP/watch" && visto=sim
  local etapas="foco-painel=${foco:-nenhum}ms injetou=$injecao tecla=${tecla:-nenhuma} data-control-viu=$visto"
  if [ -n "$copiado" ] && [ "$copiado" = "$colado" ]; then
    diz "OK    $nome ($colado) $etapas"
  elif [ -n "$CONHECIDO" ]; then
    diz "LIMITE $nome copiado=[$copiado] colado=[$colado] $etapas"
  else
    diz "FALHA $nome copiado=[$copiado] colado=[$colado] $etapas"
    falhas=$((falhas+1))
    grep -E '^ERRO|^SEM-FOCO' "$TMP/painel" | tee -a "$REL"
  fi
  kill $P $A $W 2>/dev/null; wait $P $A $W 2>/dev/null
  sleep 0.2
}

diz; diz "=== escrever no clipboard e colar (o maior risco da arquitetura)"
# `seat` usa o mapa de teclas atual, e `us` troca o mapa. O supercola usa `seat` no COSMIC.
for modo in seat us; do
  caso $modo "data-control, app nativo" GDK_BACKEND=wayland --dc
  caso $modo "data-control, 300 KB" GDK_BACKEND=wayland --dc --size 300000
  caso $modo "data-control, XWayland" GDK_BACKEND=x11 --dc
  caso $modo "data-control, XWayland, 60 KB" GDK_BACKEND=x11 --dc --size 60000
  caso $modo "data-control, tela cheia (clique fora)" GDK_BACKEND=wayland --dc --full
  caso $modo "GDK, app nativo" GDK_BACKEND=wayland
  caso $modo "GDK, XWayland" GDK_BACKEND=x11
done
diz "falhas: $falhas"

diz; diz "=== limites conhecidos do COSMIC (não contam como falha)"
CONHECIDO=1
caso seat "GDK, 300 KB" GDK_BACKEND=wayland --size 300000
caso seat "data-control, XWayland, 300 KB" GDK_BACKEND=x11 --dc --size 300000
CONHECIDO=

if [ -n "$COMPLETO" ]; then
  diz; diz "=== teclado depois de colar"
  $BIN target > "$TMP/alvo" 2>&1 & A=$!
  esperar_foco_do_alvo
  $BIN panel --auto --paste > "$TMP/painel" 2>&1 & P=$!
  echo ">>> Depois da colagem, digite 'çãé~' na janela alvo."
  sleep 6
  sim "Os acentos saíram certos depois da colagem"
  sim "Dá para digitar normalmente e o layout continua o mesmo"
  sim "O primeiro caractere digitado depois de colar apareceu"
  sim "NumLock e CapsLock continuam como estavam"
  kill $P $A 2>/dev/null; wait $P $A 2>/dev/null

  diz; diz "=== clique fora"
  $BIN panel --full > "$TMP/painel" 2>&1 & P=$!
  echo ">>> Clique em qualquer lugar fora do cartão do painel."
  for _ in $(seq 75); do grep -q CLIQUE-FORA "$TMP/painel" && break; sleep 0.2; done
  if grep -q CLIQUE-FORA "$TMP/painel"; then diz "OK    clique fora"; else diz "FALHA clique fora"; fi
  kill $P 2>/dev/null; wait $P 2>/dev/null

  diz; diz "=== app em foco"
  $BIN focus > "$TMP/foco" 2>&1 & F=$!
  echo ">>> Clique em duas janelas diferentes nos próximos 6 segundos."
  sleep 6
  kill $F 2>/dev/null; wait $F 2>/dev/null
  grep -E 'BACKEND|FOCO' "$TMP/foco" | sort -u | head -12 | tee -a "$REL"
  grep -q '^ERRO' "$TMP/foco" && diz "FALHA foco: $(grep '^ERRO' "$TMP/foco" | head -1)"
fi

diz; diz "=== atalhos e autostart"
C="$HOME/.config/cosmic/com.system76.CosmicSettings.Shortcuts/v1/custom"
if [ -f "$C" ]; then
  diz "custom existe, $(wc -l < "$C") linhas"
  grep -E 'key: *"(v|V)"' "$C" | sed 's/^/ponto de atenção: /' | tee -a "$REL"
else
  diz "custom ainda não existe"
fi
D=/usr/share/cosmic/com.system76.CosmicSettings.Shortcuts/v1/defaults
[ -f "$D" ] && grep -nE '\[Super\], *key: *"(v|V|period|semicolon)"' "$D" | tee -a "$REL"

diz; diz "=== desempenho e desfoque"
grep -E 'background_effect' "$TMP/probe" | head -2 | tee -a "$REL"

echo
echo "Relatório: $REL"
exit "$falhas"
