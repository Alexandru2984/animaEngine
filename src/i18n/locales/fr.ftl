# Français — traduction de base. Relecture par locuteur natif à faire.

app-name = animaEngine

settings-tab-inspector = Inspecteur
settings-tab-scene = Scène
settings-tab-appearance = Apparence
entity-count-zero = Aucune entité
entity-count-singular = { $n } entité
entity-count-plural = { $n } entités

inspector-section-position = Position
inspector-section-appearance = Apparence
inspector-section-animation = Animation
animation-easing-label = Interpolation
easing-linear = Linéaire
easing-ease-in-quad = Entrée douce
easing-ease-out-quad = Sortie douce
easing-ease-in-out-quad = Entrée/sortie douce
easing-sine = Sinus
easing-bounce-out = Rebond
inspector-section-behavior = Comportement
inspector-visible = Visible
inspector-gravity = Gravité
inspector-scale = Échelle
inspector-behavior-speed = Vitesse
inspector-behavior-comfort = Distance de confort
inspector-behavior-amplitude = Amplitude
inspector-behavior-period = Période
inspector-double-click-reset-hint = Double-cliquez pour rétablir la valeur par défaut.
inspector-opacity = Opacité
inspector-fps = FPS
inspector-playing = Lecture
inspector-x = X
inspector-y = Y
inspector-z-index = z-index
inspector-nothing-selected-headline = Rien de sélectionné
inspector-nothing-selected-hint = Cliquez sur une entité dans l'onglet Scène, ou appuyez sur Tab pour les parcourir.

behavior-idle = Au repos
behavior-walk = Se promener
behavior-follow = Suivre le curseur
behavior-wander = Errance bornée
behavior-bounce = Rebond
behavior-bounce-axis = Axe
behavior-bounce-horizontal = Horizontal
behavior-bounce-vertical = Vertical
behavior-bounce-both = Les deux (cercle)

scene-empty-headline = Scène vide
scene-empty-hint = Déposez un PNG / GIF / WebP / MP4 sur l'overlay — ou essayez un preset ci-dessous.
scene-drop-hint = Déposez un PNG / GIF / WebP sur l'overlay pour ajouter une entité.
scene-presets-header = Presets
scene-preset-append = Ajouter
scene-preset-replace = Remplacer
scene-preset-replace-tooltip = Efface la scène actuelle avant d'ajouter

monitor-section-header = Écrans
monitor-mode-label = Répartition
monitor-mode-per-monitor = Un par écran
monitor-mode-span = Étendre sur tous les écrans
monitor-mode-single = Un seul écran
scene-window-awareness = Atterrir sur les fenêtres (X11)
scene-window-awareness-tooltip = Les personnages avec physique active atterrissent et marchent sur le bord supérieur de vos fenêtres ouvertes. Sessions X11 uniquement — Wayland n’expose pas la position des fenêtres, donc cela n’a aucun effet là-bas.
monitor-pin-label = Épingler à l'écran
monitor-pin-auto = Auto (suit la position)
monitor-pinned-toast = Entité épinglée à { $name }
monitor-pin-cleared-toast = L'entité suit maintenant sa position
monitor-no-monitors-detected = Aucun écran détecté

appearance-theme-header = Thème
appearance-theme-label = Thème
appearance-language-header = Langue
theme-dark = Sombre
theme-light = Clair
theme-dark-hc = Sombre · Contraste élevé
theme-light-hc = Clair · Contraste élevé

onboarding-tabs = Les réglages se répartissent sur trois onglets — Inspecteur, Scène, Apparence.
onboarding-quick-toggles = Astuce : V bascule la visibilité, G la gravité — sans ouvrir ce panneau.
onboarding-theme = Les thèmes s'appliquent instantanément — pas de redémarrage.
onboarding-coach-step1 = Bienvenue ! Vos personnages vivent sur le bureau. Cliquez sur le bouton engrenage en haut à droite pour entrer en mode édition.
onboarding-coach-step2 = Déposez un PNG, GIF, WebP ou MP4 n’importe où sur l’écran pour l’ajouter comme personnage. Le panneau latéral édite tout ce que vous sélectionnez.
onboarding-coach-step3 = Ctrl+K ouvre la palette de commandes. Ctrl+Shift+A bascule le mode édition de partout, Ctrl+Shift+H masque l’overlay.
onboarding-coach-next = Suivant
onboarding-coach-skip = Passer la visite
onboarding-coach-done = Compris
palette-replace-row = Remplacer la scène par : { $preset }
palette-append-row = Ajouter le preset : { $preset }
palette-footer-hint = Échap ferme · Ctrl+K bascule · ↑↓ + Entrée choisit
onboarding-dismiss = Fermer

