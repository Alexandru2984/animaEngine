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
inspector-section-talk = Diz
inspector-talk-hint = Uma frase por linha. Um toque no personagem no modo de edição o faz dizer uma, e ele também pode falar sozinho.
inspector-talk-placeholder = Olá!
inspector-talk-pace = Fala
inspector-talk-only-poked = Só ao ser tocado
inspector-talk-every = A cada { $minutes } min, mais ou menos
inspector-talk-say-now = Dizer uma agora
placeholders-hint = Numa fala, { $time }, { $day }, { $date } e { $name } são preenchidos quando ela é dita.
placeholder-date = { $day } de { $month }
weekday-0 = domingo
weekday-1 = segunda-feira
weekday-2 = terça-feira
weekday-3 = quarta-feira
weekday-4 = quinta-feira
weekday-5 = sexta-feira
weekday-6 = sábado
month-1 = janeiro
month-2 = fevereiro
month-3 = março
month-4 = abril
month-5 = maio
month-6 = junho
month-7 = julho
month-8 = agosto
month-9 = setembro
month-10 = outubro
month-11 = novembro
month-12 = dezembro
inspector-sound-label = Som ao tocar
inspector-sound-none = Nenhum
inspector-sound-play = Tocar
inspector-sound-hint = Os arquivos de som (.ogg, .wav, .flac, .mp3) colocados em { $path } aparecem aqui.
inspector-bubble-colour = Cor do balão
inspector-bubble-size = Tamanho do texto
inspector-bubble-reset = Padrão
inspector-hours-label = Só em certos horários
inspector-hours-hint = Fora desses horários, ele não aparece na tela — o fantasma à noite, o sol de dia. No modo de edição todos aparecem, para serem editados.
inspector-hours-from = das
inspector-hours-to = às
inspector-visible = Visível
inspector-gravity = Gravidade
inspector-scale = Escala
inspector-behavior-speed = Velocidade
inspector-follow-target = Segue
inspector-follow-distance = Distância
inspector-follow-nobody = (ninguém)
inspector-follow-none-free = Ninguém para seguir: os outros não estão aqui ou já seguem este.
inspector-tag-partner = Brinca com
inspector-tag-hint = Eles se perseguem por um tempo — quem está com a vez corre atrás do outro — e depois descansam um minuto e meio. O outro não precisa de nenhuma configuração.
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
behavior-follow-character = Seguir um personagem
behavior-play-tag = Brincar de pega-pega
behavior-wander = Vagar limitado
behavior-bounce = Pulo
behavior-bounce-axis = Eixo
behavior-bounce-horizontal = Horizontal
behavior-bounce-vertical = Vertical
behavior-bounce-both = Ambos (círculo)
behavior-script = Script
behavior-script-path-label = Caminho
behavior-script-path-hint = Relativo à sua biblioteca de recursos — { $path }
behavior-script-params = Parâmetros
behavior-script-param-name = nome
behavior-script-add-param = Adicionar
behavior-script-remove-param = Remover este parâmetro
script-failed-toast = O script de comportamento { $script } falhou: { $error }
sound-failed-toast = Não foi possível tocar o som { $sound }: { $error }

scene-empty-headline = Cena vazia
scene-empty-hint = Arraste um PNG / GIF / WebP / MP4 para o overlay — ou experimente um preset abaixo.
scene-drop-hint = Arraste um PNG / GIF / WebP para o overlay para adicionar uma entidade.
# "Add file…" (1.4): machine-translated, pending native review.
scene-add-file = Adicionar arquivo…
scene-add-file-tooltip = Escolha imagens ou vídeos para adicionar como personagens.
scene-paste = Colar
scene-paste-tooltip = Adiciona os personagens copiados a esta cena. Copiados: { $count }
scene-paste-empty = Cole uma imagem ou arquivos copiados em outro lugar — ou personagens: selecione-os e pressione Ctrl+C, ou clique com o botão direito neles e escolha Copiar, nesta cena ou em outra.
file-chooser-title = Adicionar personagens
file-chooser-filter = Imagens e vídeos
share-chooser-title = Compartilhar uma cena
gif-chooser-title = Salvar um GIF
character-chooser-title = Compartilhar um personagem
backup-chooser-title = Backup do animaEngine
restore-chooser-title = Restaurar um backup
restore-chooser-filter = Backups do animaEngine
import-chooser-title = Importar uma cena
import-chooser-filter = Cenas e personagens do animaEngine
file-chooser-unavailable-toast = Não foi possível abrir o seletor de arquivos. Ele precisa do xdg-desktop-portal.
scene-presets-header = Presets
scene-groups-header = Grupos
scene-preset-append = Adicionar
scene-preset-replace = Substituir
scene-preset-replace-tooltip = Limpa a cena atual antes de adicionar

