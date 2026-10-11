# Español — traducción base. Revisión por hablante nativo pendiente.

app-name = animaEngine

settings-tab-inspector = Inspector
settings-tab-scene = Escena
settings-tab-appearance = Apariencia
entity-count-zero = Sin entidades
entity-count-singular = { $n } entidad
entity-count-plural = { $n } entidades

inspector-section-position = Posición
inspector-section-appearance = Apariencia
inspector-section-animation = Animación
animation-easing-label = Suavizado
easing-linear = Lineal
easing-ease-in-quad = Entrada suave
easing-ease-out-quad = Salida suave
easing-ease-in-out-quad = Entrada/salida suave
easing-sine = Seno
easing-bounce-out = Rebote
inspector-section-behavior = Comportamiento
inspector-section-talk = Dice
inspector-talk-hint = Una frase por línea. Un toque en el personaje en modo edición le hace decir una, y también puede hablar por su cuenta.
inspector-talk-placeholder = ¡Hola!
inspector-talk-pace = Habla
inspector-talk-only-poked = Solo al tocarlo
inspector-talk-every = Cada { $minutes } min aprox.
inspector-talk-say-now = Decir una ahora
placeholders-hint = En una frase, { $time }, { $day }, { $date } y { $name } se completan al decirla.
placeholder-date = { $day } de { $month }
weekday-0 = domingo
weekday-1 = lunes
weekday-2 = martes
weekday-3 = miércoles
weekday-4 = jueves
weekday-5 = viernes
weekday-6 = sábado
month-1 = enero
month-2 = febrero
month-3 = marzo
month-4 = abril
month-5 = mayo
month-6 = junio
month-7 = julio
month-8 = agosto
month-9 = septiembre
month-10 = octubre
month-11 = noviembre
month-12 = diciembre
inspector-sound-label = Sonido al tocar
inspector-sound-none = Ninguno
inspector-sound-play = Reproducir
inspector-sound-hint = Los archivos de sonido (.ogg, .wav, .flac, .mp3) que pongas en { $path } aparecen aquí.
inspector-bubble-colour = Color del bocadillo
inspector-bubble-size = Tamaño del texto
inspector-bubble-reset = Predeterminado
inspector-hours-label = Solo a ciertas horas
inspector-hours-hint = Fuera de estas horas no aparece en pantalla: el fantasma de noche, el sol de día. En el modo de edición están todos, para poder editarlos.
inspector-hours-from = de
inspector-hours-to = a
inspector-visible = Visible
inspector-gravity = Gravedad
inspector-scale = Escala
inspector-behavior-speed = Velocidad
inspector-follow-target = Sigue a
inspector-follow-distance = Distancia
inspector-follow-nobody = (nadie)
inspector-follow-none-free = No hay a quién seguir: los demás no están o ya siguen a este.
inspector-tag-partner = Juega con
inspector-tag-hint = Se persiguen un rato —quien la lleva corre tras el otro— y luego descansan un minuto y medio. El otro no necesita ningún ajuste.
inspector-behavior-comfort = Distancia de confort
inspector-behavior-amplitude = Amplitud
inspector-behavior-period = Periodo
inspector-double-click-reset-hint = Doble clic para restablecer el valor predeterminado.
inspector-opacity = Opacidad
inspector-fps = FPS
inspector-playing = Reproduciendo
inspector-x = X
inspector-y = Y
inspector-z-index = z-index
inspector-nothing-selected-headline = Nada seleccionado
inspector-nothing-selected-hint = Haz clic en una entidad de la pestaña Escena, o presiona Tab para recorrerlas.

behavior-idle = En reposo
behavior-walk = Caminar
behavior-follow = Seguir el cursor
behavior-follow-character = Seguir a un personaje
behavior-play-tag = Jugar a pillar
behavior-wander = Vagar acotado
behavior-bounce = Rebote
behavior-bounce-axis = Eje
behavior-bounce-horizontal = Horizontal
behavior-bounce-vertical = Vertical
behavior-bounce-both = Ambos (círculo)
behavior-script = Script
behavior-script-path-label = Ruta
behavior-script-path-hint = Relativa a tu biblioteca de recursos — { $path }
behavior-script-params = Parámetros
behavior-script-param-name = nombre
behavior-script-add-param = Añadir
behavior-script-remove-param = Eliminar este parámetro
script-failed-toast = El script de comportamiento { $script } falló: { $error }
sound-failed-toast = No se pudo reproducir el sonido { $sound }: { $error }

