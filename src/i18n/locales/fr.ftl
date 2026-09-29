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
behavior-script = Script
behavior-script-path-label = Chemin
behavior-script-path-hint = Relatif à votre bibliothèque de ressources — { $path }
behavior-script-params = Paramètres
behavior-script-param-name = nom
behavior-script-add-param = Ajouter
behavior-script-remove-param = Supprimer ce paramètre
script-failed-toast = Le script de comportement { $script } a échoué : { $error }

scene-empty-headline = Scène vide
scene-empty-hint = Déposez un PNG / GIF / WebP / MP4 sur l'overlay — ou essayez un preset ci-dessous.
scene-drop-hint = Déposez un PNG / GIF / WebP sur l'overlay pour ajouter une entité.
# "Add file…" (1.4): machine-translated, pending native review.
scene-add-file = Ajouter un fichier…
scene-add-file-tooltip = Choisissez des images ou des vidéos à ajouter comme personnages.
file-chooser-title = Ajouter des personnages
file-chooser-filter = Images et vidéos
file-chooser-unavailable-toast = Impossible d'ouvrir le sélecteur de fichiers. Il nécessite xdg-desktop-portal.
scene-presets-header = Presets
scene-groups-header = Groupes
scene-preset-append = Ajouter
scene-preset-replace = Remplacer
scene-preset-replace-tooltip = Efface la scène actuelle avant d'ajouter

monitor-section-header = Écrans
monitor-mode-label = Répartition
monitor-mode-per-monitor = Un par écran
monitor-mode-span = Étendre sur tous les écrans
monitor-mode-span-unsupported = Sur ce backend, une seule superposition ne peut pas couvrir plusieurs écrans — utilisez « par écran ».
monitor-mode-single = Un seul écran
scene-window-awareness = Atterrir sur les fenêtres (X11)
scene-window-awareness-tooltip = Les personnages avec physique active atterrissent et marchent sur le bord supérieur de vos fenêtres ouvertes. Sessions X11 uniquement — Wayland n’expose pas la position des fenêtres, donc cela n’a aucun effet là-bas.
scene-window-awareness-unavailable = Indisponible sur Wayland natif — aucun protocole n'expose la position des fenêtres.
monitor-pin-label = Épingler à l'écran
monitor-pin-auto = Auto (suit la position)
monitor-pinned-toast = Entité épinglée à { $name }
monitor-pin-cleared-toast = L'entité suit maintenant sa position
monitor-no-monitors-detected = Aucun écran détecté

appearance-theme-header = Thème
appearance-theme-label = Thème
appearance-language-header = Langue
appearance-language-no-font = Aucune police n'est installée pour cette écriture — installez d'abord un paquet Noto CJK.
theme-dark = Sombre
theme-light = Clair
theme-dark-hc = Sombre · Contraste élevé
theme-light-hc = Clair · Contraste élevé

onboarding-tabs = Les réglages tiennent en cinq onglets — Inspecteur, Scène, Bibliothèque, Apparence, Raccourcis.
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

# Palette placeholder (1.4): machine-translated, pending native review.
palette-search-placeholder = Rechercher des actions, thèmes et presets…
palette-switch-theme = Passer au thème { $theme }

settings-tab-library = Bibliothèque

# Asset library tab
library-empty-headline = Aucun asset indexé
library-empty-hint = Déposez des fichiers dans { $path } ou définissez ANIMA_ASSETS_DIR.
library-no-asset-root = Aucun dossier d'assets trouvé. Créez-en un dans { $path }
library-search-placeholder = Rechercher des assets…
library-add-to-scene = Ajouter à la scène
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
keybindings-reset-one = Réinitialiser par défaut
keybindings-remove-chord = Supprimer ce raccourci
keybindings-help = Les raccourcis personnalisés sont conservés dans config.toml
# Modifier names as this language's keyboards print them. Display only:
# config.toml always stores the English names. Kept in English unless
# the convention is certain; see R39 in docs/runtime-findings.md.
key-mod-ctrl = Ctrl
key-mod-shift = Shift
key-mod-alt = Alt
key-mod-super = Super

# ── Action labels (D.1.7) — placeholder pending D.4 native-speaker audit
action-toggle-edit-mode = Basculer le mode édition
action-hide-overlay = Masquer / afficher l’overlay
action-pause-all = Mettre toutes les animations en pause
action-quit-with-save = Quitter (enregistrer la configuration)
action-save-now = Enregistrer la configuration maintenant
action-open-command-palette = Palette de commandes
# Undo (1.5): machine-translated, pending native review.
action-undo = Annuler
action-redo = Rétablir

