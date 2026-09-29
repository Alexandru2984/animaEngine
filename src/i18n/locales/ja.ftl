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
inspector-behavior-speed = 速度
inspector-behavior-comfort = 快適距離
inspector-behavior-amplitude = 振幅
inspector-behavior-period = 周期
inspector-double-click-reset-hint = ダブルクリックで既定値に戻します。
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
behavior-script = スクリプト
behavior-script-path-label = パス
behavior-script-path-hint = アセットライブラリからの相対パス — { $path }
behavior-script-params = パラメーター
behavior-script-param-name = 名前
behavior-script-add-param = 追加
behavior-script-remove-param = このパラメーターを削除
script-failed-toast = 動作スクリプト { $script } が失敗しました: { $error }

scene-empty-headline = シーンは空です
scene-empty-hint = PNG / GIF / WebP / MP4 をオーバーレイにドロップ — もしくは下のプリセットをお試しください。
scene-drop-hint = PNG / GIF / WebP をオーバーレイにドロップしてエンティティを追加できます。
# "Add file…" (1.4): machine-translated, pending native review.
scene-add-file = ファイルを追加…
scene-add-file-tooltip = キャラクターとして追加する画像や動画を選びます。
scene-paste = 貼り付け
scene-paste-tooltip = コピーしたキャラクターをこのシーンに追加します。コピー済み: { $count }
scene-paste-empty = まだ何もコピーしていません。このシーンや別のシーンでキャラクターを選択して Ctrl+C を押すか、右クリックして「コピー」を選ぶと、ここに貼り付けられます。
file-chooser-title = キャラクターを追加
file-chooser-filter = 画像と動画
share-chooser-title = シーンを共有
import-chooser-title = シーンを読み込む
import-chooser-filter = animaEngine のシーン
file-chooser-unavailable-toast = ファイル選択ダイアログを開けませんでした。xdg-desktop-portal が必要です。
scene-presets-header = プリセット
scene-groups-header = グループ
scene-preset-append = 追加
scene-preset-replace = 置換
scene-preset-replace-tooltip = 追加前に現在のシーンを消去します

monitor-section-header = モニター
monitor-mode-label = 配分
monitor-mode-per-monitor = モニター毎
monitor-mode-span = 全モニターにまたがって表示
monitor-mode-span-unsupported = このバックエンドでは 1 つのオーバーレイで複数のモニターを覆えません。「モニターごと」を使ってください。
monitor-mode-single = 単一モニター
scene-window-awareness = ウィンドウに着地（X11）
scene-window-awareness-tooltip = 物理が有効なキャラクターは、開いているウィンドウの上端に着地して歩きます。X11 セッション限定 — Wayland はウィンドウ位置を公開しないため、そこでは何も起きません。
scene-window-awareness-unavailable = ネイティブ Wayland では利用できません — ウィンドウ位置を公開するプロトコルがありません。
monitor-pin-label = モニターに固定
monitor-pin-auto = 自動 (位置に従う)
monitor-pinned-toast = エンティティを { $name } に固定しました
monitor-pin-cleared-toast = エンティティは位置に従います
monitor-no-monitors-detected = モニターが検出されません

appearance-theme-header = テーマ
appearance-theme-label = テーマ
appearance-language-header = 言語
appearance-language-no-font = この文字体系のフォントが見つかりません。Noto CJK パッケージを導入してください。
theme-dark = ダーク
theme-light = ライト
theme-dark-hc = ダーク · ハイコントラスト
theme-light-hc = ライト · ハイコントラスト

onboarding-tabs = 設定は 5 つのタブに分かれています — インスペクター、シーン、ライブラリ、外観、ショートカット。
onboarding-quick-toggles = ヒント: V で表示の切替、G で重力の切替 — このパネルを開かずに操作できます。
onboarding-theme = テーマは即座に適用されます — 再起動は不要です。
onboarding-coach-step1 = ようこそ！キャラクターはデスクトップに住んでいます。右上の歯車ボタンをクリックして編集モードに入りましょう。
onboarding-coach-step2 = PNG・GIF・WebP・MP4 を画面のどこかにドロップすると、キャラクターとして追加されます。サイドパネルで選択したものを編集できます。
onboarding-coach-step3 = Ctrl+K でコマンドパレットが開きます。Ctrl+Shift+A はどこからでも編集モードを切り替え、Ctrl+Shift+H はオーバーレイを隠します。
onboarding-coach-next = 次へ
onboarding-coach-skip = ツアーをスキップ
onboarding-coach-done = わかった
palette-replace-row = シーンを置き換え: { $preset }
palette-append-row = プリセットを追加: { $preset }
palette-footer-hint = Esc で閉じる · Ctrl+K で切替 · ↑↓ + Enter で選択
onboarding-dismiss = 閉じる