monitor-section-header = Monitores
monitor-mode-label = Distribuição
monitor-mode-per-monitor = Por monitor
monitor-mode-span = Estender por todos os monitores
monitor-mode-span-unsupported = Neste backend uma sobreposição não pode cobrir vários monitores — use por monitor.
monitor-mode-single = Monitor único
scene-window-awareness = Pousar nas janelas (X11)
scene-window-awareness-tooltip = Personagens com física ativa pousam e caminham pela borda superior das janelas abertas. Apenas sessões X11 — o Wayland não expõe posições de janelas, então lá isso não faz nada.
scene-window-awareness-unavailable = Indisponível no Wayland nativo — nenhum protocolo expõe as posições das janelas.
monitor-pin-label = Fixar ao monitor
monitor-pin-auto = Auto (segue a posição)
monitor-pinned-toast = Entidade fixada em { $name }
monitor-pin-cleared-toast = Entidade agora segue sua posição
monitor-no-monitors-detected = Nenhum monitor detectado

appearance-theme-header = Tema
appearance-theme-label = Tema
appearance-language-header = Idioma
appearance-language-no-font = Nenhuma fonte instalada para esta escrita — instale primeiro um pacote Noto CJK.
theme-dark = Escuro
theme-light = Claro
theme-dark-hc = Escuro · Alto contraste
theme-light-hc = Claro · Alto contraste

onboarding-tabs = As configurações ficam em cinco abas — Inspetor, Cena, Biblioteca, Aparência, Atalhos.
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
menu-copy = Copiar
menu-save-gif = Salvar como GIF…
menu-share-character = Compartilhar personagem…
menu-cut = Recortar
menu-reset-transform = Redefinir transformação
menu-toggle-gravity = Alternar gravidade
menu-bring-forward = Trazer para frente
menu-send-backward = Enviar para trás
menu-delete = Excluir

toggle-enter-edit = Entrar no modo edição
toggle-exit-edit = Sair do modo edição

# Palette placeholder (1.4): machine-translated, pending native review.
palette-search-placeholder = Digite para buscar ações, temas e presets…
palette-switch-theme = Mudar para o tema { $theme }

settings-tab-library = Biblioteca

# Asset library tab
library-empty-headline = Nenhum asset indexado
library-empty-hint = Arraste arquivos para { $path } ou defina ANIMA_ASSETS_DIR.
library-no-asset-root = Diretório de assets não encontrado. Crie um em { $path }
library-search-placeholder = Buscar assets…
library-add-to-scene = Adicionar à cena
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
keybindings-reset-one = Redefinir para o padrão
keybindings-remove-chord = Remover esta associação
keybindings-help = Atalhos personalizados ficam salvos em config.toml
# Modifier names as this language's keyboards print them. Display only:
# config.toml always stores the English names. Kept in English unless
# the convention is certain; see R39 in docs/runtime-findings.md.
key-mod-ctrl = Ctrl
key-mod-shift = Shift
key-mod-alt = Alt
key-mod-super = Super

# ── Action labels (D.1.7) — placeholder pending D.4 native-speaker audit
action-toggle-edit-mode = Alternar modo de edição
action-hide-overlay = Ocultar / mostrar o overlay
action-pause-all = Pausar todas as animações
action-toggle-focus = Iniciar ou parar o timer de foco
action-toggle-snooze = Soneca de 15 minutos ou acordar
action-quit-with-save = Sair (salvando a configuração)
action-save-now = Salvar a configuração agora
action-open-command-palette = Paleta de comandos
# Undo (1.5): machine-translated, pending native review.
action-undo = Desfazer
action-redo = Refazer

