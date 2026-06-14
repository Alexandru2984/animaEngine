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
animation-easing-label = Suavização
easing-linear = Linear
easing-ease-in-quad = Entrada suave
easing-ease-out-quad = Saída suave
easing-ease-in-out-quad = Entrada/saída suave
easing-sine = Seno
easing-bounce-out = Quicar
inspector-section-behavior = Comportamento
inspector-visible = Visível
inspector-gravity = Gravidade
inspector-scale = Escala
inspector-behavior-speed = Velocidade
inspector-behavior-comfort = Distância de conforto
inspector-behavior-amplitude = Amplitude
inspector-behavior-period = Período
inspector-double-click-reset-hint = Clique duas vezes para restaurar o padrão.
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
scene-window-awareness = Pousar nas janelas (X11)
scene-window-awareness-tooltip = Personagens com física ativa pousam e caminham pela borda superior das janelas abertas. Apenas sessões X11 — o Wayland não expõe posições de janelas, então lá isso não faz nada.
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
onboarding-coach-step1 = Bem-vindo! Seus personagens vivem na área de trabalho. Clique no botão de engrenagem no canto superior direito para entrar no modo de edição.
onboarding-coach-step2 = Solte um PNG, GIF, WebP ou MP4 em qualquer lugar da tela para adicioná-lo como personagem. O painel lateral edita tudo o que você selecionar.
onboarding-coach-step3 = Ctrl+K abre a paleta de comandos. Ctrl+Shift+A alterna o modo de edição de qualquer lugar, Ctrl+Shift+H oculta o overlay.
onboarding-coach-next = Avançar
onboarding-coach-skip = Pular o tour
onboarding-coach-done = Entendi
palette-replace-row = Substituir a cena por: { $preset }
palette-append-row = Adicionar preset: { $preset }
palette-footer-hint = Esc fecha · Ctrl+K alterna · ↑↓ + Enter escolhe
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
settings-tab-keybindings = Atalhos
keybindings-unbound = (não atribuído)
keybindings-add = Adicionar
keybindings-recording = Pressione uma combinação… (Esc cancela)
keybindings-conflict = Conflita com { $action }
keybindings-reset-all = Restaurar tudo para o padrão
keybindings-help = Atalhos personalizados ficam salvos em config.toml

# ── Action labels (D.1.7) — placeholder pending D.4 native-speaker audit
action-toggle-edit-mode = Alternar modo de edição
action-hide-overlay = Ocultar / mostrar o overlay
action-pause-all = Pausar todas as animações
action-quit-with-save = Sair (salvando a configuração)
action-save-now = Salvar a configuração agora
action-open-command-palette = Paleta de comandos
action-cycle-entity = Ir para o próximo personagem
action-delete-selected = Excluir o personagem selecionado
action-nudge-up = Empurrar a seleção para cima
action-nudge-down = Empurrar a seleção para baixo
action-nudge-left = Empurrar a seleção para a esquerda
action-nudge-right = Empurrar a seleção para a direita
action-center-on-screen = Centralizar a seleção na tela
action-toggle-visible = Alternar visibilidade
action-toggle-gravity = Alternar gravidade
action-toggle-playback = Alternar reprodução/pausa
action-duplicate-selected = Duplicar a seleção
action-reset-transform = Redefinir escala / opacidade
action-bring-forward = Trazer a seleção para a frente
action-send-backward = Enviar a seleção para trás
action-fps-up = Aumentar FPS
action-fps-down = Diminuir FPS
action-opacity-up = Aumentar opacidade
action-opacity-down = Diminuir opacidade
action-cycle-monitor = Alternar fixação de monitor
action-show-entity-info = Mostrar informações do personagem
action-show-help = Mostrar ajuda do teclado

# ── Accessibility section (D.3) — placeholder pending D.4 native-speaker audit
appearance-accessibility-header = Acessibilidade
appearance-accesskit-label = Gerar atualizações da árvore AccessKit
appearance-accesskit-hint = Alimenta leitores de tela AT-SPI (Orca etc.). Deixe ligado, a menos que queira reduzir recursos ou seu desktop não tenha barramento AT-SPI. Atenção: o texto digitado nos painéis também aparece no barramento AT-SPI, onde qualquer processo do seu usuário pode lê-lo.
appearance-reduced-motion-label = Reduzir movimento
appearance-reduced-motion-hint = Pula as transições da interface (deslizar do painel, esmaecimentos, pop da paleta) e para o balanço decorativo. Animações que comunicam estado continuam ativas.

