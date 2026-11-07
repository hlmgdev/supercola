# Super Cola

Histórico da área de transferência no estilo do Win+V para Linux, no COSMIC, no GNOME e no KDE Plasma.

`Super+V` abre um painel com os últimos itens copiados e abas de emoji, GIF, kaomoji e símbolos, com a aparência do painel do Windows 11. Ao escolher um item, ele é colado no app em que você estava.

> Super Cola é um projeto independente, sem ligação com a Microsoft ou a System76.

## Compatibilidade

O pacote é para `amd64` e funciona só em sessões Wayland.

| Ambiente | Testado em |
|---|---|
| COSMIC | Pop!_OS 24.04 |
| GNOME | 46 (Ubuntu 24.04), 48 (Debian 13) e 50 (Ubuntu 26.04) |
| KDE Plasma | 6.3 (Debian 13) |
| sway | Pop!_OS 24.04, sem registrar atalhos |

A extensão do GNOME incluída no pacote aceita as versões de 46 a 51, mas a 47, a 49 e a 51 não foram testadas.

Não funciona em sessões X11, no Cinnamon, no Debian 12, no Ubuntu 22.04 nem no Kubuntu 24.04, que usa o Plasma 5.

## Instalação

Baixe da [última versão](https://github.com/hlmgdev/supercola/releases/latest) o `supercola_1.2.0_amd64.deb`, o `SHA256SUMS` e o `SHA256SUMS.asc`. Confira o download como descrito em [Conferir o download](#conferir-o-download) e instale:

```sh
sudo apt install ./supercola_1.2.0_amd64.deb
```

Encerre a sessão e entre de novo, ou rode `supercola &`. O que acontece na primeira execução depende do ambiente:

- **COSMIC:** o Super Cola registra `Super+V` e `Super+;` e guarda uma cópia do arquivo de atalhos em `~/.config/cosmic/com.system76.CosmicSettings.Shortcuts/v1/custom.supercola-backup`. Se uma das teclas já estiver em uso, ela não é alterada.
- **GNOME:** o Super Cola ativa a extensão dele e registra `Super+;`. Se você rodar `supercola &` sem sair da sessão, ele avisa que é preciso sair e entrar de novo para a extensão carregar.
- **KDE:** o Super Cola registra `Super+;`.

Por padrão, `Super+V` abre as notificações no GNOME e o Klipper no KDE. Nesses dois ambientes, o Super Cola pergunta uma vez se pode ficar com a tecla. Se você aceitar, `supercola --restore-shortcuts` desfaz a troca. Se recusar, nada muda, e ele abre com `Super+;`.

Para conferir se está tudo certo:

```sh
supercola --doctor
```

## Atualizações

O pacote configura o repositório `https://hlmgdev.github.io/supercola-apt`, então as versões novas chegam junto com as atualizações do sistema, pela loja ou por `sudo apt update && sudo apt upgrade`. Quem tem a 1.0.0 ou a 1.1.0 precisa instalar a 1.2.0 à mão uma vez.

O pacote instala três arquivos em `/etc/apt`:

| Arquivo | Para quê |
|---|---|
| `sources.list.d/supercola.sources` | Endereço do repositório |
| `keyrings/supercola-archive-keyring.gpg` | Chave pública que confere a assinatura do repositório. Ela não vale para nenhum outro |
| `preferences.d/supercola.pref` | Deste repositório, o apt só instala o pacote `supercola` |

O pacote não roda nenhum script durante a instalação: ele só copia arquivos.

Para deixar de receber atualizações, apague `/etc/apt/sources.list.d/supercola.sources`. As atualizações do pacote não recriam o arquivo.

### Conferir o download

Cada versão traz um `SHA256SUMS` assinado com a chave do repositório. A impressão digital dessa chave é:

```text
8EC40968C091F69686334ECFF36E6858EC0E92C2
```

Na pasta do download, sem importar nada para o seu GnuPG:

```sh
curl -fsSLO https://hlmgdev.github.io/supercola-apt/supercola-archive-keyring.asc
gpg --show-keys supercola-archive-keyring.asc
gpg --dearmor < supercola-archive-keyring.asc > supercola-archive-keyring.gpg
gpgv --keyring ./supercola-archive-keyring.gpg SHA256SUMS.asc SHA256SUMS
sha256sum -c SHA256SUMS
```

A impressão digital mostrada pelo `gpg --show-keys` precisa ser igual à de cima. O `gpgv` precisa dizer "Assinatura correta", ou "Good signature" em inglês, e o `sha256sum`, "SUCESSO" ou "OK". Se algo não bater, não instale.

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
- No GNOME, o painel pode aparecer no Alt+Tab enquanto está aberto.
- No GNOME, depois de colar um emoji, o clipboard anterior volta só no formato principal. Uma imagem copiada com texto, por exemplo, volta com um dos dois.
- No GNOME, cada versão nova pode desativar a extensão até sair uma atualização do Super Cola.
- Em apps XWayland, a colagem é cortada em 64 KiB. O limite vem do compositor.
- No terminal integrado do VS Code, a colagem automática aparece como `^V`, porque ali `Ctrl+V` não cola, nem digitado à mão. O Super Cola só sabe que o app é o VS Code, não qual parte dele está ativa. O item já fica no clipboard, então basta apagar o `^V` e usar `Ctrl+Shift+V`.
- Não há sincronização entre dispositivos.
- O painel fecha a cada emoji escolhido, porque precisa devolver o foco para colar.

## Problemas comuns

| Problema | O que tentar |
|---|---|
| `daemon: parado` no `--doctor` | Aperte `Super+V`, que inicia o daemon, ou rode `supercola &` |
| `Super+V` não faz nada | Rode `supercola --install-shortcut`. Se a tecla estiver em uso, a mensagem diz com o quê |
| Atalho registrado, mas sem efeito | Teste no terminal com `supercola-toggle`. Se abrir, o problema é o atalho do ambiente |
| `extensão do GNOME` ausente no `--doctor` | Saia e entre de novo na sessão. Se o `--doctor` disser que as extensões estão desligadas, ligue-as no app Extensões: o GNOME as desliga depois de uma falha |
| O histórico fica vazio no GNOME | Rode `supercola --doctor`: sem a extensão, nada é capturado |
| O painel abre, mas não cola | Nas configurações, troque "Gravar no clipboard via" para cópia comum |
| Cola o item errado ou nada | Aumente "Espera antes de colar" para 300 ms ou mais |
| Terminal cola `^V` | Rode `supercola-toggle status`, veja o valor de `focus=` com o terminal em foco e acrescente esse nome à lista de terminais |

## Remoção

Se você cedeu `Super+V` ao Super Cola no GNOME ou no KDE, devolva a tecla antes de remover:

```sh
supercola --restore-shortcuts
```

Depois, remova o pacote:

```sh
sudo apt purge supercola
```

O `purge` também apaga os três arquivos do repositório de atualizações. Com `apt remove`, eles ficam, e o repositório continua configurado.

Os atalhos ficam nas suas configurações e não são removidos com o pacote:

- **COSMIC:** apague as linhas com `supercola-toggle` no arquivo `custom`.
- **GNOME:** apague "Super Cola" em Configurações > Teclado > Atalhos personalizados.
- **KDE:** nada a fazer, porque os atalhos saem com o pacote.

Seus dados ficam em `~/.local/share/supercola`, `~/.config/supercola` e `~/.cache/supercola`.

## Desenvolvimento

O projeto compila no Pop!_OS 24.04 ou no Ubuntu 24.04, que definem as versões mínimas do pacote.

Instale as dependências e o Rust:

```sh
sudo apt install build-essential pkg-config git curl meson ninja-build libwayland-bin \
  libwayland-dev wayland-protocols libgtk-4-dev libxkbcommon-dev dpkg-dev \
  sway wl-clipboard grim wtype xwayland
curl -fsSL https://sh.rustup.rs | sh -s -- -y --profile minimal -c clippy -c rustfmt
cargo install cargo-audit --locked
```

Essa versão do Ubuntu não tem o gtk4-layer-shell, então ele é compilado da fonte e embutido no pacote. O hash abaixo é o mesmo do código que o Debian empacota:

```sh
curl -fsSLO https://github.com/wmww/gtk4-layer-shell/archive/refs/tags/v1.3.0.tar.gz
echo '1ebb01ab14e98afd1727f68f64981c37bd23305b1f131f5667c02b94cf593192  v1.3.0.tar.gz' | sha256sum -c
tar -xzf v1.3.0.tar.gz
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

Para testar em sessões reais de GNOME e KDE, sem mexer na sua, use as VMs de `tests/vm`. Elas precisam de `/dev/kvm` e rodam sem `sudo`:

```sh
bash tests/vm/vm.sh criar debian13-kde
bash tests/vm/kde.sh
bash tests/vm/vm.sh criar ubuntu2404-gnome
bash tests/vm/gnome.sh ubuntu2404-gnome
```

Os roteiros instalam o `.deb` de `dist/` do zero, usam teclas e cliques reais e tratam uma queda do GNOME Shell como falha. `bash tests/vm/vm.sh tela NOME` abre a tela de uma VM para testar à mão. Rode um roteiro por vez: com várias VMs disputando a CPU, um app lento pode perder teclas.

O repositório de atualizações tem um roteiro próprio, que roda em qualquer uma das VMs:

```sh
bash tests/vm/apt.sh ubuntu2404-gnome
bash tests/vm/apt.sh debian13-kde
bash tests/vm/apt.sh ubuntu2604-gnome
```

Ele monta um repositório com uma chave descartável, servido por este computador, e confere na VM a assinatura, a atualização, a recusa de índices adulterados ou assinados por outra chave, o bloqueio de pacotes alheios e a remoção. O Debian 13 e o Ubuntu 26.04 têm o apt 3, que confere assinaturas de outro jeito, por isso os três.

A automação não consegue clicar no COSMIC. Antes de uma versão, confira à mão:

- Colar um item antigo em um app nativo, em um app XWayland e em um terminal.
- Digitar logo depois de colar: a primeira letra precisa aparecer.
- Com o painel aberto, clicar em outra janela e na área de trabalho vazia.
- Arrastar o painel pela alça.

### Publicar uma versão

1. Atualize a versão no `Cargo.toml`, rode `cargo update --workspace` e escreva a entrada do `CHANGELOG.md`.
2. Gere o pacote e rode os testes acima. No COSMIC, rode `tests/integration/cosmic.sh` e `spike/run.sh` também na versão anterior, a partir da tag, e compare antes de fazer a conferência manual.
3. Assine e publique no repositório apt, a partir de um clone do [supercola-apt](https://github.com/hlmgdev/supercola-apt):

   ```sh
   bash packaging/publish-apt.sh dist/supercola_<versão>_amd64.deb ../supercola-apt
   ```

   O script recusa um pacote com scripts de instalação, com setuid, com outra chave ou com uma versão já publicada com outro conteúdo, e para se o `cargo audit` achar uma falha conhecida. Ele pede a senha da chave e não faz commit nem push.
4. Faça commit e push no `supercola-apt` e espere o GitHub Pages publicar.
5. Crie a Release com o `.deb`, o `SHA256SUMS` e o `SHA256SUMS.asc` de `dist/`.

A chave privada fica só com o mantenedor e vence em 6 de novembro de 2031. Antes disso, estenda a validade com `gpg --quick-set-expire`, exporte a chave pública para `packaging/apt/supercola-archive-keyring.asc` e publique uma versão nova, que leva a chave atualizada aos usuários.

As medidas do painel e as diferenças em relação ao Windows estão em [docs/visual-spec.md](docs/visual-spec.md).

## Licenças

Código sob licença MIT. Ativos de terceiros em `data/`, com as licenças em `data/licenses/`: fonte Selawik (OFL-1.1), Fluent UI System Icons (MIT) e dados do emojibase (MIT).
