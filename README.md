Nostr CLI App

ノスターのコマンドラインインターフェースアプリケーション。
機能

    鍵ペアの生成と管理
    テキストノートの送信
    イベントフィードの表示
    リレーの管理
    TUIモードでの操作
    「うぃビーム」効果音の再生

インストール
Bash
git clone https://github.com/tami1A84/nostr-cli-app.git
cd nostr-cli-app
cargo build --release

使用方法
鍵ペアの生成
Bash
cargo run -- generate-keys

TUIモードで起動
Bash
cargo run -- tui

「うぃビーム」効果音を再生
Bash
cargo run -- uibeam
