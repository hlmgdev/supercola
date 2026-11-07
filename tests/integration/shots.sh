#!/bin/bash
# Abre cada aba do painel no sway headless e salva capturas em dist/shots.
set -u
. "$(dirname "$0")/sway-env.sh"
export HOME=$XDG_RUNTIME_DIR/home SUPERCOLA_DATA_DIR=$PWD/data
export XDG_CONFIG_HOME=$HOME/.config XDG_DATA_HOME=$HOME/.local/share
mkdir -p "$HOME/.fonts" "$XDG_CONFIG_HOME/supercola" dist/shots
cp data/fonts/*.ttf "$HOME/.fonts/"; fc-cache -f >/dev/null 2>&1
TEMA=${1:-light}
printf 'theme = "%s"
language = "pt"
' "$TEMA" > "$XDG_CONFIG_HOME/supercola/config.toml"
export PATH=$PWD/target/release:$PATH

target/release/supercola > "$XDG_RUNTIME_DIR/supercola.log" 2>&1 &
for _ in $(seq 50); do [ -S "$XDG_RUNTIME_DIR/supercola.sock" ] && break; sleep 0.1; done

printf 'Reunião às 15h na sala 3' | wl-copy; sleep 0.4
printf 'https://github.com/pop-os/cosmic-epoch' | wl-copy; sleep 0.4
printf 'Um texto bem longo para testar a quebra de linha e a elipse do cartão: lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.' | wl-copy; sleep 0.4
grim -g "0,0 300x200" "$XDG_RUNTIME_DIR/x.png" && wl-copy --type image/png < "$XDG_RUNTIME_DIR/x.png"; sleep 0.5
supercola-toggle status

for aba in history home emoji kaomoji symbols gif; do
  supercola-toggle "$aba"
  sleep 3
  grim -g "1400,500 520x580" "dist/shots/$TEMA-$aba.png"
  supercola-toggle status
  timeout 3 wtype -k Escape
  sleep 0.6
done
cat "$XDG_RUNTIME_DIR/supercola.log"
supercola-toggle quit
kill $(jobs -p) 2>/dev/null
exit 0
