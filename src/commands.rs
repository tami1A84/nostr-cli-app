use clap::ArgMatches;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{Cursor, Read, Write};
use nostr_sdk::prelude::*;
use ::hex;
use rpassword;
use dirs;
use rodio::{Decoder, OutputStream, Sink};
use reqwest;

// リレー設定の構造体
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct RelayConfig {
    pub relays: Vec<String>,
}

// 新しい鍵ペアを生成する関数
pub fn generate_keys(matches: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
    // パスワードの入力を求める
    let password = if let Some(pass) = matches.get_one::<String>("password") {
        pass.clone()
    } else {
        println!("鍵を暗号化するためのパスワードを入力してください:");
        let password = rpassword::read_password()?;
        println!("確認のためもう一度パスワードを入力してください:");
        let confirm_password = rpassword::read_password()?;

        if password != confirm_password {
            return Err("パスワードが一致しません".into());
        }
        password
    };

    // キーを生成
    let keys = Keys::generate();
    let public_key = keys.public_key();
    let secret_key = keys.secret_key()?;

    // 秘密鍵をHex形式で取得（displayメソッドを使用）
    let secret_key_str = secret_key.display_secret().to_string();

    // 保存データの作成
    let encrypted_data = format!("{{\"secret_key\":\"{}\",\"password\":\"{}\"}}", secret_key_str, password);

    // 保存ディレクトリを作成
    let config_dir = dirs::home_dir()
        .ok_or("ホームディレクトリが見つかりません")?
        .join(".nostr-cli-app");
    fs::create_dir_all(&config_dir)?;

    // 鍵を保存
    let keys_path = config_dir.join("keys.json");
    let mut file = File::create(&keys_path)?;
    file.write_all(encrypted_data.as_bytes())?;

    println!("鍵ペアを生成して保存しました");
    println!("公開鍵: {}", public_key.to_bech32()?);

    if let Some(path) = keys_path.to_str() {
        println!("鍵の保存場所: {}", path);
    }

    Ok(())
}

// 秘密鍵を表示する関数
pub fn show_keys(_matches: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
    println!("鍵を復号化するためのパスワードを入力してください:");
    let password = rpassword::read_password()?;

    let keys = load_keys(&password)?;
    let public_key = keys.public_key();
    let secret_key = keys.secret_key()?;

    println!("公開鍵 (hex): {}", public_key);
    println!("公開鍵 (bech32): {}", public_key.to_bech32()?);
    println!("秘密鍵 (hex): {}", secret_key.display_secret());
    println!("秘密鍵 (bech32): {}", secret_key.to_bech32()?);

    Ok(())
}

// 保存された鍵を読み込む関数
pub fn load_keys(password: &str) -> Result<Keys, Box<dyn std::error::Error>> {
    let keys_path = dirs::home_dir()
        .ok_or("ホームディレクトリが見つかりません")?
        .join(".nostr-cli-app")
        .join("keys.json");

    if !keys_path.exists() {
        return Err(format!("鍵ファイルが見つかりません: {:?}", keys_path).into());
    }

    let encrypted_data = std::fs::read_to_string(&keys_path)?;

    // JSONからデータを解析
    #[derive(Deserialize)]
    struct KeyData {
        secret_key: String,
        password: String,
    }

    let key_data: KeyData = serde_json::from_str(&encrypted_data)?;

    // パスワードの検証
    if key_data.password != password {
        return Err("パスワードが正しくありません".into());
    }

    // 16進数文字列から秘密鍵を生成
    let bytes = hex::decode(&key_data.secret_key)?;
    let secret_key = SecretKey::from_slice(&bytes)?;
    let keys = Keys::new(secret_key);

    Ok(keys)
}

// テキストノートを送信する関数
pub async fn send_note(matches: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
    // 入力内容を取得
    let content = matches.get_one::<String>("content").ok_or("コンテンツが指定されていません")?;

    // パスワードの入力
    println!("鍵を復号化するためのパスワードを入力してください:");
    let password = rpassword::read_password()?;

    // 鍵をロード
    let keys = load_keys(&password)?;

    // クライアントの初期化
    let client = Client::new(&keys);

    // リレーの設定
    let relay_config = load_relays()?;
    if relay_config.relays.is_empty() {
        client.add_relay("wss://yabu.me").await?;
    } else {
        for url in &relay_config.relays {
            client.add_relay(url.clone()).await?;
        }
    }

    // リレーに接続
    client.connect().await;

    // イベントの作成と送信
    let event = EventBuilder::new_text_note(content, Vec::<Tag>::new()).to_event(&keys)?;
    client.send_event(event).await?;

    println!("ノートを送信しました");

    // クライアントをシャットダウン
    client.shutdown().await?;

    Ok(())
}

