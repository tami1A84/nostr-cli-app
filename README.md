# Nostr CLI App

- 鍵ペアの生成と管理

- テキストノートの送信

- イベントフィードの表示

- リレーの管理

- TUIモードでの操作

- 「ういビーム」効果音の再生

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

# コマンド一覧

### 新しい鍵ペアの生成
```Bash
cargo run -- generate-keys [--password <パスワード>]
```


### 鍵情報の表示
```Bash
cargo run -- show-keys
```


### テキストノートの送信
```Bash
cargo run -- send-note <投稿内容>
```


### イベントフィードの表示
```Bash
cargo run -- show-feed
```



### リレーの追加
```Bash
cargo run -- add-relay <リレーURL>
```

### リレーの削除
```Bash
cargo run -- remove-relay <リレーURL>
```


### リレー一覧の表示
```Bash
cargo run -- list-relays
```



### 「ういビーム」効果音の再生
```Bash
cargo run -- uibeam
```


### ターミナルUIモードでの起動
```Bash
cargo run -- tui
```
