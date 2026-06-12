# Português (Brasil) — tradução base. Revisão de falante nativo pendente.

app-name = animaEngine

settings-tab-inspector = Inspetor
settings-tab-scene = Cena
settings-tab-appearance = Aparência
entity-count-zero = Nenhuma entidade
entity-count-singular = { $n } entidade
entity-count-plural = { $n } entidades

inspector-section-position = Posição
inspector-section-appearance = Aparência
inspector-section-animation = Animação
animation-easing-label = Easing
easing-linear = Linear
easing-ease-in-quad = Ease in
easing-ease-out-quad = Ease out
easing-ease-in-out-quad = Ease in / out
easing-sine = Seno
easing-bounce-out = Bounce out
inspector-section-behavior = Comportamento
inspector-visible = Visível
inspector-gravity = Gravidade
inspector-scale = Escala
inspector-opacity = Opacidade
inspector-fps = FPS
inspector-playing = Reproduzindo
inspector-x = X
inspector-y = Y
inspector-z-index = z-index
inspector-nothing-selected-headline = Nada selecionado
inspector-nothing-selected-hint = Clique numa entidade na aba Cena, ou pressione Tab para alternar entre elas.

behavior-idle = Parado
behavior-walk = Andar
behavior-follow = Seguir o cursor
behavior-wander = Vagar limitado
behavior-bounce = Pulo
behavior-bounce-axis = Eixo
behavior-bounce-horizontal = Horizontal
behavior-bounce-vertical = Vertical
behavior-bounce-both = Ambos (círculo)

scene-empty-headline = Cena vazia
scene-empty-hint = Arraste um PNG / GIF / WebP / MP4 para o overlay — ou experimente um preset abaixo.
scene-drop-hint = Arraste um PNG / GIF / WebP para o overlay para adicionar uma entidade.
scene-presets-header = Presets
scene-preset-append = Adicionar
scene-preset-replace = Substituir
scene-preset-replace-tooltip = Limpa a cena atual antes de adicionar

monitor-section-header = Monitores
monitor-mode-label = Distribuição
monitor-mode-per-monitor = Por monitor
monitor-mode-span = Estender por todos os monitores
monitor-mode-single = Monitor único
scene-window-awareness = Land on windows (X11)
scene-window-awareness-tooltip = Physics-enabled characters land on and walk along the top edges of your open windows. X11 sessions only — Wayland offers no window positions, so this does nothing there.
monitor-pin-label = Fixar ao monitor
monitor-pin-auto = Auto (segue a posição)
monitor-pinned-toast = Entidade fixada em { $name }
monitor-pin-cleared-toast = Entidade agora segue sua posição
monitor-no-monitors-detected = Nenhum monitor detectado

appearance-theme-header = Tema
appearance-theme-label = Tema
appearance-language-header = Idioma
theme-dark = Escuro
theme-light = Claro
theme-dark-hc = Escuro · Alto contraste
theme-light-hc = Claro · Alto contraste

onboarding-tabs = As configurações estão divididas em três abas — Inspetor, Cena, Aparência.
onboarding-quick-toggles = Dica: V alterna visibilidade, G alterna gravidade — sem abrir este painel.
onboarding-theme = Temas são aplicados instantaneamente — sem reiniciar.
onboarding-coach-step1 = Welcome! Your characters live on the desktop. Click the gear button in the top-right corner to enter edit mode.
onboarding-coach-step2 = Drop a PNG, GIF, WebP or MP4 anywhere on the screen to add it as a character. The side panel edits everything you select.
onboarding-coach-step3 = Ctrl+K opens the command palette. Ctrl+Shift+A toggles edit mode from anywhere, Ctrl+Shift+H hides the overlay.
onboarding-coach-next = Next
onboarding-coach-skip = Skip tour
onboarding-coach-done = Got it
onboarding-dismiss = Fechar

menu-duplicate = Duplicar
menu-reset-transform = Redefinir transformação
menu-toggle-gravity = Alternar gravidade
menu-bring-forward = Trazer para frente
menu-send-backward = Enviar para trás
menu-delete = Excluir

toggle-enter-edit = Entrar no modo edição
toggle-exit-edit = Sair do modo edição

palette-search-placeholder = Digite para buscar temas / presets…
palette-close-hint = Esc para fechar · Ctrl+K para alternar
palette-switch-theme = Mudar para o tema { $theme }
palette-apply-preset = Aplicar preset: { $preset }

settings-tab-library = Biblioteca

# Asset library tab
library-empty-headline = Nenhum asset indexado
library-empty-hint = Arraste arquivos para ~/.local/share/animaEngine/assets/ ou defina ANIMA_ASSETS_DIR.
library-no-asset-root = Diretório de assets não encontrado. Crie um em ~/.local/share/animaEngine/assets/
library-search-placeholder = Buscar assets…
library-add-to-scene = Adicionar à cena
library-sort-recent = Recentes
library-sort-name = Nome
library-kind-image = Imagem
library-kind-animated = Animado
library-kind-video = Vídeo
library-asset-added-toast = { $name } adicionado à cena
library-asset-add-failed-toast = Não foi possível adicionar { $name }
library-count = { $n } assets indexados