scene-empty-headline = Escena vacía
scene-empty-hint = Arrastra un PNG / GIF / WebP / MP4 sobre el overlay — o prueba un preset abajo.
scene-drop-hint = Arrastra un PNG / GIF / WebP sobre el overlay para añadir una entidad.
# "Add file…" (1.4): machine-translated, pending native review.
scene-add-file = Añadir archivo…
scene-add-file-tooltip = Elige imágenes o vídeos para añadirlos como personajes.
scene-paste = Pegar
scene-paste-tooltip = Añade los personajes copiados a esta escena. Copiados: { $count }
scene-paste-empty = Pega una imagen o archivos copiados en otro sitio, o personajes: selecciónalos y pulsa Ctrl+C, o haz clic derecho sobre ellos y elige Copiar, en esta escena o en otra.
file-chooser-title = Añadir personajes
file-chooser-filter = Imágenes y vídeos
share-chooser-title = Compartir una escena
gif-chooser-title = Guardar un GIF
character-chooser-title = Compartir un personaje
backup-chooser-title = Copia de seguridad de animaEngine
restore-chooser-title = Restaurar una copia de seguridad
restore-chooser-filter = Copias de seguridad de animaEngine
import-chooser-title = Importar una escena
import-chooser-filter = Escenas y personajes de animaEngine
file-chooser-unavailable-toast = No se pudo abrir el selector de archivos. Necesita xdg-desktop-portal.
scene-presets-header = Presets
scene-groups-header = Grupos
scene-characters-header = Personajes
scene-together-header = En conjunto
scene-preset-append = Añadir
scene-preset-replace = Reemplazar
scene-preset-replace-tooltip = Limpia la escena actual antes de añadir

monitor-section-header = Monitores
monitor-mode-label = Distribución
monitor-mode-per-monitor = Por monitor
monitor-mode-span = Extender en todos los monitores
monitor-mode-span-unsupported = En este backend una sola superposición no puede cubrir varios monitores; usa «por monitor».
monitor-mode-single = Un solo monitor
scene-window-awareness = Aterrizar en ventanas (X11)
scene-window-awareness-tooltip = Los personajes con física activa aterrizan y caminan por el borde superior de sus ventanas abiertas. Solo sesiones X11 — Wayland no expone posiciones de ventanas, así que ahí no hace nada.
scene-window-awareness-unavailable = No disponible en Wayland nativo: ningún protocolo expone las posiciones de las ventanas.
monitor-pin-label = Fijar al monitor
monitor-pin-auto = Auto (sigue la posición)
monitor-pinned-toast = Entidad fijada a { $name }
monitor-pin-cleared-toast = La entidad sigue ahora su posición
monitor-no-monitors-detected = No se detectaron monitores

appearance-theme-header = Tema
appearance-theme-label = Tema
appearance-language-header = Idioma
appearance-language-no-font = No hay ninguna fuente instalada para esta escritura; instala primero un paquete Noto CJK.
theme-dark = Oscuro
theme-light = Claro
theme-dark-hc = Oscuro · Alto contraste
theme-light-hc = Claro · Alto contraste

onboarding-tabs = Los ajustes se reparten en cinco pestañas: Inspector, Escena, Biblioteca, Apariencia, Atajos.
onboarding-quick-toggles = Consejo: V alterna la visibilidad, G alterna la gravedad — sin abrir este panel.
onboarding-theme = Los temas se aplican al instante — no hace falta reiniciar.
onboarding-coach-step1 = ¡Bienvenido! Tus personajes viven en el escritorio. Haz clic en el botón de engranaje de la esquina superior derecha para entrar en el modo edición.
onboarding-coach-step2 = Suelta un PNG, GIF, WebP o MP4 en cualquier parte de la pantalla para añadirlo como personaje. El panel lateral edita todo lo que selecciones.
onboarding-coach-step3 = Ctrl+K abre la paleta de comandos. Ctrl+Shift+A alterna el modo edición desde cualquier sitio, Ctrl+Shift+H oculta el overlay.
onboarding-coach-next = Siguiente
onboarding-coach-skip = Saltar el tour
onboarding-coach-done = Entendido
palette-replace-row = Reemplazar la escena con: { $preset }
palette-append-row = Añadir preset: { $preset }
palette-footer-hint = Esc cierra · Ctrl+K alterna · ↑↓ + Enter elige
onboarding-dismiss = Cerrar

