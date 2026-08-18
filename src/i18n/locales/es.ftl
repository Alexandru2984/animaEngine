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
inspector-visible = Visible
inspector-gravity = Gravedad
inspector-scale = Escala
inspector-behavior-speed = Velocidad
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
behavior-wander = Vagar acotado
behavior-bounce = Rebote
behavior-bounce-axis = Eje
behavior-bounce-horizontal = Horizontal
behavior-bounce-vertical = Vertical
behavior-bounce-both = Ambos (círculo)

scene-empty-headline = Escena vacía
scene-empty-hint = Arrastra un PNG / GIF / WebP / MP4 sobre el overlay — o prueba un preset abajo.
scene-drop-hint = Arrastra un PNG / GIF / WebP sobre el overlay para añadir una entidad.
scene-presets-header = Presets
scene-preset-append = Añadir
scene-preset-replace = Reemplazar
scene-preset-replace-tooltip = Limpia la escena actual antes de añadir

monitor-section-header = Monitores
monitor-mode-label = Distribución
monitor-mode-per-monitor = Por monitor
monitor-mode-span = Extender en todos los monitores
monitor-mode-single = Un solo monitor
scene-window-awareness = Aterrizar en ventanas (X11)
scene-window-awareness-tooltip = Los personajes con física activa aterrizan y caminan por el borde superior de sus ventanas abiertas. Solo sesiones X11 — Wayland no expone posiciones de ventanas, así que ahí no hace nada.
monitor-pin-label = Fijar al monitor
monitor-pin-auto = Auto (sigue la posición)
monitor-pinned-toast = Entidad fijada a { $name }
monitor-pin-cleared-toast = La entidad sigue ahora su posición
monitor-no-monitors-detected = No se detectaron monitores

appearance-theme-header = Tema
appearance-theme-label = Tema
appearance-language-header = Idioma
theme-dark = Oscuro
theme-light = Claro
theme-dark-hc = Oscuro · Alto contraste
theme-light-hc = Claro · Alto contraste

onboarding-tabs = Los ajustes se reparten en tres pestañas — Inspector, Escena, Apariencia.
onboarding-quick-toggles = Consejo: V alterna la visibilidad, G alterna la gravedad — sin abrir este panel.
onboarding-theme = Los temas se aplican al instante — no hace falta reiniciar.
onboarding-coach-step1 = ¡Bienvenido! Sus personajes viven en el escritorio. Haga clic en el botón de engranaje de la esquina superior derecha para entrar en el modo edición.
onboarding-coach-step2 = Suelte un PNG, GIF, WebP o MP4 en cualquier parte de la pantalla para añadirlo como personaje. El panel lateral edita todo lo que seleccione.
onboarding-coach-step3 = Ctrl+K abre la paleta de comandos. Ctrl+Shift+A alterna el modo edición desde cualquier sitio, Ctrl+Shift+H oculta el overlay.
onboarding-coach-next = Siguiente
onboarding-coach-skip = Saltar el tour
onboarding-coach-done = Entendido
palette-replace-row = Reemplazar la escena con: { $preset }
palette-append-row = Añadir preset: { $preset }
palette-footer-hint = Esc cierra · Ctrl+K alterna · ↑↓ + Enter elige
onboarding-dismiss = Cerrar

menu-duplicate = Duplicar
menu-reset-transform = Restablecer transformación
menu-toggle-gravity = Alternar gravedad
menu-bring-forward = Traer al frente
menu-send-backward = Enviar al fondo
menu-delete = Eliminar

toggle-enter-edit = Entrar al modo edición
toggle-exit-edit = Salir del modo edición

palette-search-placeholder = Escribe para buscar temas / presets…
palette-close-hint = Esc para cerrar · Ctrl+K para alternar
palette-switch-theme = Cambiar al tema { $theme }
palette-apply-preset = Aplicar preset: { $preset }

settings-tab-library = Biblioteca

# Asset library tab
library-empty-headline = Sin activos indexados
library-empty-hint = Arrastra archivos a ~/.local/share/animaEngine/assets/ o configura ANIMA_ASSETS_DIR.
library-no-asset-root = Directorio de assets no encontrado. Crea uno en ~/.local/share/animaEngine/assets/
library-search-placeholder = Buscar activos…
library-add-to-scene = Añadir a la escena
library-sort-recent = Recientes
library-sort-name = Nombre
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
keybindings-recording = Pulse una combinación… (Esc cancela)
keybindings-conflict = Entra en conflicto con { $action }
keybindings-reset-all = Restablecer todo a los valores predeterminados
keybindings-help = Los atajos personalizados se guardan en config.toml