# ── Keybindings tab (D.1) — placeholder pending D.4 native-speaker audit
settings-tab-keybindings = Keybindings
keybindings-unbound = (unbound)
keybindings-add = Add
keybindings-recording = Press a chord… (Esc to cancel)
keybindings-conflict = Conflicts with { $action }
keybindings-reset-all = Reset all to defaults
keybindings-help = Custom shortcuts persist in config.toml

# ── Action labels (D.1.7) — placeholder pending D.4 native-speaker audit
action-toggle-edit-mode = Toggle edit mode
action-hide-overlay = Hide / show overlay
action-pause-all = Pause all animations
action-quit-with-save = Quit (save config)
action-save-now = Save config now
action-open-command-palette = Command palette
action-cycle-entity = Cycle to next entity
action-delete-selected = Delete selected entity
action-nudge-up = Nudge selection up
action-nudge-down = Nudge selection down
action-nudge-left = Nudge selection left
action-nudge-right = Nudge selection right
action-center-on-screen = Center selection on screen
action-toggle-visible = Toggle visibility
action-toggle-gravity = Toggle gravity
action-toggle-playback = Toggle play/pause
action-duplicate-selected = Duplicate selection
action-reset-transform = Reset scale / opacity
action-bring-forward = Bring selection forward
action-send-backward = Send selection backward
action-fps-up = Increase FPS
action-fps-down = Decrease FPS
action-opacity-up = Increase opacity
action-opacity-down = Decrease opacity
action-cycle-monitor = Cycle entity monitor pin
action-show-entity-info = Show entity info
action-show-help = Show keyboard help

# ── Accessibility section (D.3) — placeholder pending D.4 native-speaker audit
appearance-accessibility-header = Accessibility
appearance-accesskit-label = Generate AccessKit tree updates
appearance-accesskit-hint = Powers AT-SPI screen readers (Orca etc.). Leave on unless you want a tighter footprint or your desktop doesn't run an AT-SPI bus. Note: text you type in panels also appears on the AT-SPI bus, where any process running as your user can read it.
appearance-reduced-motion-label = Reduce motion
appearance-reduced-motion-hint = Skips UI transitions (panel slide, fades, palette pop) and stops decorative bouncing. Animations that convey state still play.

# ── Warning banners (D.5) — placeholder pending native-speaker audit
warning-global-hotkeys-unavailable = Global hotkeys couldn't register (typical on a native Wayland session). The tray menu and the ⚙ button still work.
warning-hot-reload-disconnected = The hot-reload worker stopped unexpectedly; in-flight config edits won't apply until you restart the app.
action-toggle-perf-overlay = Toggle perf overlay

# ── What's new (D.7) — placeholder pending native-speaker audit
whats-new-header = What's new in 0.4
whats-new-keybindings = Rebindable keyboard shortcuts — open the new Keybindings tab.
whats-new-collapse-state = Inspector sections remember their open/closed state across sessions.
whats-new-error-banners = Failure surfaces (silent before) now toast or banner — you'll see them.
whats-new-accessibility-toggle = AccessKit can be turned off from Appearance → Accessibility.
onboarding-keybindings = Click any chord to remove it; press a key combo to record a new one.
onboarding-perf-overlay = Press Ctrl+Shift+` to open the live perf overlay.
appearance-reset-onboarding = Reset onboarding hints

scene-empty-action-browse-presets = Browse presets
library-empty-action-copy-path = Copy path to clipboard

appearance-reset-onboarding-hint = Brings back the dismissed progressive hints and the "What's new" panel.

# ── Portal shortcuts (T.3) ────────────────────────────────────────────
portal-denied-x11-fallback-toast = Shortcut permission was declined — using X11 hotkeys instead. Retry from the Keybindings tab.
portal-denied-native-toast = Shortcut permission was declined — the tray menu and compositor bindings still work.

# ── Keybindings backend status (T.4) ─────────────────────────────────
keybindings-backend-label = Global shortcuts via:
keybindings-backend-tooltip = Which mechanism delivers the three global chords (toggle edit, hide, pause) while other apps have focus. Resolved at startup; in-app shortcuts are unaffected.
keybindings-portal-restart-hint = Trigger changes apply at the next launch (the desktop remembers your approval).

# ── Monitor hotplug (T.9) ─────────────────────────────────────────────
monitor-unplugged-toast = Monitor { $name } disconnected — { $n } pinned entities now follow their position.
monitor-plugged-toast = Monitor { $name } connected.

# ── Shimeji import (U.4) ──────────────────────────────────────────────
library-import-shimeji-header = Import Shimeji pack
library-import-shimeji-hint = Drop a pack folder onto the overlay, or paste its path here. Sprites are copied into the library.
library-import-shimeji-button = Import
shimeji-imported-toast = Imported { $name } ({ $n } parts skipped — see log)
shimeji-import-failed-toast = Import failed: { $reason }
shimeji-no-library-toast = No asset library root — create ~/.local/share/animaEngine/assets/ first.
crash-report-found-toast = The previous session crashed. A report was saved at { $path } — please attach it to a GitHub issue.

# ── Group composition hint (C.9) ──────────────────────────────────────
inspector-group-hint = Composed by group { $group }: { $transform }
