mod commands;
mod tui_app;

use clap::{Arg, ArgAction, Command};
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // コマンドラインの引数を解析
    let matches = Command::new("Nostr CLI")
        .about("Nostr CLI アプリケーション")
        .subcommand_required(true)
        .subcommand(
            Command::new("generate-keys")
                .about("新しい鍵ペアを生成")
                .arg(
                    Arg::new("password")
                        .short('p')
                        .long("password")
                        .help("鍵の暗号化に使用するパスワード")
                        .action(ArgAction::Set),
                ),
        )
        .subcommand(Command::new("show-keys").about("鍵情報を表示"))
        .subcommand(
            Command::new("send")
                .about("ノートを送信")
                .arg(
                    Arg::new("content")
                        .required(true)
                        .help("送信するメッセージの内容"),
                ),
        )
        .subcommand(
            Command::new("show-feed")
                .about("イベントフィードを表示")
                .arg(
                    Arg::new("pubkey")
                        .short('p')
                        .long("pubkey")
                        .help("特定のユーザーのイベントをフィルタリング"),
                )
                .arg(
                    Arg::new("hashtag")
                        .short('t')
                        .long("hashtag")
                        .help("特定のハッシュタグでフィルタリング"),
                )
                .arg(
                    Arg::new("limit")
                        .short('l')
                        .long("limit")
                        .help("取得するイベントの最大数")
                        .value_parser(clap::value_parser!(usize))
                        .default_value("20"),
                ),
        )
        .subcommand(
            Command::new("relay")
                .about("リレーの管理")
                .subcommand(Command::new("list").about("登録されているリレーを一覧表示"))
                .subcommand(
                    Command::new("add")
                        .about("リレーを追加")
                        .arg(
                            Arg::new("url")
                                .required(true)
                                .help("追加するリレーのURL"),
                        ),
                )
                .subcommand(
                    Command::new("remove")
                        .about("リレーを削除")
                        .arg(
                            Arg::new("url")
                                .required(true)
                                .help("削除するリレーのURL"),
                        ),
                ),
        )
        .subcommand(Command::new("tui").about("TUIモードで起動"))
        .subcommand(Command::new("uibeam").about("「うぃビームだころせ」効果音を再生"))
        .get_matches();

    // サブコマンドに応じた処理
    match matches.subcommand() {
        Some(("generate-keys", sub_matches)) => {
            commands::generate_keys(sub_matches)?;
        }
        Some(("show-keys", sub_matches)) => {
            commands::show_keys(sub_matches)?;
        }
        Some(("send", sub_matches)) => {
            commands::send_note(sub_matches).await?;
        }
        Some(("show-feed", sub_matches)) => {
            commands::show_feed(sub_matches).await?;
        }
        Some(("relay", sub_matches)) => match sub_matches.subcommand() {
            Some(("list", list_matches)) => {
                commands::list_relays(list_matches)?;
            }
            Some(("add", add_matches)) => {
                commands::add_relay(add_matches)?;
            }
            Some(("remove", remove_matches)) => {
                commands::remove_relay(remove_matches)?;
            }
            _ => unreachable!(),
        },
        Some(("tui", _)) => {
            tui_app::run_tui().await?;
        }
        Some(("uibeam", sub_matches)) => {
            commands::play_uibeam(sub_matches).await?;
        }
        _ => unreachable!(),
    }

    Ok(())
}