menu-duplicate = 複製
menu-copy = コピー
menu-cut = 切り取り
menu-reset-transform = 変形をリセット
menu-toggle-gravity = 重力を切替
menu-bring-forward = 前面へ
menu-send-backward = 背面へ
menu-delete = 削除

toggle-enter-edit = 編集モードに入る
toggle-exit-edit = 編集モードを終了

# Palette placeholder (1.4): machine-translated, pending native review.
palette-search-placeholder = 操作・テーマ・プリセットを検索…
palette-switch-theme = { $theme } テーマに切替

settings-tab-library = ライブラリ

# Asset library tab
library-empty-headline = アセットが見つかりません
library-empty-hint = { $path } にファイルを入れるか、ANIMA_ASSETS_DIR を設定してください。
library-no-asset-root = アセットディレクトリが見つかりません。{ $path } に作成してください
library-search-placeholder = アセットを検索…
library-add-to-scene = シーンに追加
library-kind-image = 画像
library-kind-animated = アニメーション
library-kind-video = 動画
library-asset-added-toast = { $name } をシーンに追加しました
library-asset-add-failed-toast = { $name } を追加できませんでした
library-count = { $n } 個のアセットがインデックス済み

# ── Keybindings tab (D.1) — placeholder pending D.4 native-speaker audit
settings-tab-keybindings = ショートカット
keybindings-unbound = （未割り当て）
keybindings-add = 追加
keybindings-recording = キーの組み合わせを押してください…（Esc でキャンセル）
keybindings-conflict = { $action } と競合しています
keybindings-reset-all = すべて既定値に戻す
keybindings-reset-one = 既定値に戻す
keybindings-remove-chord = このキー割り当てを削除
keybindings-help = カスタムショートカットは config.toml に保存されます
# Modifier names as this language's keyboards print them. Display only:
# config.toml always stores the English names. Kept in English unless
# the convention is certain; see R39 in docs/runtime-findings.md.
key-mod-ctrl = Ctrl
key-mod-shift = Shift
key-mod-alt = Alt
key-mod-super = Super

# ── Action labels (D.1.7) — placeholder pending D.4 native-speaker audit
action-toggle-edit-mode = 編集モードを切り替え
action-hide-overlay = オーバーレイの表示／非表示
action-pause-all = すべてのアニメーションを一時停止
action-quit-with-save = 終了（設定を保存）
action-save-now = 設定を今すぐ保存
action-open-command-palette = コマンドパレット
# Undo (1.5): machine-translated, pending native review.
action-undo = 元に戻す
action-redo = やり直す

# Groups (1.5): machine-translated, pending native review.
action-group-selected = 選択をグループ化
action-ungroup-selected = 選択のグループを解除
menu-group = グループ化
menu-ungroup = グループ解除
group-default-name = グループ { $number }
group-copy-name = { $name } のコピー
toast-grouped = { $name } としてグループ化しました
toast-ungrouped = 解除したグループ: { $count }
toast-nothing-to-ungroup = 選択中のものはどのグループにも属していません
scene-groups-empty-hint = 複数のキャラクターを選択して Ctrl+G を押すか、右クリックして「グループ化」を選びます。
scene-group-members = キャラクター: { $count }
scene-group-select-tooltip = このグループのキャラクターを選択
scene-group-rename = グループ名を変更
scene-group-show = グループを表示
scene-group-hide = グループを非表示
scene-group-ungroup = グループ解除(キャラクターは残ります)

# Arrange and snap (1.5): machine-translated, pending native review.
arrange-align-left = 左端を揃える
arrange-align-center = 縦の中心線で揃える
arrange-align-right = 右端を揃える
arrange-align-top = 上端を揃える
arrange-align-middle = 横の中心線で揃える
arrange-align-bottom = 下端を揃える
arrange-distribute-horizontally = 横に等間隔で並べる
arrange-distribute-vertically = 縦に等間隔で並べる
scene-snap = ドラッグ中にスナップ
scene-snap-tooltip = 端と中心が画面や他のキャラクターの端と中心に吸着します。ドラッグ中に Alt を押すと自由に配置できます。
scene-bump = キャラクター同士がぶつかる
scene-bump-tooltip = 重力 (G) がオンのとき、キャラクターは他のキャラクターの頭に着地し、歩くキャラクターは出会うと向きを変えます。

