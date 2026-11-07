# Changelog

## [1.2.0] - 2026-11-07

### Adicionado

- Atualizações pelo apt, a partir do repositório assinado `https://hlmgdev.github.io/supercola-apt`, configurado pelo próprio pacote
- `SHA256SUMS` assinado em cada versão, para conferir o download

### Segurança

- Deste repositório, o apt só instala o pacote `supercola`, mesmo que ele traga outros
- O pacote é compilado exatamente com as dependências do `Cargo.lock` e só é publicado se o `cargo audit` não achar falhas conhecidas

## [1.1.0] - 2026-11-07

### Adicionado

- Suporte ao KDE Plasma 6 em Wayland
- Suporte ao GNOME em Wayland, com uma extensão incluída no pacote
- Pergunta única antes de usar `Super+V` quando a tecla já tem outro dono, e `supercola --restore-shortcuts` para desfazer a troca

### Corrigido

- Com a cópia comum, apps X11 recebiam o conteúdo vazio

## [1.0.0] - 2026-11-07

Primeira versão.

### Adicionado

- Painel de histórico da área de transferência no estilo do Windows 11, aberto com `Super+V`
- Itens fixados e abas de emoji, kaomoji, símbolos e GIF
- Colagem automática no app de origem
- Pacote `.deb` para Pop!_OS 24.04 e Ubuntu 24.04

[1.2.0]: https://github.com/hlmgdev/supercola/releases/tag/v1.2.0
[1.1.0]: https://github.com/hlmgdev/supercola/releases/tag/v1.1.0
[1.0.0]: https://github.com/hlmgdev/supercola/releases/tag/v1.0.0
