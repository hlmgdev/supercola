# Super Cola

Histórico da área de transferência no estilo do Win+V para o Pop!_OS 24.04 com COSMIC.

`Super+V` abre um painel com os últimos itens copiados e abas de emoji, GIF, kaomoji e símbolos, com a aparência do painel do Windows 11. Ao escolher um item, ele é colado no app em que você estava.

> Super Cola é um projeto independente, sem ligação com a Microsoft ou a System76.

## Instalação

```sh
sudo apt install ./supercola_1.0.0_amd64.deb
```

Encerre a sessão e entre de novo, ou rode `supercola &`. Na primeira execução no COSMIC, o Super Cola registra `Super+V` e `Super+;` e guarda uma cópia do arquivo de atalhos em `~/.config/cosmic/com.system76.CosmicSettings.Shortcuts/v1/custom.supercola-backup`. Se uma das teclas já estiver em uso, ela não é alterada.

Para conferir se está tudo certo:

```sh
supercola --doctor
```

## Uso

| Ação | Como |
|---|---|
| Abrir ou fechar | `Super+V` |
| Abrir no início, com emojis e recentes | `Super+;` |
| Navegar | setas |
| Colar o selecionado | `Enter` ou clique |
| Pesquisar | comece a digitar |
| Excluir o item | `Delete` ou botão `⋯` |
| Fechar | `Esc`, `X` ou clique fora |
| Mover o painel | arraste pelo tracinho do topo |
| Fixar | ícone de alfinete |
| Colar como texto | botão `⋯` |

Terminais conhecidos recebem `Ctrl+Shift+V` em vez de `Ctrl+V`. A lista pode ser editada nas configurações.

Fora de Início, Emoji e GIF, o campo de busca fica oculto e aparece quando você começa a digitar.

## Regras do histórico

- Guarda 25 itens, de texto, HTML, imagem ou lista de arquivos, com até 4 MB cada.
- Itens comuns ficam só na memória e somem ao encerrar a sessão.
- Itens fixados ficam em `~/.local/share/supercola/pinned.db`, com acesso só para o seu usuário.
- Conteúdo marcado como senha nunca é lido nem guardado. Gerenciadores de senha conhecidos também são ignorados, e a lista pode ser editada.
- Copiar de novo um item sobe esse item para o topo.
- Emoji, kaomoji, símbolos e GIF não entram no histórico, e o clipboard anterior volta depois de colar.

## Configurações

Abra "Super Cola" no menu de aplicativos ou rode `supercola-toggle settings`. Tudo fica em `~/.config/supercola/config.toml`.

## GIF

A aba de GIF precisa de uma chave de API sua, da [KLIPY](https://klipy.com) ou da [GIPHY](https://developers.giphy.com), informada nas configurações. Só essa aba usa a internet, e o que você pesquisa nela é enviado ao serviço escolhido. A chave fica em `config.toml`, com acesso só para o seu usuário.

## Limitações

- O painel abre no canto ou perto do mouse, não junto ao cursor de texto, porque o Wayland não expõe essa posição a outros apps.
- O clique fora só chega ao app de trás no COSMIC, com o painel no canto. Em "Perto do mouse" e em outros compositores, ele apenas fecha o painel.
- Em apps XWayland, a colagem é cortada em 64 KiB. O limite vem do compositor.
- No terminal integrado do VS Code, a colagem automática aparece como `^V`, porque ali `Ctrl+V` não cola, nem digitado à mão. O Super Cola só sabe que o app é o VS Code, não qual parte dele está ativa. O item já fica no clipboard, então basta apagar o `^V` e usar `Ctrl+Shift+V`.
- Não há sincronização entre dispositivos.
- O painel fecha a cada emoji escolhido, porque precisa devolver o foco para colar.

## Problemas comuns

| Problema | O que tentar |
|---|---|
| `daemon: parado` no `--doctor` | Aperte `Super+V`, que inicia o daemon, ou rode `supercola &` |
| `Super+V` não faz nada | Rode `supercola --install-shortcut`. Se a tecla estiver em uso, a mensagem diz com o quê |
| Atalho registrado, mas sem efeito | Teste no terminal com `supercola-toggle`. Se abrir, o problema é o atalho do COSMIC |
| O painel abre, mas não cola | Nas configurações, troque "Gravar no clipboard via" para cópia comum |
| Cola o item errado ou nada | Aumente "Espera antes de colar" para 300 ms ou mais |
| Terminal cola `^V` | Rode `supercola-toggle status`, veja o valor de `focus=` com o terminal em foco e acrescente esse nome à lista de terminais |

## Remoção

```sh
sudo apt purge supercola
```

Os atalhos ficam no arquivo `custom`, que é seu, e não são removidos com o pacote. Apague as linhas com `supercola-toggle` nele. Seus dados ficam em `~/.local/share/supercola`, `~/.config/supercola` e `~/.cache/supercola`.

## Desenvolvimento

O projeto compila no Pop!_OS 24.04 ou no Ubuntu 24.04, que definem as versões mínimas do pacote.

Instale as dependências e o Rust:

```sh
sudo apt install build-essential pkg-config git curl meson ninja-build libwayland-bin \
  libwayland-dev wayland-protocols libgtk-4-dev libxkbcommon-dev dpkg-dev \
  sway wl-clipboard grim wtype xwayland
curl -fsSL https://sh.rustup.rs | sh -s -- -y --profile minimal -c clippy -c rustfmt
```

Essa versão do Ubuntu não tem o gtk4-layer-shell, então ele é compilado da fonte e embutido no pacote:

```sh
curl -fsSL https://github.com/wmww/gtk4-layer-shell/archive/refs/tags/v1.3.0.tar.gz | tar -xz
cd gtk4-layer-shell-1.3.0
meson setup build --prefix=/usr/local --libdir=lib --buildtype=release \
  -Dexamples=false -Ddocs=false -Dtests=false -Dintrospection=false -Dvapi=false
ninja -C build
sudo ninja -C build install
sudo ldconfig
```

Para compilar, testar e gerar o pacote:

```sh
cargo test --workspace --exclude supercola-spike
cargo build --release --workspace
bash tests/integration/e2e.sh
bash packaging/build-deb.sh
```

`tests/integration/e2e.sh` usa um sway sem tela, que tem os mesmos protocolos genéricos, mas não prova o COSMIC. Para isso, rode em uma sessão COSMIC, depois de compilar:

```sh
bash tests/integration/cosmic.sh
bash spike/run.sh
```

Os dois abrem janelas, roubam o foco e substituem o clipboard, então não use o teclado nem o mouse enquanto rodam.

A automação não consegue clicar no COSMIC. Antes de uma versão, confira à mão:

- Colar um item antigo em um app nativo, em um app XWayland e em um terminal.
- Digitar logo depois de colar: a primeira letra precisa aparecer.
- Com o painel aberto, clicar em outra janela e na área de trabalho vazia.
- Arrastar o painel pela alça.

As medidas do painel e as diferenças em relação ao Windows estão em [docs/visual-spec.md](docs/visual-spec.md).

## Licenças

Código sob licença MIT. Ativos de terceiros em `data/`, com as licenças em `data/licenses/`: fonte Selawik (OFL-1.1), Fluent UI System Icons (MIT) e dados do emojibase (MIT).