action-cycle-entity = 次のキャラクターへ
action-delete-selected = 選択したキャラクターを削除
action-nudge-up = 選択を上へ移動
action-nudge-down = 選択を下へ移動
action-nudge-left = 選択を左へ移動
action-nudge-right = 選択を右へ移動
action-center-on-screen = 選択を画面中央へ
action-toggle-visible = 表示を切り替え
action-toggle-gravity = 重力を切り替え
action-toggle-playback = 再生／一時停止
action-duplicate-selected = 選択を複製
action-copy-selected = 選択をコピー
action-cut-selected = 選択を切り取り
action-paste = 貼り付け
action-reset-transform = 拡大率／不透明度をリセット
action-bring-forward = 選択を前面へ
action-send-backward = 選択を背面へ
action-fps-up = FPS を上げる
action-fps-down = FPS を下げる
action-opacity-up = 不透明度を上げる
action-opacity-down = 不透明度を下げる
action-cycle-monitor = モニター固定を切り替え
action-show-entity-info = キャラクター情報を表示
action-show-help = キーボードヘルプを表示

# ── Accessibility section (D.3) — placeholder pending D.4 native-speaker audit
appearance-accessibility-header = アクセシビリティ
appearance-accesskit-label = AccessKit ツリー更新を生成
# Screen readers (1.4): machine-translated, pending native review.
appearance-accesskit-hint = スクリーンリーダー（Orca など、AT-SPI 経由）がパネルを読み上げ・操作できるようにします。オフにすると、スクリーンリーダーには空のウィンドウが見えます。注意：スクリーンリーダーの動作中は、パネルに入力したテキストが AT-SPI バスに流れ、同じユーザーのプロセスなら読み取れます。
# Machine-translated, pending native review.
appearance-accesskit-unsupported = このシステムではスクリーンリーダーはまだサポートされていません。
appearance-reduced-motion-label = 動きを減らす
appearance-reduced-motion-hint = UI のトランジション（パネルのスライド、フェード、パレットのポップ）を省略し、装飾的な揺れを止めます。状態を伝えるアニメーションは動き続けます。
appearance-hover-startle-label = ホバーでびっくり
appearance-hover-startle-hint = カーソルが近づくとマスコットが後ずさりします。カーソル追跡はX11のみのため、ネイティブWaylandでは編集モードでのみ反応します。
# Full-screen apps (1.5): machine-translated, pending native review.
appearance-fullscreen-label = アプリが全画面のとき
appearance-fullscreen-hint = 画面いっぱいのゲーム、動画、プレゼンテーション。X11 と、sway や Hyprland などの Wayland コンポジターで動作します。Wayland の GNOME と KDE では、XWayland で動くアプリのみが対象です。
fullscreen-hide = キャラクターを隠す
fullscreen-pause = 一時停止する
fullscreen-ignore = そのまま続ける
# Pausing when away (1.5): machine-translated, pending native review.
appearance-away-label = 離席中は一時停止
appearance-away-hint = この時間マウスやキーボードの入力がないと、キャラクターは止まって着地し、うたた寝します。次の入力で目を覚まします。
away-never = しない
away-after-minutes = { $minutes } 分後
appearance-away-unavailable = このセッションでは離席を検知できません。GNOME、実際の X サーバー、またはアイドル通知に対応した Wayland コンポジターが必要です。
appearance-battery-label = バッテリー駆動中は一時停止
appearance-battery-hint = コンピューターがバッテリーで動いている間、キャラクターを止めます。
# Start at login (1.5): machine-translated, pending native review.
appearance-autostart-label = ログイン時に起動
appearance-autostart-hint = デスクトップにログインしたときに animaEngine を起動します。
appearance-autostart-reason = ログイン時に animaEngine を起動するため。
appearance-autostart-failed = 変更できませんでした: { $error }
# Named scenes (1.5): machine-translated, pending native review.
scene-scenes-header = シーン
scene-scenes-hint = 画面の内容に名前を付けて保存し、ここやコマンドパレット、トレイの「Next scene」から戻れます。
scene-scene-name-hint = シーン名
scene-scene-save-new = シーンとして保存
scene-scene-load-tooltip = このシーンに切り替え(元に戻すで戻れます)
scene-scene-save-over = 画面の内容を { $name } として保存
scene-scene-share = { $name } を画像ごと 1 つのファイルで共有
scene-import = 読み込む…
scene-import-tooltip = 共有されたシーン（.animascene ファイル）を開きます。ファイルをオーバーレイにドロップしても開けます。
scene-scene-delete = シーンを削除
scene-scene-delete-confirm = 削除しますか?
scene-scene-delete-cancel = 残す
scene-scene-save-failed = 保存できませんでした: { $error }
toast-scene-loaded = シーン: { $name }
toast-scene-failed = シーンを読み込めませんでした: { $error }
toast-scene-shared = 共有用のシーンを保存しました: { $path }
toast-scene-shared-scripts = 動作スクリプトは共有されません。ファイル内で止まっているキャラクター: { $count }
toast-scene-share-failed = 共有に失敗しました: { $error }
toast-scene-imported = シーンを読み込みました: { $name }
toast-scene-import-failed = 読み込みに失敗しました: { $error }
toast-no-scenes = 保存したシーンはまだありません。シーンタブで保存してください。
palette-load-scene = シーンに切り替え: { $name }
# Reminders (1.5): machine-translated, pending native review.
scene-reminders-header = リマインダー
scene-reminders-hint = キャラクターが時々吹き出しで知らせます。コンピューターから離れていた時間は休憩として数え、最初からやり直します。
scene-reminder-every = { $minutes } 分ごと
scene-reminder-anyone = どのキャラクターでも
scene-reminder-delete = リマインダーを削除
scene-reminder-text-hint = 言うこと
scene-reminder-minutes = 間隔 (分)
scene-reminder-who = 言うキャラクター
scene-reminder-add = リマインダーを追加
reminder-break = 少し休憩しましょう!
reminder-water = 水を一杯飲みましょう。
reminder-stretch = 立ち上がってストレッチしましょう。
# Scenes by time of day (1.5): machine-translated, pending native review.
scene-schedule-header = 自動で切り替え
scene-schedule-hint = 決まった時刻に保存済みのシーンへ切り替えます(例: 平日 09:00 に「仕事」)。編集中は待ちます。
scene-schedule-needs-scene = スケジュールで切り替えるには、上でシーンを保存してください。
scene-schedule-row = { $time } · { $days } → { $scene }
scene-schedule-every-day = 毎日
scene-schedule-weekdays = 平日
scene-schedule-weekends = 週末
scene-schedule-at = 時刻
scene-schedule-minute = 分
scene-schedule-days = 曜日
scene-schedule-scene = シーン
scene-schedule-add = 追加
scene-schedule-delete = ルールを削除