menu-duplicate = Duplicar
menu-copy = Copiar
menu-save-gif = Guardar como GIF…
menu-share-character = Compartir personaje…
menu-cut = Cortar
menu-reset-transform = Restablecer transformación
menu-toggle-gravity = Alternar gravedad
menu-bring-forward = Traer al frente
menu-send-backward = Enviar al fondo
menu-delete = Eliminar

toggle-enter-edit = Entrar al modo edición
toggle-exit-edit = Salir del modo edición

# Palette placeholder (1.4): machine-translated, pending native review.
palette-search-placeholder = Escribe para buscar acciones, temas y presets…
palette-switch-theme = Cambiar al tema { $theme }

settings-tab-library = Biblioteca

# Asset library tab
library-empty-headline = Sin activos indexados
library-empty-hint = Arrastra archivos a { $path } o configura ANIMA_ASSETS_DIR.
library-no-asset-root = Directorio de assets no encontrado. Crea uno en { $path }
library-search-placeholder = Buscar activos…
library-add-to-scene = Añadir a la escena
library-kind-image = Imagen
library-kind-animated = Animado
library-kind-video = Vídeo
library-asset-added-toast = { $name } añadido a la escena
library-asset-add-failed-toast = No se pudo añadir { $name }
library-count = { $n } activos indexados

# ── Keybindings tab (D.1) — placeholder pending D.4 native-speaker audit
settings-tab-keybindings = Atajos
keybindings-unbound = (sin asignar)
keybindings-add = Añadir
keybindings-recording = Pulsa una combinación… (Esc cancela)
keybindings-conflict = Entra en conflicto con { $action }
keybindings-reset-all = Restablecer todo a los valores predeterminados
keybindings-reset-one = Restablecer al valor predeterminado
keybindings-remove-chord = Eliminar esta asignación
keybindings-help = Los atajos personalizados se guardan en config.toml
# Modifier names as this language's keyboards print them. Display only:
# config.toml always stores the English names. Kept in English unless
# the convention is certain; see R39 in docs/runtime-findings.md.
key-mod-ctrl = Ctrl
key-mod-shift = Shift
key-mod-alt = Alt
key-mod-super = Super

# ── Action labels (D.1.7) — placeholder pending D.4 native-speaker audit
action-toggle-edit-mode = Alternar modo edición
action-hide-overlay = Ocultar / mostrar el overlay
action-pause-all = Pausar todas las animaciones
action-toggle-focus = Iniciar o detener el temporizador
action-toggle-snooze = Dormitar 15 minutos o despertar
action-toggle-mute = Silenciar o activar los sonidos
action-quit-with-save = Salir (guardando la configuración)
action-save-now = Guardar la configuración ahora
action-open-command-palette = Paleta de comandos
# Undo (1.5): machine-translated, pending native review.
action-undo = Deshacer
action-redo = Rehacer

# Groups (1.5): machine-translated, pending native review.
action-group-selected = Agrupar la selección
action-ungroup-selected = Desagrupar la selección
menu-group = Agrupar
menu-ungroup = Desagrupar
group-default-name = Grupo { $number }
group-copy-name = { $name } (copia)
toast-grouped = Agrupados como { $name }
toast-ungrouped = Grupos deshechos: { $count }
toast-nothing-to-ungroup = Nada de lo seleccionado está en un grupo
scene-groups-empty-hint = Selecciona varios personajes y pulsa Ctrl+G, o haz clic derecho sobre ellos y elige Agrupar.
scene-group-members = Personajes: { $count }
scene-group-select-tooltip = Seleccionar los personajes de este grupo
scene-group-rename = Renombrar el grupo
scene-group-show = Mostrar el grupo
scene-group-hide = Ocultar el grupo
scene-group-ungroup = Desagrupar: los personajes se quedan

