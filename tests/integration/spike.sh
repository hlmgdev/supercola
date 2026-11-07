#!/bin/bash
# Exercita o kit de validação num sway headless. Não prova o COSMIC.
set -u
. "$(dirname "$0")/sway-env.sh"
BIN=${BIN:-target/release/supercola-spike}
L=$XDG_RUNTIME_DIR/logs; mkdir -p "$L"
falhas=0
ok(){ echo "OK    $1"; }
ko(){ echo "FALHA $1"; falhas=$((falhas+1)); }

$BIN probe > "$L/probe" 2>&1; grep -E '^(OK|AUSENTE)' "$L/probe"
$BIN watch > "$L/watch" 2>&1 & WATCH=$!
$BIN focus > "$L/focus" 2>&1 & FOCUS=$!
sleep 0.5

caso() { # nome, opções do painel
  local nome=$1; shift
  $BIN target > "$L/alvo-$nome" 2>&1 & local ALVO=$!
  sleep 1.5
  $BIN panel --paste "$@" > "$L/painel-$nome" 2>&1 & local P=$!
  sleep 4
  wtype -k Return
  sleep 3
  local esperado obtido
  esperado=$(grep -oE 'len=[0-9]+ hash=[0-9a-f]+' "$L/painel-$nome" | tail -1)
  obtido=$(grep -oE 'len=[0-9]+ hash=[0-9a-f]+' "$L/alvo-$nome" | tail -1)
  if [ -n "$esperado" ] && [ "$esperado" = "$obtido" ]; then ok "$nome ($obtido)"
  else ko "$nome: copiado='$esperado' colado='$obtido'"; cat "$L/painel-$nome"; fi
  kill $P $ALVO 2>/dev/null; wait $P $ALVO 2>/dev/null
}
caso gdk-curto
caso dc-curto --dc
caso gdk-300k --size 300000
caso dc-300k --dc --size 300000
caso full --dc --full

sleep 0.5
kill $WATCH $FOCUS 2>/dev/null
echo "--- watch"; grep -cE '^DADOS' "$L/watch"; head -3 "$L/watch"
echo "--- focus"; head -6 "$L/focus"
grep -qE 'FOCO app_id=supercola-spike' "$L/focus" && ok focus || ko focus
echo "falhas=$falhas"; exit $falhas