// イベントフィードを表示する関数
pub async fn show_feed(_matches: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
    // パスワードの入力
    println!("鍵を復号化するためのパスワードを入力してください:");
    let password = rpassword::read_password()?;

    // 鍵をロード
    let keys = load_keys(&password)?;

    // クライアントの初期化
    let client = Client::new(&keys);

    // リレーの設定
    let relay_config = load_relays()?;
    if relay_config.relays.is_empty() {
        client.add_relay("wss://yabu.me").await?;
    } else {
        for url in &relay_config.relays {
            client.add_relay(url.clone()).await?;
        }
    }

    // リレーに接続
    client.connect().await;

    // フィルターの設定
    let filter = Filter::new()
        .kind(Kind::TextNote)
        .limit(20);

    // イベントの取得
    client.subscribe(vec![filter.clone()]).await;
    let _subscription_id = "feed";

    println!("イベントを取得中...");

    // 最大20件のイベントを表示
    let mut events = Vec::new();
    let start_time = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(10);

    while events.len() < 20 && start_time.elapsed() < timeout {
        if let Ok(notification) = tokio::time::timeout(
            std::time::Duration::from_secs(1),
            client.notifications().recv(),
        ).await {
            if let Ok(RelayPoolNotification::Event { event, .. }) = notification {
                events.push(event);
            }
        }
    }

    // サブスクリプションを解除
    let _ = client.unsubscribe().await;

    // イベントを時系列順に並べ替え
    events.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    // イベントの表示
    println!("{}件のイベントを取得しました", events.len());
    for event in &events {
        let pubkey = event.pubkey.to_bech32()?;
        println!("-----------------------------------");
        println!("アカウント: {}", pubkey);
        println!("時間: {}", event.created_at);
        println!("内容: {}", event.content);
    }

    // クライアントをシャットダウン
    client.shutdown().await?;

    Ok(())
}

// リレーを追加する関数
pub fn add_relay(matches: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
    let url = matches.get_one::<String>("url").ok_or("URLが指定されていません")?;

    // 設定を読み込み
    let mut config = load_relays()?;

    // リレーが既に存在するか確認
    if config.relays.contains(url) {
        println!("リレー {} は既に登録されています", url);
        return Ok(());
    }

    // リレーを追加
    config.relays.push(url.clone());

    // 設定を保存
    save_relays(&config)?;

    println!("リレー {} を追加しました", url);
    Ok(())
}

// リレーを削除する関数
pub fn remove_relay(matches: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
    let url = matches.get_one::<String>("url").ok_or("URLが指定されていません")?;

    // 設定を読み込み
    let mut config = load_relays()?;

    // リレーが存在するか確認
    let index = config.relays.iter().position(|r| r == url);
    if let Some(idx) = index {
        // リレーを削除
        config.relays.remove(idx);

        // 設定を保存
        save_relays(&config)?;
        println!("リレー {} を削除しました", url);
    } else {
        println!("リレー {} は登録されていません", url);
    }

    Ok(())
}

// リレーリストを表示する関数
pub fn list_relays(_: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
    // 設定を読み込み
    let config = load_relays()?;

    if config.relays.is_empty() {
        println!("登録されているリレーはありません");
    } else {
        println!("登録されているリレー一覧:");
        for (i, url) in config.relays.iter().enumerate() {
            println!("{}. {}", i + 1, url);
        }
    }

    Ok(())
}

// リレー設定を読み込む関数
pub fn load_relays() -> Result<RelayConfig, Box<dyn std::error::Error>> {
    let config_dir = dirs::home_dir()
        .ok_or("ホームディレクトリが見つかりません")?
        .join(".nostr-cli-app");

    let relays_path = config_dir.join("relays.json");

    if !relays_path.exists() {
        return Ok(RelayConfig::default());
    }

    let mut file = File::open(relays_path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    let config: RelayConfig = serde_json::from_str(&contents)?;
    Ok(config)
}

// リレー設定を保存する関数
fn save_relays(config: &RelayConfig) -> Result<(), Box<dyn std::error::Error>> {
    let config_dir = dirs::home_dir()
        .ok_or("ホームディレクトリが見つかりません")?
        .join(".nostr-cli-app");

    fs::create_dir_all(&config_dir)?;

    let relays_path = config_dir.join("relays.json");
    let contents = serde_json::to_string_pretty(config)?;

    let mut file = File::create(relays_path)?;
    file.write_all(contents.as_bytes())?;

    Ok(())
}

// 「うぃビームだころせ」効果音を再生する関数
pub async fn play_uibeam(_matches: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
    println!("「うぃビームだころせ」を再生します...");

    // 音声ファイルのURL
    let url = "https://leiros.cloudfree.jp/usbtn/sound/uibeamdakorose.mp3";

    // URLからのリクエストにUser-Agentを追加
    println!("音声ファイルをダウンロード中...");
    let client = reqwest::Client::new();
    let response = client.get(url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
        .header("Referer", "https://leiros.cloudfree.jp/usbtn/usbtn.html")
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(format!("ダウンロード失敗: HTTP ステータス {}", response.status()).into());
    }

    // 以下は元のコード
    let bytes = response.bytes().await?;
    println!("ダウンロード完了: {}バイト", bytes.len());

    if bytes.len() < 100 {
        return Err("ダウンロードされたデータが小さすぎます".into());
    }

    // メモリバッファにデータを読み込む
    let cursor = Cursor::new(bytes);

    // 出力デバイスを取得
    println!("オーディオデバイスを初期化中...");
    let (_stream, stream_handle) = OutputStream::try_default()?;
    let sink = Sink::try_new(&stream_handle)?;

    // データをデコードして再生
    println!("音声データをデコード中...");
    let source = match Decoder::new(cursor) {
        Ok(s) => s,
        Err(e) => {
            println!("デコードエラー詳細: {:?}", e);
            return Err("音声データのデコードに失敗しました。MP3コーデッ���が利用可能か確認してください。".into());
        }
    };

    sink.append(source);

    println!("再生中...");

    // 再生完了まで待機
    sink.sleep_until_end();

    println!("再生完了！");
    Ok(())
}