menu-duplicate = Dupliquer
menu-reset-transform = Réinitialiser la transformation
menu-toggle-gravity = Basculer la gravité
menu-bring-forward = Mettre au premier plan
menu-send-backward = Renvoyer à l'arrière
menu-delete = Supprimer

toggle-enter-edit = Entrer en mode édition
toggle-exit-edit = Quitter le mode édition

palette-search-placeholder = Rechercher des thèmes / presets…
palette-close-hint = Esc pour fermer · Ctrl+K pour basculer
palette-switch-theme = Passer au thème { $theme }
palette-apply-preset = Appliquer le preset : { $preset }

settings-tab-library = Bibliothèque

# Asset library tab
library-empty-headline = Aucun asset indexé
library-empty-hint = Déposez des fichiers dans ~/.local/share/animaEngine/assets/ ou définissez ANIMA_ASSETS_DIR.
library-no-asset-root = Aucun dossier d'assets trouvé. Créez-en un dans ~/.local/share/animaEngine/assets/
library-search-placeholder = Rechercher des assets…
library-add-to-scene = Ajouter à la scène
library-sort-recent = Récents
library-sort-name = Nom
library-kind-image = Image
library-kind-animated = Animé
library-kind-video = Vidéo
library-asset-added-toast = { $name } ajouté à la scène
library-asset-add-failed-toast = Impossible d'ajouter { $name }
library-count = { $n } assets indexés

# ── Keybindings tab (D.1) — placeholder pending D.4 native-speaker audit
settings-tab-keybindings = Raccourcis
keybindings-unbound = (non assigné)
keybindings-add = Ajouter
keybindings-recording = Appuyez sur une combinaison… (Échap pour annuler)
keybindings-conflict = En conflit avec { $action }
keybindings-reset-all = Tout réinitialiser aux valeurs par défaut
keybindings-help = Les raccourcis personnalisés sont conservés dans config.toml

# ── Action labels (D.1.7) — placeholder pending D.4 native-speaker audit
action-toggle-edit-mode = Basculer le mode édition
action-hide-overlay = Masquer / afficher l’overlay
action-pause-all = Mettre toutes les animations en pause
action-quit-with-save = Quitter (enregistrer la configuration)
action-save-now = Enregistrer la configuration maintenant
action-open-command-palette = Palette de commandes
action-cycle-entity = Passer au personnage suivant
action-delete-selected = Supprimer le personnage sélectionné
action-nudge-up = Déplacer la sélection vers le haut
action-nudge-down = Déplacer la sélection vers le bas
action-nudge-left = Déplacer la sélection vers la gauche
action-nudge-right = Déplacer la sélection vers la droite
action-center-on-screen = Centrer la sélection à l’écran
action-toggle-visible = Basculer la visibilité
action-toggle-gravity = Basculer la gravité
action-toggle-playback = Basculer lecture/pause
action-duplicate-selected = Dupliquer la sélection
action-reset-transform = Réinitialiser échelle / opacité
action-bring-forward = Avancer la sélection
action-send-backward = Reculer la sélection
action-fps-up = Augmenter les FPS
action-fps-down = Diminuer les FPS
action-opacity-up = Augmenter l’opacité
action-opacity-down = Diminuer l’opacité
action-cycle-monitor = Changer l’épinglage d’écran
action-show-entity-info = Afficher les infos du personnage
action-show-help = Afficher l’aide clavier

# ── Accessibility section (D.3) — placeholder pending D.4 native-speaker audit
appearance-accessibility-header = Accessibilité
appearance-accesskit-label = Générer les mises à jour de l’arbre AccessKit
appearance-accesskit-hint = Alimente les lecteurs d’écran AT-SPI (Orca, etc.). Laissez activé, sauf si vous voulez alléger l’empreinte ou si votre bureau n’a pas de bus AT-SPI. Remarque : le texte saisi dans les panneaux apparaît aussi sur le bus AT-SPI, où tout processus de votre utilisateur peut le lire.
appearance-reduced-motion-label = Réduire les animations
appearance-reduced-motion-hint = Ignore les transitions de l’interface (glissement du panneau, fondus, apparition de la palette) et arrête le balancement décoratif. Les animations qui portent un état restent actives.