# Groups (1.5): machine-translated, pending native review.
action-group-selected = Agrupar a seleção
action-ungroup-selected = Desagrupar a seleção
menu-group = Agrupar
menu-ungroup = Desagrupar
group-default-name = Grupo { $number }
group-copy-name = { $name } (cópia)
toast-grouped = Agrupados como { $name }
toast-ungrouped = Grupos desfeitos: { $count }
toast-nothing-to-ungroup = Nada do que está selecionado está em um grupo
scene-groups-empty-hint = Selecione vários personagens e pressione Ctrl+G — ou clique com o botão direito neles e escolha Agrupar.
scene-group-members = Personagens: { $count }
scene-group-select-tooltip = Selecionar os personagens deste grupo
scene-group-rename = Renomear o grupo
scene-group-show = Mostrar o grupo
scene-group-hide = Ocultar o grupo
scene-group-ungroup = Desagrupar — os personagens ficam

# Arrange and snap (1.5): machine-translated, pending native review.
arrange-align-left = Alinhar bordas esquerdas
arrange-align-center = Centralizar numa linha vertical
arrange-align-right = Alinhar bordas direitas
arrange-align-top = Alinhar bordas superiores
arrange-align-middle = Centralizar numa linha horizontal
arrange-align-bottom = Alinhar bordas inferiores
arrange-distribute-horizontally = Distribuir igualmente na horizontal
arrange-distribute-vertically = Distribuir igualmente na vertical
scene-snap = Encaixar ao arrastar
scene-snap-tooltip = Bordas e centros se encaixam nos das telas e de outros personagens. Segure Alt ao arrastar para posicionar livremente.
scene-bump = Os personagens esbarram uns nos outros
scene-bump-tooltip = Com a gravidade ligada (G), os personagens pousam na cabeça dos outros; os que andam dão meia-volta ao se encontrarem.
scene-greet = Os personagens se cumprimentam quando se encontram
scene-greet-tooltip = Dois que se aproximam — andando, seguindo, aterrissando um no outro — param um instante e se cumprimentam, com um coração. Não com muita frequência.
greet-hi = Oi!
greet-hello = Olá!
greet-hey = E aí!

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
action-copy-selected = Copiar a seleção
action-cut-selected = Recortar a seleção
action-paste = Colar
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
# Screen readers (1.4): machine-translated, pending native review.
appearance-accesskit-hint = Permite que leitores de tela (Orca e outros, via AT-SPI) leiam e usem os painéis. Desligado, um leitor de tela vê uma janela vazia. Atenção: enquanto um estiver em execução, o texto digitado nos painéis fica no barramento AT-SPI, onde qualquer processo do seu usuário pode lê-lo.
# Machine-translated, pending native review.
appearance-accesskit-unsupported = Leitores de tela ainda não são suportados neste sistema.
appearance-reduced-motion-label = Reduzir movimento
appearance-reduced-motion-hint = Pula as transições da interface (deslizar do painel, esmaecimentos, pop da paleta) e para o balanço decorativo. Animações que comunicam estado continuam ativas.
appearance-large-bubbles-label = Texto grande nos balões
appearance-large-bubbles-hint = O texto de todos os balões fica maior, para ler de longe ou com baixa visão.
appearance-hover-startle-label = Sobressalto ao passar o cursor
appearance-hover-startle-hint = Os mascotes se afastam do ponteiro quando ele se aproxima. O rastreamento do cursor é só no X11, então no Wayland nativo isso só reage no modo de edição.
# Full-screen apps (1.5): machine-translated, pending native review.
appearance-fullscreen-label = Quando um app está em tela cheia
appearance-fullscreen-hint = Jogos, vídeos e apresentações que ocupam a tela. Funciona no X11 e em compositores Wayland como sway e Hyprland; no GNOME e no KDE com Wayland, só para apps que rodam pelo XWayland.
appearance-apps-label = Ocultar também com estes apps
appearance-apps-hint = Enquanto um deles for a janela em primeiro plano — uma chamada de vídeo, uma transmissão, um jogo —, os personagens se ocultam, como com um app em tela cheia. X11 e compositores Wayland como sway e Hyprland; no GNOME e no KDE com Wayland, só apps que rodam pelo XWayland.
appearance-apps-remove = Remover { $app }
appearance-apps-add = Adicionar um app aberto…
appearance-apps-none-open = Nenhum app que esta sessão consegue ver está aberto.
appearance-behind-windows-label = Atrás das suas janelas
appearance-behind-windows-hint = Os personagens vivem na área de trabalho, sob as suas janelas, em vez de por cima delas. O modo de edição os traz para a frente; se o botão ⚙ estiver coberto, abra-o com Ctrl+Shift+A ou pela bandeja.
appearance-tap-label = Dá para tocar nos personagens
appearance-tap-hint = Também fora do modo de edição: um toque num personagem dá uma cutucada — um pulo e uma frase dele. Cliques em qualquer outro lugar continuam chegando às suas janelas, e o app em que você digita fica com o teclado.
appearance-effects-label = Pequenos efeitos
appearance-effects-hint = Corações quando um personagem é tocado, uma nuvem de poeira onde um aterrissa com força e zês quando adormecem. Nenhum com Reduzir movimento.
appearance-bedtime-label = Hora de dormir
appearance-bedtime-hint = Os personagens cochilam entre estes horários, seja quem for que estiver ao teclado; o modo de edição os acorda enquanto durar. A noite conta como pausa para os lembretes.
appearance-moods-label = Humor conforme a hora do dia
appearance-moods-hint = Mais animados de manhã, mais lentos à noite — o andar e a animação seguem o relógio — e, com a hora de dormir ligada, um bocejo na meia hora antes.
mood-yawn = *bocejo*
appearance-bedtime-from = das
appearance-bedtime-to = às
fullscreen-hide = Esconder os personagens
fullscreen-pause = Pausá-los
fullscreen-ignore = Deixá-los continuar
# Pausing when away (1.5): machine-translated, pending native review.
appearance-away-label = Pausar quando eu sair
appearance-away-hint = Sem mouse nem teclado por esse tempo, os personagens param, pousam e cochilam; sua próxima ação os acorda.
away-never = Nunca
away-after-minutes = Após { $minutes } min
appearance-away-unavailable = Esta sessão não consegue saber quando você saiu: é preciso GNOME, um servidor X de verdade ou um compositor Wayland com avisos de inatividade.
appearance-battery-label = Pausar na bateria
appearance-battery-hint = Os personagens ficam parados enquanto o computador funciona na bateria.
appearance-welcome-label = Me cumprimentar quando eu voltar
appearance-welcome-hint = Depois de um tempo longe do computador, um personagem cumprimenta você na volta — “Bom dia!” depois da noite. Não na hora de dormir.
appearance-backup = Fazer backup…
appearance-backup-hint = Tudo em um arquivo — configurações, cenas salvas e suas imagens — para guardar ou levar para outro computador.
appearance-restore = Restaurar…
appearance-restore-hint = Traga de volta um backup: as cenas dele são adicionadas às suas e as configurações dele substituem estas (as antigas ficam em config.toml.before-restore).
welcome-back = Que bom que você voltou!
welcome-morning = Bom dia!
# Start at login (1.5): machine-translated, pending native review.
appearance-autostart-label = Iniciar ao entrar
appearance-autostart-hint = Inicia o animaEngine quando você entra na área de trabalho.
appearance-autostart-reason = Para iniciar o animaEngine ao entrar na sessão.
appearance-autostart-failed = Não foi possível mudar: { $error }
# Named scenes (1.5): machine-translated, pending native review.
scene-scenes-header = Cenas
scene-scenes-hint = Salve o que está na tela com um nome e volte a ele aqui, pela paleta de comandos ou com “Next scene” na bandeja.
scene-scene-name-hint = Nome da cena
scene-scene-save-new = Salvar como cena
scene-scene-load-tooltip = Mudar para esta cena — Desfazer volta
scene-scene-save-over = Salvar o que está na tela como { $name }
scene-scene-share = Compartilhar { $name } como um só arquivo, com as imagens
scene-import = Importar…
scene-import-tooltip = Abra uma cena que compartilharam com você (um arquivo .animascene). Soltar o arquivo sobre a sobreposição também funciona.
scene-scene-delete = Excluir cena
scene-scene-delete-confirm = Excluir?
scene-scene-delete-cancel = Manter
scene-scene-save-failed = Não salva: { $error }
toast-scene-loaded = Cena: { $name }
toast-scene-failed = Cena não carregada: { $error }
toast-scene-shared = Cena pronta para compartilhar: { $path }
toast-scene-shared-scripts = Scripts de comportamento não são compartilhados; personagens parados no arquivo: { $count }
toast-scene-share-failed = Falha ao compartilhar: { $error }
toast-gif-saved = GIF salvo: { $path }
toast-gif-failed = Não foi possível salvar o GIF: { $error }
toast-character-shared = Personagem pronto para compartilhar: { $path }
toast-character-shared-script = O script de comportamento não é compartilhado: no arquivo, ele fica parado.
toast-character-added = { $name } adicionado
toast-characters-added = Personagens adicionados: { $count }
toast-backup-saved = Backup salvo: { $path }
toast-backup-failed = Falha no backup ou na restauração: { $error }
toast-restored = Restaurado: as configurações, e cenas adicionadas: { $count }
toast-scene-imported = Cena importada: { $name }
toast-scene-import-failed = Falha ao importar: { $error }
toast-no-scenes = Nenhuma cena salva ainda — salve uma na aba Cena.
palette-load-scene = Mudar para a cena: { $name }
palette-snooze-minutes = Soneca de { $minutes } minutos
palette-focus-start = Iniciar o timer de foco
palette-focus-stop = Parar o timer de foco
palette-snooze-morning = Soneca até de manhã
# Reminders (1.5): machine-translated, pending native review.
scene-reminders-header = Lembretes
scene-reminders-hint = Um personagem diz isso de vez em quando, num balão. O tempo longe do computador conta como pausa e recomeça a contagem.
scene-focus-header = Timer de foco
scene-focus-hint = Trabalhe um pouco, e um personagem avisa quando é hora de uma pausa — e quando voltar. Enquanto isso, os lembretes esperam. Também pela paleta de comandos ou com “Start focus” na bandeja.
scene-focus-work = Trabalho
scene-focus-break = Pausa
scene-focus-start = Iniciar
scene-focus-stop = Parar
scene-focus-working = Foco até { $until }
scene-focus-on-break = Em pausa até { $until }
reminder-sound-label = Som com os lembretes e o timer de foco
reminder-sound-none = Nenhum
reminder-sound-speaker = O som de toque de quem fala
reminder-notify-label = Também como notificações da área de trabalho
reminder-notify-hint = Lembretes e o timer de foco também vão para as notificações da área de trabalho — na hora, quando os personagens não estão visíveis, em vez de esperar por eles.
focus-said-started = Hora de focar — até { $until }.
focus-said-break = Pausa! Voltamos às { $until }.
focus-said-back = Fim da pausa — de volta ao trabalho até { $until }.
scene-reminder-every = a cada { $minutes } min
scene-reminder-anyone = qualquer personagem
scene-reminder-delete = Excluir lembrete
scene-reminder-text-hint = O que dizer
scene-reminder-minutes = A cada (minutos)
scene-reminder-timed = Em um horário definido
scene-reminder-at = às
scene-reminder-on-date = em uma data
scene-reminder-days = Dias
scene-reminder-date = Data
scene-reminder-who = Quem diz
scene-reminder-add = Adicionar lembrete
reminder-break = Hora de uma pausa curta!
reminder-water = Beba um copo d’água.
reminder-stretch = Levante-se e alongue-se.
# Scenes by time of day (1.5): machine-translated, pending native review.
scene-schedule-header = Trocar automaticamente
scene-schedule-hint = Troca para uma cena salva em horários fixos — Trabalho às 09:00 nos dias úteis, por exemplo. Não enquanto você edita: aí espera.
scene-schedule-needs-scene = Salve uma cena acima para trocar para ela conforme um horário.
scene-schedule-row = { $time } · { $days } → { $scene }
scene-schedule-every-day = todos os dias
scene-schedule-weekdays = dias úteis
scene-schedule-weekends = fins de semana
scene-schedule-at = Às
scene-schedule-minute = Minuto
scene-schedule-days = Dias
scene-schedule-scene = Cena
scene-schedule-add = Adicionar
scene-schedule-delete = Excluir regra

