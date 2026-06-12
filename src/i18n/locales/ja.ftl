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
scene-window-awareness = ウィンドウに着地（X11）
scene-window-awareness-tooltip = 物理が有効なキャラクターは、開いているウィンドウの上端に着地して歩きます。X11 セッション限定 — Wayland はウィンドウ位置を公開しないため、そこでは何も起きません。
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
settings-tab-keybindings = ショートカット
keybindings-unbound = （未割り当て）
keybindings-add = 追加
keybindings-recording = キーの組み合わせを押してください…（Esc でキャンセル）
keybindings-conflict = { $action } と競合しています
keybindings-reset-all = すべて既定値に戻す
keybindings-help = カスタムショートカットは config.toml に保存されます

# ── Action labels (D.1.7) — placeholder pending D.4 native-speaker audit
action-toggle-edit-mode = 編集モードを切り替え
action-hide-overlay = オーバーレイの表示／非表示
action-pause-all = すべてのアニメーションを一時停止
action-quit-with-save = 終了（設定を保存）
action-save-now = 設定を今すぐ保存
action-open-command-palette = コマンドパレット
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
appearance-accesskit-hint = AT-SPI スクリーンリーダー（Orca など）に情報を提供します。リソースを節約したい場合やデスクトップに AT-SPI バスがない場合を除き、オンのままにしてください。注意：パネルに入力したテキストも AT-SPI バスに流れ、同じユーザーのプロセスなら読み取れます。
appearance-reduced-motion-label = 動きを減らす
appearance-reduced-motion-hint = UI のトランジション（パネルのスライド、フェード、パレットのポップ）を省略し、装飾的な揺れを止めます。状態を伝えるアニメーションは動き続けます。

# ── Warning banners (D.5) — placeholder pending native-speaker audit
warning-global-hotkeys-unavailable = グローバルホットキーを登録できませんでした（ネイティブ Wayland セッションでは一般的）。トレイメニューと ⚙ ボタンは引き続き使えます。
warning-hot-reload-disconnected = ホットリロードのワーカーが予期せず停止しました。進行中の設定変更はアプリの再起動後に反映されます。
action-toggle-perf-overlay = パフォーマンス表示を切り替え

# ── What's new (D.7) — placeholder pending native-speaker audit
whats-new-header = 0.4 の新着情報
whats-new-keybindings = キーボードショートカットの割り当て変更 — 新しい「ショートカット」タブを開いてください。
whats-new-collapse-state = インスペクターのセクションは開閉状態をセッションをまたいで記憶します。
whats-new-error-banners = これまで無音だったエラーは、トーストやバナーで表示されるようになりました。
whats-new-accessibility-toggle = AccessKit は 外観 → アクセシビリティ からオフにできます。
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
shimeji-no-library-toast = ライブラリのフォルダーがありません — まず ~/.local/share/animaEngine/assets/ を作成してください。
crash-report-found-toast = 前回のセッションがクラッシュしました。レポートを { $path } に保存しました — GitHub の issue に添付してください。

# ── Group composition hint (C.9) ──────────────────────────────────────
inspector-group-hint = グループ { $group } による合成: { $transform }

# ── App-layer toasts (V.6 — F1 closure) ──────────────────────────────
toast-config-saved = 設定を保存しました
toast-save-failed = 保存に失敗しました: { $error }
toast-rejected = 拒否されました: { $reason }
toast-added = { $name } を追加しました
toast-load-failed = 読み込みに失敗しました: { $error }
toast-entity-load-failed = { $name }: { $error }
toast-theme-switched = テーマ: { $theme }
toast-preset-entry-failed = プリセット項目を追加できませんでした: { $error }
toast-preset-loaded = プリセットを読み込みました: { $name }
toast-duplicated = { $name } を複製しました
toast-duplicate-failed = 複製に失敗しました: { $error }
toast-deleted = { $name } を削除しました
toast-playback-resumed = 再生を再開しました
toast-playback-paused = 再生を一時停止しました
toast-wayland-no-library = Wayland 経路ではアセットライブラリはまだ使えません
inspector-wander-box = 徘徊範囲
toast-perf-snapshot = パフォーマンススナップショット: { $path }
toast-perf-snapshot-failed = スナップショットに失敗しました: { $error }
