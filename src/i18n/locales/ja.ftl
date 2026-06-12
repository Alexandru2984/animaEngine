# 日本語 — ベース翻訳。ネイティブ話者によるレビュー待ち。

app-name = animaEngine

settings-tab-inspector = インスペクター
settings-tab-scene = シーン
settings-tab-appearance = 外観
entity-count-zero = エンティティなし
entity-count-singular = { $n } 個のエンティティ
entity-count-plural = { $n } 個のエンティティ

inspector-section-position = 位置
inspector-section-appearance = 外観
inspector-section-animation = アニメーション
animation-easing-label = イージング
easing-linear = リニア
easing-ease-in-quad = イーズイン
easing-ease-out-quad = イーズアウト
easing-ease-in-out-quad = イーズイン/アウト
easing-sine = サイン
easing-bounce-out = バウンスアウト
inspector-section-behavior = ふるまい
inspector-visible = 表示
inspector-gravity = 重力
inspector-scale = スケール
inspector-opacity = 不透明度
inspector-fps = FPS
inspector-playing = 再生中
inspector-x = X
inspector-y = Y
inspector-z-index = z-index
inspector-nothing-selected-headline = 何も選択されていません
inspector-nothing-selected-hint = シーンタブでエンティティをクリックするか、Tab キーで順に切り替えてください。

behavior-idle = 待機
behavior-walk = 歩き回る
behavior-follow = カーソルを追う
behavior-wander = 範囲内をさまよう
behavior-bounce = バウンス
behavior-bounce-axis = 軸
behavior-bounce-horizontal = 水平
behavior-bounce-vertical = 垂直
behavior-bounce-both = 両方 (円)

scene-empty-headline = シーンは空です
scene-empty-hint = PNG / GIF / WebP / MP4 をオーバーレイにドロップ — もしくは下のプリセットをお試しください。
scene-drop-hint = PNG / GIF / WebP をオーバーレイにドロップしてエンティティを追加できます。
scene-presets-header = プリセット
scene-preset-append = 追加
scene-preset-replace = 置換
scene-preset-replace-tooltip = 追加前に現在のシーンを消去します

monitor-section-header = モニター
monitor-mode-label = 配分
monitor-mode-per-monitor = モニター毎
monitor-mode-span = 全モニターにまたがって表示
monitor-mode-single = 単一モニター
scene-window-awareness = Land on windows (X11)
scene-window-awareness-tooltip = Physics-enabled characters land on and walk along the top edges of your open windows. X11 sessions only — Wayland offers no window positions, so this does nothing there.
monitor-pin-label = モニターに固定
monitor-pin-auto = 自動 (位置に従う)
monitor-pinned-toast = エンティティを { $name } に固定しました
monitor-pin-cleared-toast = エンティティは位置に従います
monitor-no-monitors-detected = モニターが検出されません

appearance-theme-header = テーマ
appearance-theme-label = テーマ
appearance-language-header = 言語
theme-dark = ダーク
theme-light = ライト
theme-dark-hc = ダーク · ハイコントラスト
theme-light-hc = ライト · ハイコントラスト

onboarding-tabs = 設定は 3 つのタブに分かれています — インスペクター・シーン・外観。
onboarding-quick-toggles = ヒント: V で表示の切替、G で重力の切替 — このパネルを開かずに操作できます。
onboarding-theme = テーマは即座に適用されます — 再起動は不要です。
onboarding-dismiss = 閉じる

menu-duplicate = 複製
menu-reset-transform = 変形をリセット
menu-toggle-gravity = 重力を切替
menu-bring-forward = 前面へ
menu-send-backward = 背面へ
menu-delete = 削除

toggle-enter-edit = 編集モードに入る
toggle-exit-edit = 編集モードを終了

palette-search-placeholder = テーマ / プリセットを検索…
palette-close-hint = Esc で閉じる · Ctrl+K で切替
palette-switch-theme = { $theme } テーマに切替
palette-apply-preset = プリセットを適用: { $preset }

settings-tab-library = ライブラリ

# Asset library tab
library-empty-headline = アセットが見つかりません
library-empty-hint = ~/.local/share/animaEngine/assets/ にファイルを入れるか、ANIMA_ASSETS_DIR を設定してください。
library-no-asset-root = アセットディレクトリが見つかりません。~/.local/share/animaEngine/assets/ に作成してください
library-search-placeholder = アセットを検索…
library-add-to-scene = シーンに追加
library-sort-recent = 最近
library-sort-name = 名前
library-kind-image = 画像
library-kind-animated = アニメーション
library-kind-video = 動画
library-asset-added-toast = { $name } をシーンに追加しました
library-asset-add-failed-toast = { $name } を追加できませんでした
library-count = { $n } 個のアセットがインデックス済み

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