# Groups (1.5): machine-translated, pending native review.
action-group-selected = Grouper la sélection
action-ungroup-selected = Dissocier la sélection
menu-group = Grouper
menu-ungroup = Dissocier
group-default-name = Groupe { $number }
group-copy-name = { $name } (copie)
toast-grouped = Groupés sous { $name }
toast-ungrouped = Groupes dissous : { $count }
toast-nothing-to-ungroup = Rien de la sélection n’est dans un groupe
scene-groups-empty-hint = Sélectionnez plusieurs personnages et appuyez sur Ctrl+G — ou faites un clic droit dessus et choisissez Grouper.
scene-group-members = Personnages : { $count }
scene-group-select-tooltip = Sélectionner les personnages de ce groupe
scene-group-rename = Renommer le groupe
scene-group-show = Afficher le groupe
scene-group-hide = Masquer le groupe
scene-group-ungroup = Dissocier — les personnages restent

# Arrange and snap (1.5): machine-translated, pending native review.
arrange-align-left = Aligner les bords gauches
arrange-align-center = Centrer sur une ligne verticale
arrange-align-right = Aligner les bords droits
arrange-align-top = Aligner les bords supérieurs
arrange-align-middle = Centrer sur une ligne horizontale
arrange-align-bottom = Aligner les bords inférieurs
arrange-distribute-horizontally = Répartir également en largeur
arrange-distribute-vertically = Répartir également en hauteur
scene-snap = Magnétisme pendant le glissement
scene-snap-tooltip = Les bords et les centres s’aimantent à ceux des écrans et des autres personnages. Maintenez Alt en glissant pour placer librement.

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
# Screen readers (1.4): machine-translated, pending native review.
appearance-accesskit-hint = Permet aux lecteurs d’écran (Orca et d’autres, via AT-SPI) de lire et d’utiliser les panneaux. Désactivé, un lecteur d’écran voit une fenêtre vide. Remarque : tant qu’un lecteur tourne, le texte saisi dans les panneaux passe sur le bus AT-SPI, où tout processus de votre utilisateur peut le lire.
# Machine-translated, pending native review.
appearance-accesskit-unsupported = Les lecteurs d’écran ne sont pas encore pris en charge sur ce système.
appearance-reduced-motion-label = Réduire les animations
appearance-reduced-motion-hint = Ignore les transitions de l’interface (glissement du panneau, fondus, apparition de la palette) et arrête le balancement décoratif. Les animations qui portent un état restent actives.
appearance-hover-startle-label = Sursaut au survol
appearance-hover-startle-hint = Les mascottes reculent devant le pointeur quand il s'approche. Le suivi du curseur est réservé à X11, donc sous Wayland natif cela ne réagit qu'en mode édition.
# Full-screen apps (1.5): machine-translated, pending native review.
appearance-fullscreen-label = Quand une app est en plein écran
appearance-fullscreen-hint = Jeux, vidéos et présentations qui remplissent l’écran. Fonctionne sous X11 et sur les compositeurs Wayland comme sway et Hyprland ; sous GNOME et KDE avec Wayland, seulement pour les apps qui passent par XWayland.
fullscreen-hide = Masquer les personnages
fullscreen-pause = Les mettre en pause
fullscreen-ignore = Les laisser continuer
# Pausing when away (1.5): machine-translated, pending native review.
appearance-away-label = Pause quand je suis absent
appearance-away-hint = Sans souris ni clavier pendant ce temps, les personnages s’immobilisent ; ils reprennent à votre prochaine action.
away-never = Jamais
away-after-minutes = Après { $minutes } min
appearance-away-unavailable = Cette session ne peut pas savoir quand vous êtes absent : il faut GNOME, un vrai serveur X ou un compositeur Wayland avec notifications d’inactivité.
appearance-battery-label = Pause sur batterie
appearance-battery-hint = Les personnages s’immobilisent tant que l’ordinateur fonctionne sur batterie.
# Named scenes (1.5): machine-translated, pending native review.
scene-scenes-header = Scènes
scene-scenes-hint = Enregistrez ce qui est à l’écran sous un nom, et revenez-y ici, depuis la palette de commandes ou avec « Next scene » dans la barre système.
scene-scene-name-hint = Nom de la scène
scene-scene-save-new = Enregistrer comme scène
scene-scene-load-tooltip = Passer à cette scène — Annuler revient en arrière
scene-scene-save-over = Enregistrer ce qui est à l’écran sous { $name }
scene-scene-delete = Supprimer la scène
scene-scene-delete-confirm = Supprimer ?
scene-scene-delete-cancel = Garder
scene-scene-save-failed = Non enregistrée : { $error }
toast-scene-loaded = Scène : { $name }
toast-scene-failed = Scène non chargée : { $error }
toast-no-scenes = Aucune scène enregistrée pour l’instant — enregistrez-en une dans l’onglet Scène.
palette-load-scene = Passer à la scène : { $name }