# Arrange and snap (1.5): machine-translated, pending native review.
arrange-align-left = Alinear bordes izquierdos
arrange-align-center = Centrar en una línea vertical
arrange-align-right = Alinear bordes derechos
arrange-align-top = Alinear bordes superiores
arrange-align-middle = Centrar en una línea horizontal
arrange-align-bottom = Alinear bordes inferiores
arrange-distribute-horizontally = Repartir por igual a lo ancho
arrange-distribute-vertically = Repartir por igual a lo alto
scene-snap = Ajustar al arrastrar
scene-snap-tooltip = Los bordes y los centros se ajustan a los de las pantallas y de otros personajes. Mantén Alt al arrastrar para colocar libremente.
scene-bump = Los personajes chocan entre sí
scene-bump-tooltip = Con la gravedad activada (G), los personajes aterrizan en la cabeza de otros; los que caminan se dan la vuelta al encontrarse.
scene-greet = Los personajes se saludan al encontrarse
scene-greet-tooltip = Dos que se juntan — caminando, siguiéndose o aterrizando uno sobre otro — se detienen un momento y se saludan, con un corazón. No muy a menudo.
greet-hi = ¡Hola!
greet-hello = ¡Buenas!
greet-hey = ¡Eh, hola!

action-cycle-entity = Pasar al siguiente personaje
action-delete-selected = Eliminar el personaje seleccionado
action-nudge-up = Empujar la selección hacia arriba
action-nudge-down = Empujar la selección hacia abajo
action-nudge-left = Empujar la selección a la izquierda
action-nudge-right = Empujar la selección a la derecha
action-center-on-screen = Centrar la selección en pantalla
action-toggle-visible = Alternar visibilidad
action-toggle-gravity = Alternar gravedad
action-toggle-playback = Alternar reproducción/pausa
action-duplicate-selected = Duplicar la selección
action-copy-selected = Copiar la selección
action-cut-selected = Cortar la selección
action-paste = Pegar
action-reset-transform = Restablecer escala / opacidad
action-bring-forward = Traer la selección al frente
action-send-backward = Enviar la selección atrás
action-fps-up = Aumentar FPS
action-fps-down = Reducir FPS
action-opacity-up = Aumentar opacidad
action-opacity-down = Reducir opacidad
action-cycle-monitor = Cambiar el anclaje de monitor
action-show-entity-info = Mostrar información del personaje
action-show-help = Mostrar ayuda de teclado