# ── Warning banners (D.5) — placeholder pending native-speaker audit
warning-global-hotkeys-unavailable = グローバルホットキーを登録できませんでした（ネイティブ Wayland セッションでは一般的）。トレイメニューと ⚙ ボタンは引き続き使えます。
warning-hot-reload-disconnected = ホットリロードのワーカーが予期せず停止しました。進行中の設定変更はアプリの再起動後に反映されます。
# Machine-translated, pending native review.
warning-config-unreadable = 設定ファイルを読み込めなかったため、既定のシーンを読み込みました。元のファイルは同じ場所に config.toml.bak-corrupt として残してあります。
action-toggle-perf-overlay = パフォーマンス表示を切り替え

# ── What's new (D.7) — placeholder pending native-speaker audit
# 1.4 highlights: machine-translated, pending native review.
whats-new-header = 1.4 の新機能
whats-new-add-file = シーンタブの「ファイルを追加…」で、デスクトップのファイル選択ダイアログからキャラクターを追加できます。ドラッグは不要です。
whats-new-palette = Ctrl+K で任意の操作を実行できるようになり、横にショートカットも表示されます。
whats-new-screen-readers = Orca などのスクリーンリーダーで設定パネルを読み上げ・操作できるようになりました。
onboarding-keybindings = ショートカットをクリックすると削除、キーの組み合わせを押すと新規登録できます。
onboarding-perf-overlay = Ctrl+Shift+` でライブのパフォーマンス表示を開けます。
appearance-reset-onboarding = オンボーディングのヒントをリセット

scene-empty-action-browse-presets = プリセットを見る
library-empty-action-copy-path = パスをクリップボードにコピー

appearance-reset-onboarding-hint = 閉じたヒントと「新着情報」パネルを復活させます。

# ── Portal shortcuts (T.3) ────────────────────────────────────────────
portal-denied-x11-fallback-toast = ショートカットの許可が拒否されました — 代わりに X11 ホットキーを使用します。「ショートカット」タブから再試行できます。
portal-denied-native-toast = ショートカットの許可が拒否されました — トレイメニューとコンポジターのバインドは引き続き使えます。

# ── Keybindings backend status (T.4) ─────────────────────────────────
keybindings-backend-label = グローバルショートカットの方式:
keybindings-backend-tooltip = 他のアプリにフォーカスがあるとき、3 つのグローバルショートカット（編集・非表示・一時停止）をどの仕組みで届けるか。起動時に決定されます。アプリ内ショートカットには影響しません。
keybindings-portal-restart-hint = トリガーの変更は次回起動時に反映されます（デスクトップが承認を記憶します）。

# ── Monitor hotplug (T.9) ─────────────────────────────────────────────
monitor-unplugged-toast = モニター { $name } が切断されました — 固定中の { $n } 体は位置に従います。
monitor-plugged-toast = モニター { $name } が接続されました。

# ── Shimeji import (U.4) ──────────────────────────────────────────────
library-import-shimeji-header = Shimeji パックをインポート
library-import-shimeji-hint = パックのフォルダーをオーバーレイにドロップするか、パスをここに貼り付けてください。スプライトはライブラリにコピーされます。
library-import-shimeji-button = インポート
shimeji-imported-toast = { $name } をインポートしました（{ $n } 個の要素をスキップ — ログ参照）
shimeji-import-failed-toast = インポートに失敗しました: { $reason }
shimeji-no-library-toast = ライブラリのフォルダーがありません — まず { $path } を作成してください。
crash-report-found-toast = 前回のセッションがクラッシュしました。レポートを { $path } に保存しました — GitHub の issue に添付してください。

# ── Group composition hint (C.9) ──────────────────────────────────────
inspector-group-hint = グループ { $group } による合成: { $transform }
# Multiple selection (1.5): machine-translated, pending native review.
inspector-multi-selected = { $count } 件を選択中。このパネルは { $name } を編集します。ドラッグ、ショートカット、右クリックメニューはすべてに作用します。

# ── App-layer toasts (V.6 — F1 closure) ──────────────────────────────
toast-config-saved = 設定を保存しました
# Hot-reload toasts: machine-translated, pending native review.
toast-config-reloaded = 設定をディスクから再読み込みしました
toast-config-reload-failed = 設定を再読み込みしませんでした。ファイルが無効か、書き込み中です。現在のシーンをそのまま使います。
toast-config-reload-discarded = 設定を再読み込みしませんでした。読み込み中にシーンが編集されました。
toast-save-failed = 保存に失敗しました: { $error }
toast-rejected = 拒否されました: { $reason }
toast-added = { $name } を追加しました
toast-load-failed = 読み込みに失敗しました: { $error }
toast-entity-load-failed = { $name }: { $error }
toast-theme-switched = テーマ: { $theme }
toast-preset-entry-failed = プリセット項目を追加できませんでした: { $error }
toast-preset-loaded = プリセットを読み込みました: { $name }
toast-duplicated = { $name } を複製しました
# Multiple selection (1.5): machine-translated, pending native review.
toast-duplicated-many = 複製したキャラクター: { $count }
toast-duplicate-failed = 複製に失敗しました: { $error }
toast-deleted = { $name } を削除しました
# Multiple selection (1.5): machine-translated, pending native review.
toast-deleted-many = 削除したキャラクター: { $count }
toast-copied = { $name } をコピーしました
toast-copied-many = コピーしたキャラクター: { $count }
toast-cut = { $name } を切り取りました
toast-cut-many = 切り取ったキャラクター: { $count }
toast-pasted = { $name } を貼り付けました
toast-pasted-many = 貼り付けたキャラクター: { $count }
toast-paste-failed = 貼り付けに失敗しました: { $error }
toast-nothing-to-paste = 貼り付けるものがありません。先にキャラクターをコピーしてください
toast-playback-resumed = 再生を再開しました
toast-playback-paused = 再生を一時停止しました
# Undo (1.5): machine-translated, pending native review.
toast-undone = 元に戻しました
toast-redone = やり直しました
toast-nothing-to-undo = 元に戻す操作はありません
toast-nothing-to-redo = やり直す操作はありません
inspector-wander-box = 徘徊範囲
toast-perf-snapshot = パフォーマンススナップショット: { $path }
toast-perf-snapshot-failed = スナップショットに失敗しました: { $error }
