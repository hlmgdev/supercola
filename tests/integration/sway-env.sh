# Sobe um sway headless. Use com `. tests/integration/sway-env.sh`.
export XDG_RUNTIME_DIR=/tmp/xdg-$$ WLR_BACKENDS=headless WLR_LIBINPUT_NO_DEVICES=1 \
       WLR_RENDERER=pixman GSK_RENDERER=cairo GDK_BACKEND=wayland NO_AT_BRIDGE=1 \
       GTK_A11Y=none LANG=C.UTF-8
# Sem D-Bus, como no container: o app_id não muda, e o teste nunca fala com um supercola real.
export DBUS_SESSION_BUS_ADDRESS=disabled:
mkdir -p "$XDG_RUNTIME_DIR"; chmod 700 "$XDG_RUNTIME_DIR"
printf 'output HEADLESS-1 resolution 1920x1080\nexec_always true\n' > "$XDG_RUNTIME_DIR/sway.cfg"
sway -c "$XDG_RUNTIME_DIR/sway.cfg" >"$XDG_RUNTIME_DIR/sway.log" 2>&1 &
SWAY_PID=$!
for _ in $(seq 100); do
  WAYLAND_DISPLAY=$(ls "$XDG_RUNTIME_DIR" 2>/dev/null | grep -E '^wayland-[0-9]+$' | head -1)
  [ -n "$WAYLAND_DISPLAY" ] && break
  sleep 0.1
done
[ -n "$WAYLAND_DISPLAY" ] || { echo "sway não subiu"; cat "$XDG_RUNTIME_DIR/sway.log"; exit 1; }
export WAYLAND_DISPLAY
trap 'kill $SWAY_PID 2>/dev/null' EXIT