# ── Accessibility section (D.3) — placeholder pending D.4 native-speaker audit
appearance-accessibility-header = Accesibilidad
appearance-accesskit-label = Generar actualizaciones del árbol AccessKit
# Screen readers (1.4): machine-translated, pending native review.
appearance-accesskit-hint = Permite que los lectores de pantalla (Orca y otros, mediante AT-SPI) lean y usen los paneles. Desactivado, un lector de pantalla ve una ventana vacía. Nota: mientras uno está en marcha, el texto que escribas en los paneles está en el bus AT-SPI, donde cualquier proceso de tu usuario puede leerlo.
# Machine-translated, pending native review.
appearance-accesskit-unsupported = Los lectores de pantalla aún no son compatibles con este sistema.
appearance-reduced-motion-label = Reducir movimiento
appearance-reduced-motion-hint = Omite las transiciones de la interfaz (deslizamiento del panel, fundidos, aparición de la paleta) y detiene el balanceo decorativo. Las animaciones que comunican estado siguen activas.
appearance-large-bubbles-label = Texto grande en los bocadillos
appearance-large-bubbles-hint = El texto de todos los bocadillos es más grande, para leerlo de lejos o con baja visión.
appearance-hover-startle-label = Sobresalto al pasar el cursor
appearance-hover-startle-hint = Las mascotas se apartan del puntero cuando se acerca. El seguimiento del cursor es solo en X11, así que en Wayland nativo solo reacciona en modo edición.
# Full-screen apps (1.5): machine-translated, pending native review.
appearance-fullscreen-label = Cuando una app está a pantalla completa
appearance-fullscreen-hint = Juegos, vídeos y presentaciones que llenan la pantalla. Funciona en X11 y en compositores Wayland como sway y Hyprland; en GNOME y KDE con Wayland, solo para las apps que funcionan con XWayland.
appearance-apps-label = Ocultar también con estas apps
appearance-apps-hint = Mientras una de ellas sea la ventana al frente —una videollamada, una emisión, un juego—, los personajes se ocultan, como con una app a pantalla completa. X11 y compositores Wayland como sway e Hyprland; en GNOME y KDE con Wayland, solo apps que se ejecutan con XWayland.
appearance-apps-remove = Quitar { $app }
appearance-apps-add = Añadir una app abierta…
appearance-apps-none-open = No hay abierta ninguna app que esta sesión pueda ver.
appearance-behind-windows-label = Detrás de tus ventanas
appearance-behind-windows-hint = Los personajes viven en el escritorio, bajo tus ventanas, en lugar de encima. El modo edición los trae al frente; si el botón ⚙ queda tapado, ábrelo con Ctrl+Shift+A o desde la bandeja.
appearance-tap-label = Se puede tocar a los personajes
appearance-tap-hint = También fuera del modo edición: un toque en un personaje le da un toquecito — un salto y una frase suya. Los clics en cualquier otro sitio siguen llegando a tus ventanas, y la app en la que escribes conserva el teclado.
appearance-effects-label = Pequeños efectos
appearance-effects-hint = Corazones al tocar un personaje, una nube de polvo donde uno aterriza con fuerza y zetas cuando se duermen. Ninguno con Reducir movimiento.
appearance-bedtime-label = Hora de dormir
appearance-bedtime-hint = Los personajes se quedan dormidos entre estas horas, esté quien esté al teclado; el modo edición los despierta mientras dura. La noche cuenta como descanso para los recordatorios.
appearance-moods-label = Ánimo según la hora del día
appearance-moods-hint = Más vivos por la mañana, más lentos por la tarde — el paso y la animación siguen el reloj — y, con la hora de dormir activada, un bostezo en la media hora anterior.
appearance-sounds-mute = Silenciar los sonidos
appearance-sounds-hint = Toques, recordatorios, el temporizador de concentración y los scripts. Silenciados, no se oye nada y el dispositivo de sonido queda cerrado.
appearance-sounds-volume = Volumen
appearance-sounds-bedtime = Silencio a la hora de dormir
appearance-sounds-bedtime-hint = Ningún sonido mientras los personajes duermen a la hora de dormir.
mood-yawn = *bosteza*
appearance-bedtime-from = de
appearance-bedtime-to = a
fullscreen-hide = Ocultar los personajes
fullscreen-pause = Pausarlos
fullscreen-ignore = Dejarlos seguir
# Pausing when away (1.5): machine-translated, pending native review.
appearance-away-label = Pausar cuando no estoy
appearance-away-hint = Sin ratón ni teclado durante este tiempo, los personajes se detienen, aterrizan y se quedan dormidos; tu siguiente acción los despierta.
away-never = Nunca
away-after-minutes = Tras { $minutes } min
appearance-away-unavailable = Esta sesión no puede saber cuándo no estás: hace falta GNOME, un servidor X real o un compositor Wayland con avisos de inactividad.
appearance-battery-label = Pausar con batería
appearance-battery-hint = Los personajes se quedan quietos mientras el equipo funciona con batería.
appearance-welcome-label = Saludarme cuando vuelva
appearance-welcome-hint = Tras un rato lejos del ordenador, un personaje te saluda al volver — «¡Buenos días!» después de la noche. No a la hora de dormir.
appearance-backup = Hacer copia de seguridad…
appearance-backup-hint = Todo en un archivo — ajustes, escenas guardadas y sus imágenes — para guardarlo o llevarlo a otro ordenador.
appearance-restore = Restaurar…
appearance-restore-hint = Recupera una copia: sus escenas se añaden junto a las tuyas y sus ajustes sustituyen a estos (los anteriores quedan en config.toml.before-restore).
welcome-back = ¡Hola de nuevo!
welcome-morning = ¡Buenos días!
# Start at login (1.5): machine-translated, pending native review.
appearance-autostart-label = Iniciar al entrar
appearance-autostart-hint = Inicia animaEngine cuando entras en el escritorio.
appearance-autostart-reason = Para iniciar animaEngine al entrar en la sesión.
appearance-autostart-failed = No se pudo cambiar: { $error }
# Named scenes (1.5): machine-translated, pending native review.
scene-scenes-header = Escenas
scene-scenes-hint = Guarda lo que hay en pantalla con un nombre y vuelve a ello aquí, desde la paleta de comandos o con «Escena siguiente» en la bandeja.
scene-scene-name-hint = Nombre de la escena
scene-scene-save-new = Guardar como escena
scene-scene-load-tooltip = Cambiar a esta escena: Deshacer vuelve atrás
scene-scene-save-over = Guardar lo que hay en pantalla como { $name }
scene-scene-share = Compartir { $name } como un solo archivo, con sus imágenes
scene-import = Importar…
scene-import-tooltip = Abre una escena que alguien compartió contigo (un archivo .animascene). También puedes soltar el archivo sobre la superposición.
scene-scene-delete = Eliminar escena
scene-scene-delete-confirm = ¿Eliminar?
scene-scene-delete-cancel = Conservar
scene-scene-save-failed = No guardada: { $error }
toast-scene-loaded = Escena: { $name }
toast-scene-failed = Escena no cargada: { $error }
toast-scene-shared = Escena lista para compartir: { $path }
toast-scene-shared-scripts = Los scripts de comportamiento no se comparten; personajes quietos en el archivo: { $count }
toast-scene-share-failed = No se pudo compartir: { $error }
toast-gif-saved = GIF guardado: { $path }
toast-gif-failed = No se pudo guardar el GIF: { $error }
toast-character-shared = Personaje listo para compartir: { $path }
toast-character-shared-script = Su script de comportamiento no se comparte: en el archivo se queda quieto.
toast-character-added = { $name } añadido
toast-characters-added = Personajes añadidos: { $count }
toast-backup-saved = Copia guardada: { $path }
toast-backup-failed = Falló la copia o la restauración: { $error }
toast-restored = Restaurado: los ajustes, y escenas añadidas: { $count }
toast-scene-imported = Escena importada: { $name }
toast-scene-import-failed = No se pudo importar: { $error }
toast-no-scenes = Aún no hay escenas guardadas: guarda una en la pestaña Escena.
palette-load-scene = Cambiar a la escena: { $name }
palette-snooze-minutes = Dormitar { $minutes } minutos
palette-focus-start = Iniciar el temporizador de concentración
palette-focus-stop = Detener el temporizador de concentración
palette-snooze-morning = Dormitar hasta la mañana
tray-next-scene = Escena siguiente
tray-show-overlay = Mostrar el overlay
tray-hide-overlay = Ocultar el overlay
tray-snoozing-until = Dormitando hasta las { $until }
tray-wake = Despertar ahora
tray-quit = Salir
palette-mute = Silenciar los sonidos
palette-unmute = Activar los sonidos
# Reminders (1.5): machine-translated, pending native review.
scene-reminders-header = Recordatorios
scene-reminders-hint = Un personaje lo dice de vez en cuando, en un bocadillo. El tiempo lejos del ordenador cuenta como descanso y lo reinicia.
scene-focus-header = Temporizador de concentración
scene-focus-hint = Trabaja un rato y un personaje te dirá cuándo toca un descanso, y cuándo volver. Mientras tanto, los recordatorios esperan. También desde la paleta de comandos o con «Iniciar el temporizador de concentración» en la bandeja.
scene-focus-work = Trabajo
scene-focus-break = Descanso
scene-focus-start = Iniciar
scene-focus-stop = Detener
scene-focus-working = Concentración hasta las { $until }
scene-focus-on-break = En descanso hasta las { $until }
reminder-sound-label = Sonido con los recordatorios y el temporizador
reminder-sound-none = Ninguno
reminder-sound-speaker = El sonido al tocar de quien habla
reminder-notify-label = También como notificaciones del escritorio
reminder-notify-hint = Los recordatorios y el temporizador también llegan a las notificaciones del escritorio — al momento si los personajes no se ven, en lugar de esperarlos.
focus-said-started = Hora de concentrarse — hasta las { $until }.
focus-said-break = ¡Descanso! Volvemos a las { $until }.
focus-said-back = Se acabó el descanso — a seguir hasta las { $until }.
scene-reminder-every = cada { $minutes } min
scene-reminder-anyone = cualquier personaje
scene-reminder-delete = Eliminar recordatorio
scene-reminder-text-hint = Qué decir
scene-reminder-minutes = Cada (minutos)
scene-reminder-timed = A una hora fija
scene-reminder-at = a las
scene-reminder-on-date = en una fecha
scene-reminder-days = Días
scene-reminder-date = Fecha
scene-reminder-who = Quién lo dice
scene-reminder-add = Añadir recordatorio
reminder-break = ¡Hora de un descanso corto!
reminder-water = Bebe un vaso de agua.
reminder-stretch = Levántate y estírate.
# Scenes by time of day (1.5): machine-translated, pending native review.
scene-schedule-header = Cambiar automáticamente
scene-schedule-hint = Cambia a una escena guardada a horas fijas: Trabajo a las 09:00 entre semana, por ejemplo. No mientras editas: entonces espera.
scene-schedule-needs-scene = Guarda una escena arriba para cambiar a ella según un horario.
scene-schedule-row = { $time } · { $days } → { $scene }
scene-schedule-every-day = todos los días
scene-schedule-weekdays = entre semana
scene-schedule-weekends = fines de semana
scene-schedule-at = A las
scene-schedule-minute = Minuto
scene-schedule-days = Días
scene-schedule-scene = Escena
scene-schedule-add = Añadir
scene-schedule-delete = Eliminar regla