# ── Warning banners (D.5) — placeholder pending native-speaker audit
warning-global-hotkeys-unavailable = Não foi possível registrar os atalhos globais (típico em sessões Wayland nativas). O menu da bandeja e o botão ⚙ continuam funcionando.
warning-hot-reload-disconnected = O processo de hot-reload parou inesperadamente; edições de configuração em andamento só valerão após reiniciar o app.
# Machine-translated, pending native review.
warning-config-unreadable = Não foi possível ler o arquivo de configuração, então a cena padrão foi carregada. O original foi mantido ao lado como config.toml.bak-corrupt.
action-toggle-perf-overlay = Alternar overlay de desempenho

# ── What's new (D.7) — placeholder pending native-speaker audit
# 1.4 highlights: machine-translated, pending native review.
whats-new-header = Novidades da 1.4
whats-new-add-file = “Adicionar arquivo…”, na aba Cena, adiciona personagens pelo seletor de arquivos do seu desktop — sem arrastar nada.
whats-new-palette = Ctrl+K agora executa qualquer ação e mostra o atalho ao lado.
whats-new-screen-readers = Leitores de tela como o Orca agora podem ler e operar o painel de configurações.
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
shimeji-no-library-toast = Sem pasta de biblioteca — crie primeiro { $path }.
crash-report-found-toast = A sessão anterior travou. Um relatório foi salvo em { $path } — anexe-o a um issue no GitHub.