# ── Action labels (D.1.7) — placeholder pending D.4 native-speaker audit
action-toggle-edit-mode = Alternar modo edición
action-hide-overlay = Ocultar / mostrar el overlay
action-pause-all = Pausar todas las animaciones
action-quit-with-save = Salir (guardando la configuración)
action-save-now = Guardar la configuración ahora
action-open-command-palette = Paleta de comandos
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
appearance-accesskit-hint = Alimenta lectores de pantalla AT-SPI (Orca, etc.). Déjelo activado salvo que quiera reducir recursos o su escritorio no tenga bus AT-SPI. Nota: el texto que escriba en los paneles también aparece en el bus AT-SPI, donde cualquier proceso de su usuario puede leerlo.
appearance-reduced-motion-label = Reducir movimiento
appearance-reduced-motion-hint = Omite las transiciones de la interfaz (deslizamiento del panel, fundidos, aparición de la paleta) y detiene el balanceo decorativo. Las animaciones que comunican estado siguen activas.
appearance-hover-startle-label = Sobresalto al pasar el cursor
appearance-hover-startle-hint = Las mascotas se apartan del puntero cuando se acerca y luego vuelven. El seguimiento del cursor es solo en X11, así que en Wayland nativo solo reacciona en modo edición.

# ── Warning banners (D.5) — placeholder pending native-speaker audit
warning-global-hotkeys-unavailable = No se pudieron registrar los atajos globales (típico en sesiones Wayland nativas). El menú de bandeja y el botón ⚙ siguen funcionando.
warning-hot-reload-disconnected = El proceso de recarga en caliente se detuvo inesperadamente; los cambios de configuración pendientes no se aplicarán hasta reiniciar la app.
action-toggle-perf-overlay = Alternar overlay de rendimiento

# ── What's new (D.7) — placeholder pending native-speaker audit
whats-new-header = Novedades de la 0.4
whats-new-keybindings = Atajos de teclado reasignables — abra la nueva pestaña Atajos.
whats-new-collapse-state = Las secciones del Inspector recuerdan su estado abierto/cerrado entre sesiones.
whats-new-error-banners = Las superficies de error (antes silenciosas) ahora muestran toasts o banners — las verá.
whats-new-accessibility-toggle = AccessKit puede desactivarse en Apariencia → Accesibilidad.
onboarding-keybindings = Haga clic en un atajo para quitarlo; pulse una combinación para grabar uno nuevo.
onboarding-perf-overlay = Pulse Ctrl+Shift+` para abrir el overlay de rendimiento en vivo.
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
library-import-shimeji-hint = Arrastre la carpeta del paquete al overlay o pegue su ruta aquí. Los sprites se copian a la biblioteca.
library-import-shimeji-button = Importar
shimeji-imported-toast = { $name } importado ({ $n } partes omitidas — vea el log)
shimeji-import-failed-toast = Importación fallida: { $reason }
shimeji-no-library-toast = No hay carpeta de biblioteca — cree primero ~/.local/share/animaEngine/assets/.
crash-report-found-toast = La sesión anterior se cerró inesperadamente. Se guardó un informe en { $path } — adjúntelo a un issue de GitHub.

# ── Group composition hint (C.9) ──────────────────────────────────────
inspector-group-hint = Compuesto por el grupo { $group }: { $transform }

# ── App-layer toasts (V.6 — F1 closure) ──────────────────────────────
toast-config-saved = Configuración guardada
toast-save-failed = El guardado falló: { $error }
toast-rejected = Rechazado: { $reason }
toast-added = { $name } añadido
toast-load-failed = La carga falló: { $error }
toast-entity-load-failed = { $name }: { $error }
toast-theme-switched = Tema: { $theme }
toast-preset-entry-failed = No se pudo añadir la entrada del preset: { $error }
toast-preset-loaded = Preset cargado: { $name }
toast-duplicated = { $name } duplicado
toast-duplicate-failed = La duplicación falló: { $error }
toast-deleted = { $name } eliminado
toast-playback-resumed = Reproducción reanudada
toast-playback-paused = Reproducción en pausa
inspector-wander-box = Zona de paseo
toast-perf-snapshot = Captura de rendimiento: { $path }
toast-perf-snapshot-failed = La captura falló: { $error }