# ── Warning banners (D.5) — placeholder pending native-speaker audit
warning-global-hotkeys-unavailable = Les raccourcis globaux n’ont pas pu être enregistrés (typique d’une session Wayland native). Le menu de la zone de notification et le bouton ⚙ fonctionnent toujours.
warning-hot-reload-disconnected = Le processus de rechargement à chaud s’est arrêté de façon inattendue ; les modifications de configuration en cours ne s’appliqueront qu’après un redémarrage.
# Machine-translated, pending native review.
warning-config-unreadable = Votre fichier de configuration n'a pas pu être lu ; la scène par défaut a été chargée. L'original est conservé à côté sous le nom config.toml.bak-corrupt.
action-toggle-perf-overlay = Basculer l’overlay de performance

# ── What's new (D.7) — placeholder pending native-speaker audit
# 1.4 highlights: machine-translated, pending native review.
whats-new-header = Nouveautés de la 1.4
whats-new-add-file = « Ajouter un fichier… », dans l’onglet Scène, ajoute des personnages via le sélecteur de fichiers de votre bureau — sans glisser-déposer.
whats-new-palette = Ctrl+K exécute désormais n’importe quelle action, avec son raccourci à côté.
whats-new-screen-readers = Les lecteurs d’écran comme Orca peuvent désormais lire et utiliser le panneau des réglages.
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
shimeji-no-library-toast = Aucun dossier de bibliothèque — créez d’abord { $path }.
crash-report-found-toast = La session précédente a planté. Un rapport a été enregistré dans { $path } — joignez-le à un ticket GitHub.

# ── Group composition hint (C.9) ──────────────────────────────────────
inspector-group-hint = Composé par le groupe { $group } : { $transform }
# Multiple selection (1.5): machine-translated, pending native review.
inspector-multi-selected = { $count } sélectionnés. Ce panneau modifie { $name } ; le glisser, les raccourcis et le menu du clic droit agissent sur tous.

# ── App-layer toasts (V.6 — F1 closure) ──────────────────────────────
toast-config-saved = Configuration enregistrée
# Hot-reload toasts: machine-translated, pending native review.
toast-config-reloaded = Configuration rechargée depuis le disque
toast-config-reload-failed = Configuration non rechargée — le fichier est invalide ou en cours d'écriture. La scène actuelle est conservée.
toast-config-reload-discarded = Configuration non rechargée — vous avez modifié la scène pendant le chargement.
toast-save-failed = Échec de l’enregistrement : { $error }
toast-rejected = Rejeté : { $reason }
toast-added = { $name } ajouté
toast-load-failed = Échec du chargement : { $error }
toast-entity-load-failed = { $name } : { $error }
toast-theme-switched = Thème : { $theme }
toast-preset-entry-failed = Impossible d’ajouter l’entrée du preset : { $error }
toast-preset-loaded = Preset chargé : { $name }
toast-duplicated = { $name } dupliqué
# Multiple selection (1.5): machine-translated, pending native review.
toast-duplicated-many = Personnages dupliqués : { $count }
toast-duplicate-failed = Échec de la duplication : { $error }
toast-deleted = { $name } supprimé
# Multiple selection (1.5): machine-translated, pending native review.
toast-deleted-many = Personnages supprimés : { $count }
toast-playback-resumed = Lecture reprise
toast-playback-paused = Lecture en pause
# Undo (1.5): machine-translated, pending native review.
toast-undone = Annulé
toast-redone = Rétabli
toast-nothing-to-undo = Rien à annuler
toast-nothing-to-redo = Rien à rétablir
inspector-wander-box = Zone d’errance
toast-perf-snapshot = Instantané de performance : { $path }
toast-perf-snapshot-failed = Échec de l’instantané : { $error }
