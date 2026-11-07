#!/usr/bin/env python3
"""Baixa os ativos de licença aberta e gera os arquivos de data/.

Rode só para atualizar: o resultado fica versionado, e o build não acessa a rede.
Fontes: emojibase-data (MIT), Selawik (OFL-1.1), Fluent UI System Icons (MIT).
"""
import io
import json
import pathlib
import tarfile
import urllib.request
import zipfile

RAIZ = pathlib.Path(__file__).resolve().parent.parent / "data"
EMOJIBASE = "https://registry.npmjs.org/emojibase-data/-/emojibase-data-18.0.0.tgz"
SELAWIK = "https://github.com/microsoft/Selawik/releases/download/1.01/Selawik_Release.zip"
FLUENT = "https://raw.githubusercontent.com/microsoft/fluentui-system-icons/main/assets"
MAX_VERSION = 15.1  # o que a fonte de emoji do Ubuntu 24.04 desenha
GRUPO_COMPONENTES = 2

# nome local: (pasta no repositório, base do arquivo, variante)
ICONES = {
    "emoji": ("Emoji", "emoji", "regular"),
    "clipboard": ("Clipboard", "clipboard", "regular"),
    "symbols": ("Math Symbols", "math_symbols", "regular"),
    "gif": ("GIF", "gif", "regular"),
    "pin": ("Pin", "pin", "regular"),
    "pin-filled": ("Pin", "pin", "filled"),
    "more": ("More Horizontal", "more_horizontal", "regular"),
    "delete": ("Delete", "delete", "regular"),
    "recent": ("History", "history", "regular"),
    "people": ("People", "people", "regular"),
    "food": ("Food", "food", "regular"),
    "travel": ("Vehicle Car", "vehicle_car", "regular"),
    "activity": ("Sport", "sport", "regular"),
    "heart": ("Heart", "heart", "regular"),
    "dismiss": ("Dismiss", "dismiss", "regular"),
    "back": ("Arrow Left", "arrow_left", "regular"),
    "chevron": ("Chevron Right", "chevron_right", "regular"),
    "trending": ("Arrow Trending", "arrow_trending", "regular"),
    "paste-text": ("Clipboard Letter", "clipboard_letter", "regular"),
}


def baixar(url):
    pedido = urllib.request.Request(url, headers={"User-Agent": "supercola-assets"})
    with urllib.request.urlopen(pedido, timeout=120) as r:
        return r.read()


def emoji():
    bruto = tarfile.open(fileobj=io.BytesIO(baixar(EMOJIBASE)), mode="r:gz")

    def ler(nome):
        return json.load(bruto.extractfile(f"package/{nome}"))

    pt = {e["hexcode"]: e for e in ler("pt/data.json")}
    en = {e["hexcode"]: e for e in ler("en/data.json")}

    def busca(e):
        if not e:
            return ""
        return " ".join(dict.fromkeys([e.get("label", "")] + e.get("tags", [])))

    saida = []
    for h, e in sorted(en.items(), key=lambda kv: kv[1].get("order", 1 << 30)):
        if e.get("group") in (None, GRUPO_COMPONENTES) or e.get("version", 0) > MAX_VERSION:
            continue
        item = {"e": e["emoji"], "g": e["group"], "pt": busca(pt.get(h)), "en": busca(e)}
        tons = [s["emoji"] for s in e.get("skins", []) if isinstance(s.get("tone"), int)]
        if len(tons) == 5:
            item["s"] = tons
        saida.append(item)
    (RAIZ / "emoji.json").write_text(
        json.dumps(saida, ensure_ascii=False, separators=(",", ":")), encoding="utf-8"
    )
    print("emoji:", len(saida))


def fontes():
    z = zipfile.ZipFile(io.BytesIO(baixar(SELAWIK)))
    destino = RAIZ / "fonts"
    destino.mkdir(exist_ok=True)
    for nome in z.namelist():
        base = nome.rsplit("/", 1)[-1]
        if base.lower() in ("selawk.ttf", "selawksb.ttf", "selawkl.ttf"):
            (destino / base.lower()).write_bytes(z.read(nome))
            print("fonte:", base)


def icones():
    destino = RAIZ / "icons"
    destino.mkdir(exist_ok=True)
    for local, (pasta, base, variante) in ICONES.items():
        url = f"{FLUENT}/{pasta.replace(' ', '%20')}/SVG/ic_fluent_{base}_20_{variante}.svg"
        (destino / f"supercola-{local}-symbolic.svg").write_bytes(baixar(url))
    print("ícones:", len(ICONES))


def licencas():
    destino = RAIZ / "licenses"
    destino.mkdir(exist_ok=True)
    for nome, url in {
        "Selawik-OFL.txt": "https://raw.githubusercontent.com/microsoft/Selawik/master/LICENSE.txt",
        "fluentui-system-icons-MIT.txt": "https://raw.githubusercontent.com/microsoft/fluentui-system-icons/main/LICENSE",
        "emojibase-MIT.txt": "https://raw.githubusercontent.com/milesj/emojibase/master/LICENSE",
    }.items():
        (destino / nome).write_bytes(baixar(url))


if __name__ == "__main__":
    RAIZ.mkdir(exist_ok=True)
    emoji()
    fontes()
    icones()
    licencas()
