# Locale audit — Português brasileiro (`pt-BR`)

**Status:** partial. Pre-D.1 strings translated previously; D.1.6, D.1.7, D.3 keys carry English placeholder.
**AI cross-check confidence:** medium-high. Claude has decent BR-Portuguese coverage for technical UI; spelling and register are mostly safe but native review should verify casual vs formal tone consistency.

## Glossary

| English | Português brasileiro (recommended) | Notes |
|---|---|---|
| overlay | **sobreposição** / **overlay** | Loan accepted. |
| scene | **cena** | |
| entity | **entidade** | |
| edit mode | **modo de edição** | |
| chord (key combo) | **combinação de teclas** | |
| library | **biblioteca** | |
| monitor pin | **fixar no monitor** | |
| preset | **predefinição** / **preset** | |

## Placeholder English — proposed translations

### D.1.6 — Keybindings tab UI

```ftl
settings-tab-keybindings = Atalhos
keybindings-unbound = (não atribuído)
keybindings-add = Adicionar
keybindings-recording = Pressione uma combinação… (Esc para cancelar)
keybindings-conflict = Conflito com { $action }
keybindings-reset-all = Restaurar tudo para os padrões
keybindings-help = Atalhos personalizados ficam salvos em config.toml
```

### D.1.7 — Action labels

| key | suggested BR-Portuguese |
|---|---|
| `action-toggle-edit-mode` | Alternar modo de edição |
| `action-hide-overlay` | Ocultar / exibir sobreposição |
| `action-pause-all` | Pausar todas as animações |
| `action-quit-with-save` | Sair (salvar configuração) |
| `action-save-now` | Salvar configuração agora |
| `action-open-command-palette` | Paleta de comandos |
| `action-cycle-entity` | Próxima entidade |
| `action-delete-selected` | Excluir entidade selecionada |
| `action-nudge-up` | Mover seleção para cima |
| `action-nudge-down` | Mover seleção para baixo |
| `action-nudge-left` | Mover seleção para a esquerda |
| `action-nudge-right` | Mover seleção para a direita |
| `action-center-on-screen` | Centralizar seleção na tela |
| `action-toggle-visible` | Alternar visibilidade |
| `action-toggle-gravity` | Alternar gravidade |
| `action-toggle-playback` | Alternar reprodução |
| `action-duplicate-selected` | Duplicar seleção |
| `action-reset-transform` | Restaurar escala / opacidade |
| `action-bring-forward` | Trazer seleção para frente |
| `action-send-backward` | Enviar seleção para trás |
| `action-fps-up` | Aumentar FPS |
| `action-fps-down` | Diminuir FPS |
| `action-opacity-up` | Aumentar opacidade |
| `action-opacity-down` | Diminuir opacidade |
| `action-cycle-monitor` | Trocar monitor da entidade |
| `action-show-entity-info` | Mostrar detalhes da entidade |
| `action-show-help` | Mostrar ajuda do teclado |

### D.3 — Accessibility section

```ftl
appearance-accessibility-header = Acessibilidade
appearance-accesskit-label = Gerar atualizações da árvore AccessKit
appearance-accesskit-hint = Alimenta leitores de tela AT-SPI (Orca etc.). Deixe ativado; desative só para reduzir o consumo ou se seu desktop não expõe um barramento AT-SPI.
```

## Suspected issues for native reviewer

### Tu vs você

BR-Portuguese UI conventions use *você* (treated as third-person grammatically) for direct address. The proposed strings use imperatives that are você-compatible (`Pressione`, `Deixe ativado`). Confirm consistency with the pre-D.1 file — if it slipped into *tu* anywhere, fix to você.

### *Sobreposição* vs *overlay*

If the existing file mixes both, pick one. *Sobreposição* is more native; *overlay* is more recognised by dev/designer audiences who follow English-language tutorials.

### *Sair* vs *encerrar*

For "quit," BR-Portuguese conventionally uses *Sair*. Some software uses *Fechar* (close). The proposed *Sair* matches the most common pattern.

## Open questions for native reviewer

- *Atalhos* vs *atalhos do teclado* for the tab title?
- Loanword policy: *FPS*, *overlay*, *preset* — accept these or translate everywhere?
- Spot-check formal vs informal register across the whole file — Brazilian Portuguese UI often shifts depending on product positioning (consumer-friendly vs professional tool). animaEngine reads more like a power-user tool; lean slightly formal but conversational.