# ── Warning banners (D.5) — placeholder pending native-speaker audit
warning-global-hotkeys-unavailable = Não foi possível registrar os atalhos globais (típico em sessões Wayland nativas). O menu da bandeja e o botão ⚙ continuam funcionando.
warning-hot-reload-disconnected = O processo de hot-reload parou inesperadamente; edições de configuração em andamento só valerão após reiniciar o app.
action-toggle-perf-overlay = Alternar overlay de desempenho

# ── What's new (D.7) — placeholder pending native-speaker audit
whats-new-header = Novidades da 0.4
whats-new-keybindings = Atalhos de teclado reatribuíveis — abra a nova aba Atalhos.
whats-new-collapse-state = As seções do Inspetor lembram seu estado aberto/fechado entre sessões.
whats-new-error-banners = Superfícies de erro (antes silenciosas) agora mostram toasts ou banners — você as verá.
whats-new-accessibility-toggle = O AccessKit pode ser desligado em Aparência → Acessibilidade.
onboarding-keybindings = Clique em um atalho para removê-lo; pressione uma combinação para gravar um novo.
onboarding-perf-overlay = Pressione Ctrl+Shift+` para abrir o overlay de desempenho ao vivo.
appearance-reset-onboarding = Restaurar dicas de boas-vindas

scene-empty-action-browse-presets = Explorar presets
library-empty-action-copy-path = Copiar caminho para a área de transferência

appearance-reset-onboarding-hint = Traz de volta as dicas dispensadas e o painel “Novidades”.

# ── Portal shortcuts (T.3) ────────────────────────────────────────────
portal-denied-x11-fallback-toast = A permissão de atalhos foi negada — atalhos X11 em uso. Tente novamente na aba Atalhos.
portal-denied-native-toast = A permissão de atalhos foi negada — o menu da bandeja e os atalhos do compositor continuam funcionando.

# ── Keybindings backend status (T.4) ─────────────────────────────────
keybindings-backend-label = Atalhos globais via:
keybindings-backend-tooltip = Qual mecanismo entrega os três atalhos globais (editar, ocultar, pausar) enquanto outros apps têm o foco. Resolvido na inicialização; atalhos internos não são afetados.
keybindings-portal-restart-hint = Mudanças de atalho valem a partir da próxima inicialização (o desktop lembra sua aprovação).

# ── Monitor hotplug (T.9) ─────────────────────────────────────────────
monitor-unplugged-toast = Monitor { $name } desconectado — { $n } personagens fixados agora seguem sua posição.
monitor-plugged-toast = Monitor { $name } conectado.

# ── Shimeji import (U.4) ──────────────────────────────────────────────
library-import-shimeji-header = Importar pacote Shimeji
library-import-shimeji-hint = Arraste a pasta do pacote para o overlay ou cole o caminho aqui. Os sprites são copiados para a biblioteca.
library-import-shimeji-button = Importar
shimeji-imported-toast = { $name } importado ({ $n } partes ignoradas — veja o log)
shimeji-import-failed-toast = Falha na importação: { $reason }
shimeji-no-library-toast = Sem pasta de biblioteca — crie primeiro ~/.local/share/animaEngine/assets/.
crash-report-found-toast = A sessão anterior travou. Um relatório foi salvo em { $path } — anexe-o a um issue no GitHub.

# ── Group composition hint (C.9) ──────────────────────────────────────
inspector-group-hint = Composto pelo grupo { $group }: { $transform }

# ── App-layer toasts (V.6 — F1 closure) ──────────────────────────────
toast-config-saved = Configuração salva
toast-save-failed = Falha ao salvar: { $error }
toast-rejected = Rejeitado: { $reason }
toast-added = { $name } adicionado
toast-load-failed = Falha ao carregar: { $error }
toast-entity-load-failed = { $name }: { $error }
toast-theme-switched = Tema: { $theme }
toast-preset-entry-failed = Não foi possível adicionar a entrada do preset: { $error }
toast-preset-loaded = Preset carregado: { $name }
toast-duplicated = { $name } duplicado
toast-duplicate-failed = Falha ao duplicar: { $error }
toast-deleted = { $name } excluído
toast-playback-resumed = Reprodução retomada
toast-playback-paused = Reprodução pausada
toast-wayland-no-library = A biblioteca de assets ainda não está disponível no caminho Wayland
inspector-wander-box = Área de perambulação
toast-perf-snapshot = Snapshot de desempenho: { $path }
toast-perf-snapshot-failed = Falha no snapshot: { $error }