# ── Warning banners (D.5) — placeholder pending native-speaker audit
warning-global-hotkeys-unavailable = Les raccourcis globaux n’ont pas pu être enregistrés (typique d’une session Wayland native). Le menu de la zone de notification et le bouton ⚙ fonctionnent toujours.
warning-hot-reload-disconnected = Le processus de rechargement à chaud s’est arrêté de façon inattendue ; les modifications de configuration en cours ne s’appliqueront qu’après un redémarrage.
action-toggle-perf-overlay = Basculer l’overlay de performance

# ── What's new (D.7) — placeholder pending native-speaker audit
whats-new-header = Nouveautés de la 0.4
whats-new-keybindings = Raccourcis clavier réassignables — ouvrez le nouvel onglet Raccourcis.
whats-new-collapse-state = Les sections de l’Inspecteur retiennent leur état ouvert/fermé entre les sessions.
whats-new-error-banners = Les erreurs (silencieuses auparavant) affichent désormais des toasts ou des bannières — vous les verrez.
whats-new-accessibility-toggle = AccessKit peut être désactivé dans Apparence → Accessibilité.
onboarding-keybindings = Cliquez sur un raccourci pour le retirer ; appuyez sur une combinaison pour en enregistrer un nouveau.
onboarding-perf-overlay = Appuyez sur Ctrl+Shift+` pour ouvrir l’overlay de performance en direct.
appearance-reset-onboarding = Réinitialiser les astuces de démarrage

scene-empty-action-browse-presets = Parcourir les presets
library-empty-action-copy-path = Copier le chemin dans le presse-papiers

appearance-reset-onboarding-hint = Fait revenir les astuces masquées et le panneau « Nouveautés ».

# ── Portal shortcuts (T.3) ────────────────────────────────────────────
portal-denied-x11-fallback-toast = Permission de raccourcis refusée — les raccourcis X11 prennent le relais. Réessayez depuis l’onglet Raccourcis.
portal-denied-native-toast = Permission de raccourcis refusée — le menu de la zone de notification et les raccourcis du compositeur fonctionnent toujours.

# ── Keybindings backend status (T.4) ─────────────────────────────────
keybindings-backend-label = Raccourcis globaux via :
keybindings-backend-tooltip = Quel mécanisme délivre les trois raccourcis globaux (édition, masquage, pause) quand d’autres applications ont le focus. Résolu au démarrage ; les raccourcis internes ne sont pas concernés.
keybindings-portal-restart-hint = Les changements de déclencheurs s’appliquent au prochain lancement (le bureau retient votre accord).

# ── Monitor hotplug (T.9) ─────────────────────────────────────────────
monitor-unplugged-toast = Écran { $name } déconnecté — { $n } personnages épinglés suivent désormais leur position.
monitor-plugged-toast = Écran { $name } connecté.

# ── Shimeji import (U.4) ──────────────────────────────────────────────
library-import-shimeji-header = Importer un pack Shimeji
library-import-shimeji-hint = Déposez le dossier du pack sur l’overlay ou collez son chemin ici. Les sprites sont copiés dans la bibliothèque.
library-import-shimeji-button = Importer
shimeji-imported-toast = { $name } importé ({ $n } éléments ignorés — voir le journal)
shimeji-import-failed-toast = Échec de l’import : { $reason }
shimeji-no-library-toast = Aucun dossier de bibliothèque — créez d’abord ~/.local/share/animaEngine/assets/.
crash-report-found-toast = La session précédente a planté. Un rapport a été enregistré dans { $path } — joignez-le à un ticket GitHub.

# ── Group composition hint (C.9) ──────────────────────────────────────
inspector-group-hint = Composé par le groupe { $group } : { $transform }

# ── App-layer toasts (V.6 — F1 closure) ──────────────────────────────
toast-config-saved = Configuration enregistrée
toast-save-failed = Échec de l’enregistrement : { $error }
toast-rejected = Rejeté : { $reason }
toast-added = { $name } ajouté
toast-load-failed = Échec du chargement : { $error }
toast-entity-load-failed = { $name } : { $error }
toast-theme-switched = Thème : { $theme }
toast-preset-entry-failed = Impossible d’ajouter l’entrée du preset : { $error }
toast-preset-loaded = Preset chargé : { $name }
toast-duplicated = { $name } dupliqué
toast-duplicate-failed = Échec de la duplication : { $error }
toast-deleted = { $name } supprimé
toast-playback-resumed = Lecture reprise
toast-playback-paused = Lecture en pause
inspector-wander-box = Zone d’errance
toast-perf-snapshot = Instantané de performance : { $path }
toast-perf-snapshot-failed = Échec de l’instantané : { $error }