# ── Group composition hint (C.9) ──────────────────────────────────────
inspector-group-hint = Composto pelo grupo { $group }: { $transform }
# Multiple selection (1.5): machine-translated, pending native review.
inspector-multi-selected = { $count } selecionados. Este painel edita { $name }; arrastar, os atalhos e o menu do botão direito agem em todos.

# ── App-layer toasts (V.6 — F1 closure) ──────────────────────────────
toast-config-saved = Configuração salva
# Hot-reload toasts: machine-translated, pending native review.
toast-config-reloaded = Configuração recarregada do disco
toast-config-reload-failed = Configuração não recarregada — o arquivo é inválido ou ainda está sendo gravado. A cena atual foi mantida.
toast-config-reload-discarded = Configuração não recarregada — você editou a cena enquanto ela carregava.
toast-save-failed = Falha ao salvar: { $error }
toast-rejected = Rejeitado: { $reason }
toast-added = { $name } adicionado
toast-load-failed = Falha ao carregar: { $error }
toast-entity-load-failed = { $name }: { $error }
toast-theme-switched = Tema: { $theme }
toast-preset-entry-failed = Não foi possível adicionar a entrada do preset: { $error }
toast-preset-loaded = Preset carregado: { $name }
toast-duplicated = { $name } duplicado
# Multiple selection (1.5): machine-translated, pending native review.
toast-duplicated-many = Personagens duplicados: { $count }
toast-duplicate-failed = Falha ao duplicar: { $error }
toast-deleted = { $name } excluído
# Multiple selection (1.5): machine-translated, pending native review.
toast-deleted-many = Personagens excluídos: { $count }
toast-copied = { $name } copiado
toast-copied-many = Personagens copiados: { $count }
toast-cut = { $name } recortado
toast-cut-many = Personagens recortados: { $count }
toast-pasted = { $name } colado
toast-pasted-many = Personagens colados: { $count }
toast-paste-failed = Falha ao colar: { $error }
toast-nothing-to-paste = Nada para colar: copie um personagem ou uma imagem primeiro
pasted-image-name = Imagem colada
toast-playback-resumed = Reprodução retomada
toast-playback-paused = Reprodução pausada
# Undo (1.5): machine-translated, pending native review.
toast-undone = Desfeito
toast-redone = Refeito
toast-nothing-to-undo = Nada para desfazer
toast-nothing-to-redo = Nada para refazer
inspector-wander-box = Área de perambulação
toast-perf-snapshot = Snapshot de desempenho: { $path }
toast-perf-snapshot-failed = Falha no snapshot: { $error }
