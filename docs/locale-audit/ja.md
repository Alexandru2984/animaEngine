# Locale audit — 日本語 (`ja`)

**Status:** partial. Pre-D.1 strings translated previously; D.1.6, D.1.7, D.3 keys carry English placeholder.
**AI cross-check confidence:** low. An LLM can produce grammatical Japanese for technical UI but cannot reliably judge politeness level (です/ます-form for UI labels is conventional; ない/する dictionary-form for button verbs varies by product). Particle choice (は / が / を / に) for short imperative labels can also drift. **Native review essential before merging.**

## Glossary

| English | 日本語 (recommended) | Notes |
|---|---|---|
| overlay | **オーバーレイ** | Katakana loan, standard. |
| scene | **シーン** | |
| entity | **エンティティ** | Katakana loan; *オブジェクト* is the alternative if more general. |
| edit mode | **編集モード** | |
| chord (key combo) | **キーの組み合わせ** / **ショートカット** | |
| library | **ライブラリ** | |
| monitor pin | **モニターに固定** | |
| preset | **プリセット** | |

## Placeholder English — proposed translations (AI; review essential)

### D.1.6 — Keybindings tab UI

```ftl
settings-tab-keybindings = キーバインド
keybindings-unbound = （未割り当て）
keybindings-add = 追加
keybindings-recording = キーの組み合わせを押してください…（Esc でキャンセル）
keybindings-conflict = { $action } と競合しています
keybindings-reset-all = すべて初期値に戻す
keybindings-help = カスタムショートカットは config.toml に保存されます
```

### D.1.7 — Action labels

| key | suggested Japanese |
|---|---|
| `action-toggle-edit-mode` | 編集モードを切り替え |
| `action-hide-overlay` | オーバーレイを表示／非表示 |
| `action-pause-all` | すべてのアニメーションを一時停止 |
| `action-quit-with-save` | 終了（設定を保存） |
| `action-save-now` | 設定を保存 |
| `action-open-command-palette` | コマンドパレット |
| `action-cycle-entity` | 次のエンティティへ |
| `action-delete-selected` | 選択中のエンティティを削除 |
| `action-nudge-up` | 選択を上に移動 |
| `action-nudge-down` | 選択を下に移動 |
| `action-nudge-left` | 選択を左に移動 |
| `action-nudge-right` | 選択を右に移動 |
| `action-center-on-screen` | 選択を画面中央に配置 |
| `action-toggle-visible` | 表示を切り替え |
| `action-toggle-gravity` | 重力を切り替え |
| `action-toggle-playback` | 再生／一時停止を切り替え |
| `action-duplicate-selected` | 選択を複製 |
| `action-reset-transform` | スケール／不透明度をリセット |
| `action-bring-forward` | 選択を前面へ |
| `action-send-backward` | 選択を背面へ |
| `action-fps-up` | FPS を上げる |
| `action-fps-down` | FPS を下げる |
| `action-opacity-up` | 不透明度を上げる |
| `action-opacity-down` | 不透明度を下げる |
| `action-cycle-monitor` | エンティティのモニターを切り替え |
| `action-show-entity-info` | エンティティ情報を表示 |
| `action-show-help` | キーボードヘルプを表示 |

### D.3 — Accessibility section

```ftl
appearance-accessibility-header = アクセシビリティ
appearance-accesskit-label = AccessKit ツリーの更新を生成
appearance-accesskit-hint = AT-SPI 系スクリーンリーダー（Orca など）に情報を供給します。基本的にはオンのままで構いません。負荷を減らしたい場合や AT-SPI バスを持たないデスクトップ環境では無効化できます。
```

## Suspected issues for native reviewer

### Politeness level mixing

UI labels in JA often drop です/ます endings for brevity (`保存` instead of `保存します`). The proposed strings use the bare-verb / noun-only forms consistently, but the `appearance-accesskit-hint` paragraph mixes informal verb forms with polite ones. Pick one register for hint paragraphs (likely です/ます) and apply.

### Particle drift on `keybindings-conflict`

`{ $action } と競合しています` reads as "is in conflict with { $action }." Verify this reads naturally when `{ $action }` is a quoted action label like `編集モードを切り替え` — the resulting sentence parses but feels formal. Alternatives:

- `{ $action } と重複しています` (duplicates with…)
- `競合: { $action }` (Conflict: …) — terser, mirrors VSCode-style.

### Katakana loan management

The proposed strings use loanwords (オーバーレイ, シーン, エンティティ, プリセット) which match modern Japanese software conventions. Some house styles prefer native compounds where possible — e.g., *重ね描画* for overlay, *場面* for scene. Confirm policy with the existing pre-D file.

## Open questions for native reviewer

- Polite vs casual register for full-sentence hints (recommend です/ます).
- Loanword vs native compound policy.
- Whether to use の/を/に particles after `{ $action }` arguments — the placeholder above uses `と` which is grammatically valid but may sound stilted.
- Spacing around inline arguments: JA conventionally has no space around variables, but punctuation conventions differ from EN.
