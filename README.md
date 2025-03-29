# Nostr CLI App

ノストのコマンドラインインターフェースアプリケーション。

## 機能

- 鍵ペアの生成と管理
- テキストノートの送信
- イベントフィードの表示
- リレーの管理
- TUIモードでの操作
- 「うぃビーム」効果音の再生

## インストール


git clone https://github.com/あなたのユーザー名/nostr-cli-app.git
cd nostr-cli-app
cargo build --release
Text Only

## 使用方法

鍵ペアの生成

cargo run -- generate-keys
TUIモードで起動

cargo run -- tui
「うぃビームだころせ」効果音を再生

cargo run -- uibeam