# ── Warning banners (D.5) — placeholder pending native-speaker audit
warning-global-hotkeys-unavailable = No se pudieron registrar los atajos globales (típico en sesiones Wayland nativas). El menú de bandeja y el botón ⚙ siguen funcionando.
warning-hot-reload-disconnected = El proceso de recarga en caliente se detuvo inesperadamente; los cambios de configuración pendientes no se aplicarán hasta reiniciar la app.
# Machine-translated, pending native review.
warning-config-unreadable = No se pudo leer tu archivo de configuración, así que se cargó la escena predeterminada. El original se conserva al lado como config.toml.bak-corrupt.
action-toggle-perf-overlay = Alternar overlay de rendimiento

# ── What's new (D.7) — placeholder pending native-speaker audit
# 1.4 highlights: machine-translated, pending native review.
whats-new-header = Novedades de 1.5
whats-new-alive = Los personajes cobran vida: tócalos y deja que se saluden, jueguen a pillar o se sigan unos a otros.
whats-new-your-day = Recordatorios a una hora fija, un temporizador de concentración, hora de dormir y dormitar, y un saludo cuando vuelves.
whats-new-editing = Deshaz con Ctrl+Z, selecciona varios personajes a la vez, agrúpalos y alinéalos.
whats-new-files = Las escenas tienen nombre, imagen y horario; comparte una escena o un solo personaje, o haz una copia de seguridad de todo en un solo archivo.
onboarding-keybindings = Haz clic en un atajo para quitarlo; pulsa una combinación para grabar uno nuevo.
onboarding-perf-overlay = Pulsa Ctrl+Shift+` para abrir el overlay de rendimiento en vivo.
appearance-reset-onboarding = Restablecer las pistas de bienvenida

scene-empty-action-browse-presets = Explorar presets
library-empty-action-copy-path = Copiar la ruta al portapapeles

appearance-reset-onboarding-hint = Recupera las pistas descartadas y el panel «Novedades».

# ── Portal shortcuts (T.3) ────────────────────────────────────────────
portal-denied-x11-fallback-toast = Se denegó el permiso de atajos — se usarán atajos X11. Reintente desde la pestaña Atajos.
portal-denied-native-toast = Se denegó el permiso de atajos — el menú de bandeja y los atajos del compositor siguen funcionando.

# ── Keybindings backend status (T.4) ─────────────────────────────────
keybindings-backend-label = Atajos globales mediante:
keybindings-backend-tooltip = Qué mecanismo entrega los tres atajos globales (editar, ocultar, pausar) mientras otras apps tienen el foco. Se resuelve al arrancar; los atajos internos no se ven afectados.
keybindings-portal-restart-hint = Los cambios de atajos se aplican en el próximo arranque (el escritorio recuerda su aprobación).

# ── Monitor hotplug (T.9) ─────────────────────────────────────────────
monitor-unplugged-toast = Monitor { $name } desconectado — { $n } personajes anclados ahora siguen su posición.
monitor-plugged-toast = Monitor { $name } conectado.

# ── Shimeji import (U.4) ──────────────────────────────────────────────
library-import-shimeji-header = Importar paquete Shimeji
library-import-shimeji-hint = Arrastra la carpeta del paquete al overlay o pega su ruta aquí. Los sprites se copian a la biblioteca.
library-import-shimeji-button = Importar
shimeji-imported-toast = { $name } importado ({ $n } partes omitidas — vea el log)
shimeji-import-failed-toast = Importación fallida: { $reason }
shimeji-no-library-toast = No hay carpeta de biblioteca — cree primero { $path }.
crash-report-found-toast = La sesión anterior se cerró inesperadamente. Se guardó un informe en { $path } — adjúntelo a un issue de GitHub.

# ── Group composition hint (C.9) ──────────────────────────────────────
inspector-group-hint = Compuesto por el grupo { $group }: { $transform }
# Multiple selection (1.5): machine-translated, pending native review.
inspector-multi-selected = { $count } seleccionados. Este panel edita { $name }; arrastrar, los atajos y el menú contextual actúan sobre todos.

# ── App-layer toasts (V.6 — F1 closure) ──────────────────────────────
toast-config-saved = Configuración guardada
# Hot-reload toasts: machine-translated, pending native review.
toast-config-reloaded = Configuración recargada desde el disco
toast-config-reload-failed = Configuración no recargada: el archivo no es válido o aún se está escribiendo. Se mantiene la escena actual.
toast-config-reload-discarded = Configuración no recargada: editaste la escena mientras se cargaba.
toast-save-failed = El guardado falló: { $error }
toast-rejected = Rechazado: { $reason }
toast-added = { $name } añadido
toast-load-failed = La carga falló: { $error }
toast-entity-load-failed = { $name }: { $error }
toast-theme-switched = Tema: { $theme }
toast-preset-entry-failed = No se pudo añadir la entrada del preset: { $error }
toast-preset-loaded = Preset cargado: { $name }
toast-duplicated = { $name } duplicado
# Multiple selection (1.5): machine-translated, pending native review.
toast-duplicated-many = Personajes duplicados: { $count }
toast-duplicate-failed = La duplicación falló: { $error }
toast-deleted = { $name } eliminado
# Multiple selection (1.5): machine-translated, pending native review.
toast-deleted-many = Personajes eliminados: { $count }
toast-copied = { $name } copiado
toast-copied-many = Personajes copiados: { $count }
toast-cut = { $name } cortado
toast-cut-many = Personajes cortados: { $count }
toast-pasted = { $name } pegado
toast-pasted-many = Personajes pegados: { $count }
toast-paste-failed = No se pudo pegar: { $error }
toast-nothing-to-paste = Nada que pegar: copia primero un personaje o una imagen
pasted-image-name = Imagen pegada
toast-playback-resumed = Reproducción reanudada
toast-playback-paused = Reproducción en pausa
# Undo (1.5): machine-translated, pending native review.
toast-undone = Deshecho
toast-redone = Rehecho
toast-nothing-to-undo = No hay nada que deshacer
toast-nothing-to-redo = No hay nada que rehacer
inspector-wander-box = Zona de paseo
toast-perf-snapshot = Captura de rendimiento: { $path }
toast-perf-snapshot-failed = La captura falló: { $error }
