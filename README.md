# Nostr CLI App

- 鍵ペアの生成と管理

- テキストノートの送信

- イベントフィードの表示

- リレーの管理

- TUIモードでの操作

- 意味のない電卓

# インストール
```Bash
git clone https://github.com/tami1A84/nostr-cli-app.git 
cd nostr-cli-app 
cargo build --release
```

# 使用方法
### 鍵ペアの生成
```Bash
cargo run -- generate-keys
```

### TUIモードで起動
```Bash
cargo run -- tui
```
### キー操作ガイド

### 共通
- `q`: アプリケーション終了
- `Tab`: タブ切り替え（イベントリスト <-> 投稿作成）

### 通常モード
- `i`: 入力モードに切り替え
- `r`: イベントを更新
- `a`: About画面の表示/非表示
- `s`: 電卓の表示/非表示
- `Enter`: 選択したイベントの詳細表示
- `↑`/`↓`: リスト内移動
- `Home`/`End`: リストの先頭/末尾に移動
- `PageUp`/`PageDown`: リスト内ページ移動

### 編集モード
- `Enter`: メッセージ送信
- `Esc`: 通常モードに戻る
- `Backspace`: 文字を削除

### 詳細表示モード
- `Esc`: イベントリストに戻る
- `↑`/`↓`: 長文スクロール


# コマンド一覧
```Bash
cargo run -- generate-keys [--password <パスワード>] 新しい鍵ペアの生成
cargo run -- show-keys 鍵情報の表示
cargo run -- send-note <投稿内容> テキストノートの送信
cargo run -- show-feed イベントフィードの表示
cargo run -- add-relay <リレーURL> リレーの追加
cargo run -- remove-relay <リレーURL> リレーの削除
cargo run -- list-relays リレー一覧の表示
cargo run -- uibeam 「ういビーム」効果音の再生
cargo run -- tui ターミナルUIモードでの起動
```

