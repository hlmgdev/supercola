# Especificação visual

A referência é o painel do Windows 11.

**Origem das medidas:** as seções do cartão e da área de transferência foram medidas em uma captura real do painel, com 360 px de largura e texto compatível com escala de 100%. As medidas das outras abas vêm de capturas públicas de 2022, em escala estimada de 125%. Uma captura feita no seu Windows pode refiná-las.

## Cartão

| Item | Valor |
|---|---|
| Tamanho | 360 × 400 px |
| Cantos | 8 px |
| Margem até a borda da tela | 12 px |
| Sombra | `0 8px 32px` a 28% e `0 2px 8px` a 18% |
| Fonte | Selawik, com métricas da Segoe UI |
| Texto | 14 px; rótulos secundários 12 px |
| Fundo | Translúcido com desfoque do compositor. Sem o efeito, sólido a 96% |

## Cores

| Token | Claro | Escuro |
|---|---|---|
| Painel com desfoque | `rgba(243,243,243,.85)` | `rgba(44,44,44,.8)` |
| Painel sem desfoque | `rgba(243,243,243,.96)` | `rgba(44,44,44,.96)` |
| Cartão | `rgba(255,255,255,.7)` | `rgba(255,255,255,.0605)` |
| Borda do cartão | `rgba(0,0,0,.0578)` | `rgba(255,255,255,.0698)` |
| Texto principal | `rgba(0,0,0,.894)` | `#fff` |
| Texto secundário | `rgba(0,0,0,.62)` | `rgba(255,255,255,.786)` |
| Destaque padrão | `#0067C0` | `#60CDFF` |

A cor de destaque vem do portal `Settings`, quando o sistema informa uma. No COSMIC, ela segue o tema, não o azul do Windows.

## Estrutura, de cima para baixo

Posições em relação ao canto superior esquerdo do cartão.

| Elemento | Valor |
|---|---|
| Alça | 32 × 3 px, centralizada, em y≈15 |
| Fechar | Centro em (338, 15) |
| Abas | 6, alinhadas à esquerda desde x=15, com 41 px cada e ícone desenhado de 16 px, em y≈58 |
| Indicador da aba | 16 × 3 px, centralizado, em y≈75 |
| Busca | 328 × 30 px, de y=98 a 128, cantos de 4 px |

A ordem das abas é Início, Emoji, GIF, Kaomoji, Símbolos e Área de transferência. Em Emoji, GIF, Kaomoji e Símbolos, a linha das abas dá lugar à seta de voltar e às categorias.

## Área de transferência

| Elemento | Valor |
|---|---|
| Título | Em x=16, y≈112, 14 px |
| Limpar tudo | Altura de 24 px, com borda, a 16 px da direita |
| Lista | Começa em y≈135, com margens de 16 px |
| Item | 328 × 80 px, cantos de 4 px, 8 px entre itens |
| Conteúdo | Até 3 linhas, ou miniatura à esquerda com 64 px de altura |
| Ações | `⋯` a 20 px do topo e alfinete a 20 px da base, a 21 px da direita |
| Item selecionado | Borda de 2 px na cor de destaque |
| Ações do `⋯` | Ícones ao lado do item: colar como texto e excluir |

## Outras abas

| Aba | Valor |
|---|---|
| Início | Seções de recentes, cada uma com botão `>` |
| Emoji | 6 colunas, célula de ~55 px, emoji de 32 px |
| Kaomoji | 3 colunas de botões com ~31 px de altura |
| Símbolos | 6 colunas de botões com ~47 px de altura |
| GIF | 3 colunas de ~104 × 90 px |

## Diferenças conscientes

- **Busca na área de transferência:** o Windows não tem. No Super Cola, o campo fica oculto e aparece ao digitar, ocupando o lugar do cabeçalho. Kaomoji e Símbolos seguem a mesma regra.
- **Emoji:** usa a fonte colorida do sistema (Noto Color Emoji), não a do Windows. É a mesma que os apps do Linux usam para mostrar o que foi colado.
- **Categorias de emoji:** a barra filtra por categoria, sem rolagem contínua entre seções.
- **GIF:** células de tamanho uniforme, sem mosaico, porque os provedores não informam as dimensões usadas aqui. O início mostra o GIF só como atalho, para abrir o painel não acessar a rede.
- **Ícones:** recoloridos por filtro CSS, porque o GTK do Ubuntu 24.04 não recoloriu os SVGs da Fluent nos testes.
