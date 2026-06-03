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
monitor-pin-label = モニターに固定
monitor-pin-auto = 自動 (位置に従う)
monitor-pinned-toast = エンティティを { $name } に固定しました
monitor-pin-cleared-toast = エンティティは位置に従います
monitor-no-monitors-detected = モニターが検出されません

appearance-theme-header = テーマ
appearance-theme-label = テーマ
appearance-language-header = 言語
appearance-keyboard-header = キーボード
appearance-keyboard-note = 0.2.0 では読み取り専用です。リバインドは後続リリースで対応予定。
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
