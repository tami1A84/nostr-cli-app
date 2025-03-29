// src/tui_app.rsの先頭付近に追加
#![allow(dead_code)]

use crossterm::{
    event::{self, Event as CrosstermEvent, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen, Clear, ClearType},
    cursor::{Hide, Show},
};
use nostr_sdk::prelude::*;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect, Alignment},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, BorderType, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};
use std::{
    io::{self, Write},
    time::{Duration, Instant},
};
use crate::commands::{load_keys, load_relays};
use chrono::{DateTime, Utc, FixedOffset};
use unicode_width::UnicodeWidthStr;


// Chicago風フォント表現用の定数
const MAC_APPLE_LOGO: &str = "⌘"; // Commandキーのシンボル
const MAC_FOLDER: &str = "📁";
const MAC_DOCUMENT: &str = "📄";
const MAC_NOTE: &str = "📝";
const MAC_CHECKMARK: &str = "✓";
const MAC_DIVIDER: &str = "━━━━━━━━━━━━━━━━━━━━━━━━";
const MAC_HAPPY_MAC: &str = "🙂"; // ハッピーマック（実際のアイコンに近いもの）

// 初代Mac風パターン（繰り返し使用可能）
const MAC_PATTERN1: &str = "■ □ ■ □ ■ □ ■ □ ■ □ ■ □";
const MAC_PATTERN2: &str = "□ ■ □ ■ □ ■ □ ■ □ ■ □ ■";

// 電卓関連の定数
const CALC_CLEAR: &str = "C";
const CALC_DIVIDE: &str = "÷";
const CALC_MULTIPLY: &str = "×";
const CALC_MINUS: &str = "−";
const CALC_PLUS: &str = "+";
const CALC_EQUAL: &str = "=";
const CALC_DOT: &str = ".";

// InputModeにPartialEqを追加
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputMode {
    Normal,
    Editing,
}

pub struct App {
    pub input: String,
    pub input_mode: InputMode,
    pub events: Vec<nostr_sdk::Event>,
    pub list_state: ListState,
    pub active_tab: usize,
    pub status: String,
    pub client: Option<Client>,
    pub my_public_key: Option<String>,
    pub keys: Option<Keys>,
    pub message_to_send: Option<String>,
    pub detail_mode: bool,
    pub detail_scroll: u16, // 詳細表示のスクロール位置
    pub show_about: bool,   // About画面表示フラグ
    pub show_calculator: bool,       // 電卓表示フラグ
    pub calculator_display: String,  // 電卓の表示値
    pub calculator_value: f64,       // 計算中の値
    pub calculator_op: Option<char>, // 演算子（+,-,*,/）
    pub calculator_new_input: bool,  // 新しい入力開始フラグ
    
}

impl Default for App {
    fn default() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        Self {
            input: String::new(),
            input_mode: InputMode::Normal,
            events: Vec::new(),
            list_state,
            active_tab: 0,
            status: String::from("起動しました"),
            client: None,
            my_public_key: None,
            keys: None,
            message_to_send: None,
            detail_mode: false,
            detail_scroll: 0, // 初期値は0
            show_about: false,
            show_calculator: false,
            calculator_display: "0".to_string(),
            calculator_value: 0.0,
            calculator_op: None,
            calculator_new_input: true,
        }
    }
}

impl App {
    pub fn new() -> Self {
        Self::default()
    }

    // About画面の表示・非表示
    pub fn toggle_about(&mut self) {
        self.show_about = !self.show_about;
    }

    // 電卓の表示・非表示切り替え
    pub fn toggle_calculator(&mut self) {
        self.show_calculator = !self.show_calculator;
        if self.show_calculator {
            // 電卓表示時にはリセット
            self.calculator_display = "0".to_string();
            self.calculator_value = 0.0;
            self.calculator_op = None;
            self.calculator_new_input = true;
        }
    }

    // 電卓の数字入力処理
    pub fn calculator_input_digit(&mut self, digit: char) {
        if self.calculator_new_input {
            self.calculator_display = digit.to_string();
            self.calculator_new_input = false;
        } else {
            // 桁数制限（初代Mac電卓は9桁まで）
            if self.calculator_display.len() < 9 {
                if self.calculator_display == "0" {
                    self.calculator_display = digit.to_string();
                } else {
                    self.calculator_display.push(digit);
                }
            }
        }
    }

    // 電卓の小数点入力
    pub fn calculator_input_dot(&mut self) {
        if self.calculator_new_input {
            self.calculator_display = "0.".to_string();
            self.calculator_new_input = false;
        } else if !self.calculator_display.contains('.') {
            self.calculator_display.push('.');
        }
    }

    // 電卓のクリア処理
    pub fn calculator_clear(&mut self) {
        self.calculator_display = "0".to_string();
        self.calculator_value = 0.0;
        self.calculator_op = None;
        self.calculator_new_input = true;
    }

    // 電卓の演算子処理
    pub fn calculator_operator(&mut self, op: char) {
        // 現在の表示値を取得
        let current_value = self.calculator_display.parse::<f64>().unwrap_or(0.0);

        // 前回の演算子がある場合は計算を実行
        if let Some(prev_op) = self.calculator_op {
            let result = match prev_op {
                '+' => self.calculator_value + current_value,
                '-' => self.calculator_value - current_value,
                '*' => self.calculator_value * current_value,
                '/' => {
                    if current_value != 0.0 {
                        self.calculator_value / current_value
                    } else {
                        // 0除算エラー
                        self.calculator_display = "Error".to_string();
                        self.calculator_new_input = true;
                        return;
                    }
                },
                _ => current_value,
            };

            // 結果を表示（初代Macの電卓風に整形）
            self.calculator_display = format_calculator_result(result);
            self.calculator_value = result;
        } else {
            // 初回の演算子入力時は現在値を保存
            self.calculator_value = current_value;
        }

        // 新しい演算子を設定
        self.calculator_op = Some(op);
        self.calculator_new_input = true;
    }

    // =ボタン（計算結果表示）
    pub fn calculator_equals(&mut self) {
        if let Some(op) = self.calculator_op {
            let current_value = self.calculator_display.parse::<f64>().unwrap_or(0.0);
            let result = match op {
                '+' => self.calculator_value + current_value,
                '-' => self.calculator_value - current_value,
                '*' => self.calculator_value * current_value,
                '/' => {
                    if current_value != 0.0 {
                        self.calculator_value / current_value
                    } else {
                        // 0除算エラー
                        self.calculator_display = "Error".to_string();
                        self.calculator_new_input = true;
                        return;
                    }
                },
                _ => current_value,
            };

            // 結果を表示（初代Macの電卓風に整形）
            self.calculator_display = format_calculator_result(result);
            self.calculator_value = result;
            self.calculator_op = None;
            self.calculator_new_input = true;
        }
    }

    pub fn toggle_input_mode(&mut self) {
        self.input_mode = match self.input_mode {
            InputMode::Normal => InputMode::Editing,
            InputMode::Editing => InputMode::Normal,
        };
    }

    // 詳細表示モードの切り替え - スクロール位置もリセット
    pub fn toggle_detail_mode(&mut self) {
        self.detail_mode = !self.detail_mode;
        if self.detail_mode {
            self.detail_scroll = 0; // 詳細表示に入るたびスクロール位置をリセット
        }
    }

    // 詳細表示時のスクロール - 上
    pub fn detail_scroll_up(&mut self) {
        if self.detail_scroll > 0 {
            self.detail_scroll -= 1;
        }
    }

    // 詳細表示時のスクロール - 下
    pub fn detail_scroll_down(&mut self) {
        self.detail_scroll += 1;
    }

    // ページ単位のスクロール - 上
    pub fn detail_page_up(&mut self) {
        if self.detail_scroll >= 5 {
            self.detail_scroll -= 5;
        } else {
            self.detail_scroll = 0;
        }
    }

    // ページ単位のスクロール - 下
    pub fn detail_page_down(&mut self) {
        self.detail_scroll += 5;
    }

    // 上にスクロール
    pub fn previous(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    0
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    // 下にスクロール
    pub fn next(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.events.len().saturating_sub(1) {
                    self.events.len().saturating_sub(1)
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    // ページ上
    pub fn page_up(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => {
                if i < 5 {
                    0
                } else {
                    i - 5
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    // ページ下
    pub fn page_down(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => {
                if i + 5 >= self.events.len() {
                    self.events.len().saturating_sub(1)
                } else {
                    i + 5
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    // 先頭へ
    pub fn home(&mut self) {
        self.list_state.select(Some(0));
    }

    // 末尾へ
    pub fn end(&mut self) {
        if !self.events.is_empty() {
            self.list_state.select(Some(self.events.len() - 1));
        }
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> bool {
        // 電卓表示中の処理
        if self.show_calculator {
            match key.code {
                // 電卓を閉じる
                KeyCode::Esc | KeyCode::Char('q') => {
                    self.show_calculator = false;
                    return true;
                }
                // 数字入力
                KeyCode::Char('0') | KeyCode::Char('1') | KeyCode::Char('2') |
                KeyCode::Char('3') | KeyCode::Char('4') | KeyCode::Char('5') |
                KeyCode::Char('6') | KeyCode::Char('7') | KeyCode::Char('8') |
                KeyCode::Char('9') => {
                    if let KeyCode::Char(digit) = key.code {
                        self.calculator_input_digit(digit);
                    }
                    return true;
                }
                // 小数点
                KeyCode::Char('.') => {
                    self.calculator_input_dot();
                    return true;
                }
                // 演算子
                KeyCode::Char('+') => {
                    self.calculator_operator('+');
                    return true;
                }
                KeyCode::Char('-') => {
                    self.calculator_operator('-');
                    return true;
                }
                KeyCode::Char('*') => {
                    self.calculator_operator('*');
                    return true;
                }
                KeyCode::Char('/') => {
                    self.calculator_operator('/');
                    return true;
                }
                // イコール
                KeyCode::Char('=') | KeyCode::Enter => {
                    self.calculator_equals();
                    return true;
                }
                // クリア
                KeyCode::Char('c') => {
                    self.calculator_clear();
                    return true;
                }
                _ => return true, // 他のキーは無視
            }
        }

        // About画面表示中の処理
        if self.show_about {
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    self.show_about = false;
                    return true;
                }
                _ => return true,
            }
        }

        match self.input_mode {
            InputMode::Normal => {
                if self.detail_mode {
                    // 詳細表示モード中
                    match key.code {
                        KeyCode::Esc | KeyCode::Char('q') => {
                            self.detail_mode = false; // 詳細表示を閉じる
                            self.detail_scroll = 0;   // スクロール位置リセット
                            return true;
                        }
                        KeyCode::Up => {
                            self.detail_scroll_up();
                            return true;
                        }
                        KeyCode::Down => {
                            self.detail_scroll_down();
                            return true;
                        }
                        KeyCode::PageUp => {
                            self.detail_page_up();
                            return true;
                        }
                        KeyCode::PageDown => {
                            self.detail_page_down();
                            return true;
                        }
                        KeyCode::Home => {
                            self.detail_scroll = 0;
                            return true;
                        }
                        KeyCode::End => {
                            // 特に大きな値を設定 - 実際のスクロール最大値は表示時に制限される
                            self.detail_scroll = 1000;
                            return true;
                        }
                        _ => return true, // 他のキーは無視
                    }
                }

                // 通常モード
                match key.code {
                    KeyCode::Char('q') => return false,
                    KeyCode::Char('i') => self.toggle_input_mode(),
                    KeyCode::Char('r') => self.status = "イベントを更新中...".to_string(),
                    KeyCode::Char('a') => self.toggle_about(), // About画面表示
                    KeyCode::Char('s') => self.toggle_calculator(), // cからsキーに変更
                    KeyCode::Tab => {
                        self.active_tab = (self.active_tab + 1) % 2;
                        // 作成画面に切り替わったら自動で編集モードに
                        if self.active_tab == 1 {
                            self.input_mode = InputMode::Editing;
                        }
                    }
                    KeyCode::Enter => {
                        // Enterで詳細表示モードに
                        if !self.events.is_empty() && self.active_tab == 0 {
                            self.toggle_detail_mode();
                        }
                    }
                    KeyCode::Up => self.previous(),
                    KeyCode::Down => self.next(),
                    KeyCode::Home => self.home(),
                    KeyCode::End => self.end(),
                    KeyCode::PageUp => self.page_up(),
                    KeyCode::PageDown => self.page_down(),
                    _ => {}
                }
            }
            InputMode::Editing => match key.code {
                KeyCode::Enter => {
                    self.send_message();
                }
                KeyCode::Char(c) => {
                    self.input.push(c);
                }
                KeyCode::Backspace => {
                    self.input.pop();
                }
                KeyCode::Esc => {
                    self.toggle_input_mode();
                }
                _ => {}
            },
        }
        true
    }

    pub fn send_message(&mut self) {
        if self.input.is_empty() {
            return;
        }

        self.message_to_send = Some(self.input.clone());
        self.status = "メッセージを送信中...".to_string();

        // 既存の送信処理...
        // ここに自分のバックエンド処理があると仮定

        // 送信成功処理
        self.status = "メッセージを送信し、イベントを取得しました".to_string();
        self.input.clear();
        self.input_mode = InputMode::Normal;

        // 投稿作成画面からイベントリスト画面に自動で戻る
        self.active_tab = 0;

        // 最新のイベントを選択
        if !self.events.is_empty() {
            self.list_state.select(Some(0));
        }
    }

    // タブ変更時のヘルパーメソッド（必要に応じて使用）
    pub fn change_tab(&mut self, tab_index: usize) {
        self.active_tab = tab_index;

        // 投稿作成タブに切り替わったら自動で編集モードに
        if tab_index == 1 {
            self.input_mode = InputMode::Editing;
        } else {
            self.input_mode = InputMode::Normal;
        }
    }
}



// 電卓の結果を初代Mac風に整形する関数
fn format_calculator_result(value: f64) -> String {
    if value.is_infinite() || value.is_nan() {
        return "Error".to_string();
    }

    // 整数部か小数部か判断して適切にフォーマット
    if value == (value as i64) as f64 {
        // 整数値の場合
        format!("{}", value as i64)
    } else {
        // 小数値の場合（初代Mac電卓は小数点以下最大9桁）
        let formatted = format!("{:.9}", value);
        // 末尾の0を削除
        formatted.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

// パスワード入力処理
fn read_password() -> io::Result<String> {
    print!("鍵を復号化するためのパスワードを入力してください: ");
    io::stdout().flush()?;

    match rpassword::read_password() {
        Ok(pw) => Ok(pw),
        Err(e) => Err(io::Error::new(io::ErrorKind::Other, e.to_string()))
    }
}

// イベントの取得 - nostr-sdk APIの更新に対応
async fn fetch_events(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(client) = &app.client {
        let filter = Filter::new()
            .limit(100)
            .kinds(vec![Kind::TextNote]);

        let events = client.get_events_of(vec![filter], None).await?;

        // 時間順（降順）に並び替え
        let mut sorted_events = events;
        sorted_events.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        app.events = sorted_events;
        app.status = format!("{}件のイベントを取得しました", app.events.len());
    }

    Ok(())
}

// メッセージ送信 - nostr-sdk APIの更新に対応
async fn send_message(app: &mut App, message: &str) -> Result<(), Box<dyn std::error::Error>> {
    if let (Some(client), Some(_keys)) = (&app.client, &app.keys) {
        let event_id = client.publish_text_note(message.to_string(), Vec::<Tag>::new()).await?;
        app.status = format!("メッセージを送信しました: {}", event_id);
    } else {
        app.status = "クライアントまたは鍵が初期化されていません".to_string();
    }

    Ok(())
}

fn render_compose_mac_style(f: &mut Frame, app: &App, area: Rect) {
    let title = format!("{} 投稿作成", MAC_NOTE);
    let window = mac_window_block(&title);

    f.render_widget(window.clone(), area);
    let inner_area = window.inner(area);

    // 投稿作成エリアを描画するテキスト要素を準備
    let mut text = Vec::new();

    // 公開鍵情報
    text.push(Line::from(vec![
        Span::styled("現在、以下の公開鍵として投稿します：", 
                  Style::default().fg(Color::Black))
    ]));

    // 公開鍵表示
    let pubkey_display = match &app.my_public_key {
        Some(pk) => pk.clone(),
        None => "公開鍵が読み込まれていません".to_string(),
    };

    text.push(Line::from(vec![
        Span::styled(pubkey_display, 
                  Style::default().fg(Color::Black).add_modifier(Modifier::BOLD))
    ]));

    // 境界線（幅を広げる）
    // 現在の短い区切り線の代わりに画面幅いっぱいの区切り線を使用
    let divider = "─".repeat((inner_area.width as usize).saturating_sub(2));
    text.push(Line::from(""));  // 空行
    text.push(Line::from(divider));

    // 入力欄のタイトル
    text.push(Line::from(vec![
        Span::styled("メッセージ内容：", 
                  Style::default().fg(Color::Black).add_modifier(Modifier::BOLD))
    ]));

    // 入力内容を表示
    let input_style = Style::default().fg(Color::Black);

    // 現在の入力内容
    let input_content = if app.input.is_empty() {
        "".to_string()
    } else {
        app.input.clone()
    };

    // 改行で分割して表示
    for line in input_content.split('\n') {
        text.push(Line::from(vec![
            Span::styled(line, input_style)
        ]));
    }

    // 「編集モード」表示を削除

    // パラグラフとして描画
    let paragraph = Paragraph::new(text)
        .style(Style::default().bg(Color::White).fg(Color::Black))
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, inner_area);

    // 編集モードの場合はカーソルを表示
    if let InputMode::Editing = app.input_mode {
        // カーソル位置の計算を修正（数値を調整）
        let base_lines = 5; // 公開鍵表示 + 空行 + 区切り線 + タイトル行

        // 入力済み行数とカーソル位置を計算
        let input_lines: Vec<&str> = app.input.split('\n').collect();
        let cursor_y_offset = input_lines.len().saturating_sub(1);
        let last_line = input_lines.last().unwrap_or(&"");

        // カーソル位置を設定（Y位置を調整）
        f.set_cursor(
            inner_area.x + last_line.width() as u16,
            inner_area.y + base_lines as u16 + cursor_y_offset as u16
        );
    }
}




// About画面を描画 - 新しいデザイン
fn render_about_screen(f: &mut Frame, _app: &App) {
    let area = f.size();

    // Aboutウィンドウのサイズ
    let about_width = 60;
    let about_height = 20;

    // 画面中央に配置
    let about_x = (area.width.saturating_sub(about_width)) / 2;
    let about_y = (area.height.saturating_sub(about_height)) / 2;

    let about_area = Rect::new(
        area.x + about_x,
        area.y + about_y,
        about_width.min(area.width),
        about_height.min(area.height)
    );

    // 影の位置
    let shadow_area = Rect::new(
        about_area.x + 1,
        about_area.y + 1,
        about_width.min(area.width),
        about_height.min(area.height)
    );

    // 影を描画
    let shadow = Block::default()
        .style(Style::default().bg(Color::DarkGray));

    f.render_widget(shadow, shadow_area);

    // Aboutウィンドウ
    let about_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Black))
        .style(Style::default().bg(Color::White).fg(Color::Black));

    f.render_widget(about_block.clone(), about_area);

    // コンテンツエリア
    let inner_area = about_block.inner(about_area);

    // アプリ情報を表示
    let about_text = vec![
        Line::from(vec![
            Span::raw("🙂 "),
            Span::styled(
                "About Nostr Macintosh Client",
                Style::default().fg(Color::Black).add_modifier(Modifier::BOLD)
            )
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Nostr Macintosh Client",
            Style::default().fg(Color::Black).add_modifier(Modifier::BOLD)
        )),
        Line::from(Span::styled(
            "Version 1.0.0",
            Style::default().fg(Color::Black)
        )),
        Line::from(""),
        Line::from(Span::raw("━━━━━━━━━━━━━━━━━━━━━━━━")),
        Line::from(""),
        Line::from(Span::styled(
            "初代Macintosh風のNostrクライアント",
            Style::default().fg(Color::Black)
        )),
        Line::from(Span::styled(
            "Rust/ratatuiで実装",
            Style::default().fg(Color::Black)
        )),
        Line::from(""),
        // チェッカーボードパターン（3行）
        Line::from("■ □ ■ □ ■ □ ■ □ ■ □"),
        Line::from("□ ■ □ ■ □ ■ □ ■ □ ■"),
        Line::from("■ □ ■ □ ■ □ ■ □ ■ □"),
        Line::from(""),
        Line::from(Span::styled(
            "© 2025 Nostr Macintosh Team",
            Style::default().fg(Color::Black)
        )),
        Line::from(""),
        Line::from(Span::styled(
            "ESC または q キーで閉じる",
            Style::default().fg(Color::Black)
        )),
    ];

    let about_paragraph = Paragraph::new(about_text)
        .style(Style::default().bg(Color::White).fg(Color::Black))
        .alignment(Alignment::Center);

    f.render_widget(about_paragraph, inner_area);
}



// 詳細表示
fn render_event_detail_mac_style(f: &mut Frame, app: &App, area: Rect) {
    if let Some(selected) = app.list_state.selected() {
        if selected < app.events.len() {
            let event = &app.events[selected];

            // Mac風ダイアログウィンドウ
            let dialog_width = area.width.saturating_sub(10).min(80).max(60);
            let dialog_height = area.height.saturating_sub(8).min(30).max(20);

            let dialog_x = area.width.saturating_sub(dialog_width) / 2;
            let dialog_y = area.height.saturating_sub(dialog_height) / 2;

            let dialog_area = Rect::new(
                area.x.saturating_add(dialog_x),
                area.y.saturating_add(dialog_y),
                dialog_width,
                dialog_height
            );

            // 影を付ける (初代Macの特徴)
            let shadow_area = Rect::new(
                dialog_area.x.saturating_add(1),
                dialog_area.y.saturating_add(1),
                dialog_width,
                dialog_height
            );

            let shadow = Block::default()
                .style(Style::default().bg(Color::DarkGray));

            f.render_widget(shadow, shadow_area);

            // ダイアログ本体
            let dialog_title = format!("{} Event Detail" ,MAC_DOCUMENT ); 
            let dialog_block = Block::default()
                .title(Span::styled(
                    format!(" {} ", dialog_title),
                    Style::default().fg(Color::Black).bg(Color::White).add_modifier(Modifier::BOLD)
                ))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Black))
                .style(Style::default().bg(Color::White).fg(Color::Black));

            f.render_widget(dialog_block.clone(), dialog_area);
            let inner_area = dialog_block.inner(dialog_area);

            // 左右に分割して情報を配置する
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(inner_area.height.min(4)), // メタデータ用
                    Constraint::Min(1),    // 内容表示用
                    Constraint::Length(1), // 操作説明用
                ])
                .split(inner_area);

            // メタデータ領域
            let metadata_area = chunks[0];

            // 各メタデータの行を作成
            let mut metadata_text = Vec::new();

            // 公開鍵
            let pubkey_str = match event.pubkey.to_bech32() {
                Ok(pk) => {
                    if pk.len() > 20 {
                        let start = pk.get(0..10).unwrap_or("").to_string();
                        let end = if pk.len() >= 10 {
                            pk.get(pk.len().saturating_sub(10)..).unwrap_or("")
                        } else {
                            ""
                        };
                        format!("{}...{}", start, end)
                    } else {
                        pk
                    }
                },
                Err(_) => "不明な公開鍵".to_string(),
            };

            metadata_text.push(Line::from(vec![
                Span::styled("公開鍵: ", Style::default().fg(Color::Black).add_modifier(Modifier::BOLD)),
                Span::raw(pubkey_str),
            ]));

            // 日時 (JSTに変換)
let timestamp_value = event.created_at.as_u64() as i64;
let utc_date = DateTime::<Utc>::from_timestamp(timestamp_value, 0)
    .unwrap_or_default();

// UTC+9時間（日本時間）に変換
let jst_offset = FixedOffset::east_opt(9 * 3600).unwrap(); // 9時間=32400秒
let jst_date = utc_date.with_timezone(&jst_offset);

// 日本時間でフォーマット
let date = jst_date.format("%Y-%m-%d %H:%M:%S (JST)").to_string();

metadata_text.push(Line::from(vec![
    Span::styled("日時: ", Style::default().fg(Color::Black).add_modifier(Modifier::BOLD)),
    Span::raw(date),
]));


            // ID - 安全に処理
            let id_hex = event.id.to_hex();
            let short_id = if id_hex.len() > 16 {
                let start = id_hex.get(0..8).unwrap_or("");
                let end = if id_hex.len() >= 8 {
                    id_hex.get(id_hex.len().saturating_sub(8)..).unwrap_or("")
                } else {
                    ""
                };
                format!("{}...{}", start, end)
            } else {
                id_hex
            };

            metadata_text.push(Line::from(vec![
                Span::styled("ID: ", Style::default().fg(Color::Black).add_modifier(Modifier::BOLD)),
                Span::raw(short_id),
            ]));

            // 署名 - 安全に処理
            let sig = event.sig.to_string();
            let short_sig = if sig.len() > 16 {
                let start = sig.get(0..8).unwrap_or("");
                let end = if sig.len() >= 8 {
                    sig.get(sig.len().saturating_sub(8)..).unwrap_or("")
                } else {
                    ""
                };
                format!("{}...{}", start, end)
            } else {
                sig
            };

            metadata_text.push(Line::from(vec![
                Span::styled("署名: ", Style::default().fg(Color::Black).add_modifier(Modifier::BOLD)),
                Span::raw(short_sig),
            ]));

            let metadata_paragraph = Paragraph::new(metadata_text)
                .style(Style::default().bg(Color::White).fg(Color::Black));

            f.render_widget(metadata_paragraph, metadata_area);

            // コンテンツ領域 (メインの内容表示)
            let content_area = chunks[1];

            // 区切り線を動的に生成 - ウィンドウ幅に合わせる
            let divider_char = '─'; // または MAC_DIVIDER に含まれる文字
            let divider_count = content_area.width as usize;
            let divider_str: String = std::iter::repeat(divider_char).take(divider_count).collect();
            let divider = Line::from(divider_str);

            // 改行で分割した内容
            let content_lines: Vec<&str> = event.content.split('\n').collect();

            // スクロールに対応して表示範囲を制限 - 型の修正
            let max_visible_lines = content_area.height.saturating_sub(2) as usize; // ヘッダー分を引く

            // 型の不一致を修正
            let max_scroll = content_lines.len().saturating_sub(1);
            let max_scroll_u16 = if max_scroll > u16::MAX as usize {
                u16::MAX
            } else {
                max_scroll as u16
            };

            let start_line = app.detail_scroll.min(max_scroll_u16) as usize;

            let mut text = vec![
                Line::from(Span::styled("内容:", Style::default().fg(Color::Black).add_modifier(Modifier::BOLD))),
                divider.clone(),
            ];

            for line in content_lines.iter().skip(start_line).take(max_visible_lines) {
                text.push(Line::from(Span::raw(line.to_string())));
            }

            // スクロール情報 - 安全に計算
            if content_lines.len() > max_visible_lines {
                let scroll_percent = if content_lines.len() > 0 {
                    (start_line as f64 / content_lines.len().saturating_sub(1).max(1) as f64 * 100.0).min(100.0) as u32
                } else {
                    0
                };

                let scroll_info = format!(
                    "[{}/{}行目 ({}%) 表示中]",
                    start_line.saturating_add(1).min(content_lines.len()),
                    content_lines.len(),
                    scroll_percent
                );

                text.push(Line::from(Span::styled(
                    scroll_info,
                    Style::default().fg(Color::Black).add_modifier(Modifier::ITALIC)
                )));
            }

            let paragraph = Paragraph::new(text)
                .style(Style::default().bg(Color::White).fg(Color::Black))
                .wrap(Wrap { trim: true });

            f.render_widget(paragraph, content_area);

            // 操作説明
            f.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    "↑↓: スクロール | Esc: 戻る",
                    Style::default().fg(Color::Black).add_modifier(Modifier::BOLD)
                ))),
                chunks[2]
            );
        }
    }
}





// イベントリスト表示
fn render_events_mac_style(f: &mut Frame, app: &App, area: Rect) {
    if app.detail_mode {
        // 詳細表示モード - Mac風ダイアログとして表示
        render_event_detail_mac_style(f, app, area);
        return;
    }

    // 通常表示モード
    // 修正後（イベント数を表示しない場合）
let title = format!("{} Events", MAC_FOLDER);

    let window = mac_window_block(&title);

    // 白背景に設定
    f.render_widget(window.clone(), area);
    let inner_area = window.inner(area);

    if app.events.is_empty() {
        let message = format!("{} No events. Press R to refresh.", MAC_HAPPY_MAC);
        let paragraph = Paragraph::new(message)
            .style(Style::default()
                .bg(Color::White)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD)) // Chicago風
            .alignment(Alignment::Center);
        f.render_widget(paragraph, inner_area);
        return;
    }

    // イベントリスト項目を作成
    let mut list_items = Vec::new();
    for event in &app.events {
        // 公開鍵表示（短縮形）
        let pubkey = match event.pubkey.to_bech32() {
            Ok(pk) => format!("npub...{}", &pk[pk.len()-8..]),
            Err(_) => "unknown".to_string(),
        };

        // 日時表示 - Macスタイルの短い形式 (JSTに変換)
let utc_date = DateTime::<Utc>::from_timestamp(event.created_at.as_u64() as i64, 0)
    .unwrap_or_default();
let jst_offset = FixedOffset::east_opt(9 * 3600).unwrap();
let jst_date = utc_date.with_timezone(&jst_offset);
let date = jst_date.format("%m/%d/%y %H:%M").to_string();


        // コンテンツのプレビュー - スマート切り捨て処理
let content_preview = smart_truncate(&event.content, 137);


        // Mac風のリストアイテム (Chicago風アイコン使用)
        let item = ListItem::new(vec![
            Line::from(vec![
                Span::styled(format!("{} {} - ",MAC_DOCUMENT,  pubkey), 
                            Style::default().fg(Color::Black).add_modifier(Modifier::BOLD)), // Chicago風
                Span::styled(date, Style::default().fg(Color::Black)),
            ]),
            Line::from(Span::styled(content_preview, 
                    Style::default().fg(Color::Black))),
            Line::from(""),  // 項目間の空白行
        ]);

        list_items.push(item);
    }

    // ハイライト用の文字列を変数に格納し、ライフタイムを延長
    let highlight_prefix = format!("{} ", MAC_CHECKMARK);

    let events_list = List::new(list_items)
        .style(Style::default().bg(Color::White).fg(Color::Black))
        .highlight_style(
            Style::default()
                .bg(Color::Black)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD)) // 選択項目はChicago風に強調
        .highlight_symbol(&highlight_prefix);

    f.render_stateful_widget(events_list, inner_area, &mut app.list_state.clone());
}

// スマートな切り捨て処理 - 飽和演算使用
fn smart_truncate(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_string();
    }

    // 指定文字数まで切り取る
    let chars: Vec<char> = text.chars().collect();
    let mut truncated: String = chars.iter().take(limit).collect();

    // 最後の単語を完全に含めるか切る判断 - 飽和演算使用
    if let Some(last_space) = truncated.rfind(' ') {
        // saturating_subを使ってオーバーフロー防止
        if limit.saturating_sub(last_space) < 20 {
            truncated = truncated[0..last_space].to_string();
        }
    }

    format!("{}...", truncated)
}



pub async fn run_tui() -> io::Result<()> {
    // 初期化
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, Hide)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let mut app = App::new();
    app.status = "アプリケーションを起動しました。パスワードを入力してください...".to_string();

    terminal.draw(|f| ui(f, &app))?;

    // パスワード入力のために一時的にraw modeを無効化し、通常画面に戻る
    execute!(terminal.backend_mut(), LeaveAlternateScreen, Show)?;
    disable_raw_mode()?;

    // パスワード入力
    let password = match read_password() {
        Ok(pw) => pw,
        Err(e) => {
            return Err(io::Error::new(io::ErrorKind::Other, format!("パスワード入力エラー: {}", e)));
        }
    };

    // TUIに戻る前に状態をクリーンにする
    enable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        Clear(ClearType::All),
        EnterAlternateScreen,
        Hide
    )?;
    terminal.clear()?; // 再度クリア

    app.status = "パスワードを受け付けました。鍵を復号化しています...".to_string();
    terminal.draw(|f| ui(f, &app))?;

    let keys = match load_keys(&password) {
        Ok(k) => k,
        Err(e) => {
            app.status = format!("鍵の読み込みに失敗: {}", e);
            terminal.draw(|f| ui(f, &app))?;
            std::thread::sleep(std::time::Duration::from_secs(3));

            disable_raw_mode()?;
            execute!(terminal.backend_mut(), LeaveAlternateScreen, Show)?;
            return Err(io::Error::new(io::ErrorKind::Other, e.to_string()));
        }
    };

    app.my_public_key = match keys.public_key().to_bech32() {
        Ok(pk) => Some(pk),
        Err(e) => {
            app.status = format!("公開鍵の変換に失敗: {}", e);
            terminal.draw(|f| ui(f, &app))?;
            std::thread::sleep(std::time::Duration::from_secs(3));
            return Err(io::Error::new(io::ErrorKind::Other, e.to_string()));
        }
    };

    app.keys = Some(keys.clone());

    let client = Client::new(&keys);
    app.status = "クライアントを作成しました。リレーに接続しています...".to_string();
    terminal.draw(|f| ui(f, &app))?;

    let relay_config = match load_relays() {
        Ok(c) => c,
        Err(e) => {
            app.status = format!("リレー設定の読み込みに失敗: {}、デフォルトを使用します", e);
            terminal.draw(|f| ui(f, &app))?;
            let mut config = crate::commands::RelayConfig::default();
            config.relays = vec![];
            config
        }
    };

    if relay_config.relays.is_empty() {
        app.status = "デフォルトリレーに接続しています...".to_string();
        terminal.draw(|f| ui(f, &app))?;

        match client.add_relay("wss://relay-jp.nostr.wirednet.jp").await {
            Ok(_) => {
                app.status = "デフォルトリレーに接続しました".to_string();
                terminal.draw(|f| ui(f, &app))?;
            },
            Err(e) => {
                app.status = format!("デフォルトリレー接続エラー: {}", e);
                terminal.draw(|f| ui(f, &app))?;
            }
        }

        // デフォルトリレーを変更（wss://yabu.me）
        match client.add_relay("wss://yabu.me").await {
            Ok(_) => {
                app.status = format!("追加リレーに接続しました: wss://yabu.me");
                terminal.draw(|f| ui(f, &app))?;
            },
            Err(e) => {
                app.status = format!("リレー接続エラー (wss://yabu.me): {}", e);
                terminal.draw(|f| ui(f, &app))?;
            }
        }
    } else {
        for url in &relay_config.relays {
            app.status = format!("リレーに接続中: {}", url);
            terminal.draw(|f| ui(f, &app))?;

            match client.add_relay(url.clone()).await {
                Ok(_) => {
                    app.status = format!("リレーに接続: {}", url);
                    terminal.draw(|f| ui(f, &app))?;
                },
                Err(e) => {
                    app.status = format!("リレー接続エラー ({}): {}", url, e);
                    terminal.draw(|f| ui(f, &app))?;
                }
            }
        }
    }

    client.connect().await;
    app.client = Some(client);
    app.status = "接続完了。rキーで更新、aキーでAbout画面、sキーで電卓を表示します。".to_string(); // cキーをsキーに変更
    terminal.draw(|f| ui(f, &app))?;

    let tick_rate = Duration::from_millis(100);
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| ui(f, &app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if event::poll(timeout)? {
            if let CrosstermEvent::Key(key) = event::read()? {
                if !app.handle_key_event(key) {
                    break;
                }

                if key.code == KeyCode::Char('r') && app.input_mode == InputMode::Normal {
                    if let Err(e) = fetch_events(&mut app).await {
                        app.status = format!("イベント取得エラー: {}", e);
                    }
                }
            }
        }

        if let Some(message) = app.message_to_send.take() {
            match send_message(&mut app, &message).await {
                Ok(()) => {
                    // 修正：マルチバイト文字にも対応するプレビュー生成
                    let preview = if message.chars().count() > 20 {
                        let truncated: String = message.chars().take(17).collect();
                        format!("{}...", truncated)
                    } else {
                        message.clone()
                    };

                    app.status = format!("メッセージ「{}」を送信しました。イベントを更新中...", preview);

                    if let Err(e) = fetch_events(&mut app).await {
                        app.status = format!("イベント取得エラー: {}", e);
                    } else {
                        app.status = format!("メッセージを送信し、{}件のイベントを取得しました", 
                            app.events.len());
                    }
                }
                Err(e) => {
                    app.status = format!("送信エラー: {}", e);
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }

    // 終了処理
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, Show)?;

    Ok(())
}

// 初代Macスタイルの背景ブロックを作成 - ライフタイムエラー修正版
fn mac_background_block() -> Block<'static> {
    Block::default()
        .style(Style::default().bg(Color::White).fg(Color::Black))
}

// 初代Macスタイルのウィンドウブロックを作成 - ライフタイムエラー修正版
fn mac_window_block<'a>(title: &'a str) -> Block<'a> {
    Block::default()
        .title(Span::styled(
            format!(" {} ", title),
            Style::default()
                .fg(Color::Black)
                .bg(Color::White)
                .add_modifier(Modifier::BOLD) // Chicago風の太字
        ))
        .title_style(Style::default().fg(Color::Black).bg(Color::White))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Black))
        .border_type(BorderType::Rounded)
        .style(Style::default().bg(Color::White).fg(Color::Black))
}

// 初代Mac風のUI関数
fn ui(f: &mut Frame, app: &App) {
    // 画面全体を白背景に設定
    let bg_block = mac_background_block();
    f.render_widget(bg_block, f.size());

    // 電卓表示の場合とAbout画面表示の場合は変更なし
    if app.show_calculator {
        render_calculator(f, app);
        return;
    }

    if app.show_about {
        render_about_screen(f, app);
        return;
    }

    // レイアウトを条件分岐で変更
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(0)
        .constraints([
            Constraint::Length(1),  // メニューバー
            Constraint::Min(0),     // メインコンテンツ（拡大）
            Constraint::Length(1),  // ステータスバー
        ])
        .split(f.size());

    // Mac風メニューバー (変更なし)
    let menu_items = vec![
        format!(" {} File ", MAC_APPLE_LOGO), 
        " Edit ".to_string(), 
        " View ".to_string(), 
        " Special ".to_string(), 
    ];

    let menu_spans: Vec<Span> = menu_items.iter()
        .map(|item| Span::styled(
            item, 
            Style::default()
                .fg(Color::Black)
                .bg(Color::White)
                .add_modifier(Modifier::BOLD)
        ))
        .collect();

    let menu_line = Line::from(menu_spans);
    let menu_bar = Paragraph::new(menu_line)
        .style(Style::default().bg(Color::White).fg(Color::Black));

    f.render_widget(menu_bar, chunks[0]);

    // タブに応じたコンテンツ表示 (変更なし)
    match app.active_tab {
        0 => render_events_mac_style(f, app, chunks[1]),
        1 => render_compose_mac_style(f, app, chunks[1]),
        _ => {}
    }

    // ステータスバー (常に表示)
    let status_text = format!("{} {}", MAC_HAPPY_MAC, app.status);
    let status_style = Style::default()
        .bg(Color::White)
        .fg(Color::Black)
        .add_modifier(Modifier::BOLD);

    let status = Paragraph::new(status_text)
        .style(status_style);

    f.render_widget(status, chunks[2]);
}



// 電卓画面描画関数 - 最終版
fn render_calculator(f: &mut Frame, app: &App) {
    let area = f.size();

    // 電卓のサイズを調整
    let calc_width = 28; 
    let calc_height = 22; 

    // 画面中央に配置
    let calc_x = (area.width.saturating_sub(calc_width)) / 2;
    let calc_y = (area.height.saturating_sub(calc_height)) / 2;

    let calc_area = Rect::new(
        area.x + calc_x,
        area.y + calc_y,
        calc_width.min(area.width),
        calc_height.min(area.height)
    );

    // 影の位置
    let shadow_area = Rect::new(
        calc_area.x + 1,
        calc_area.y + 1,
        calc_width.min(area.width),
        calc_height.min(area.height)
    );

    // 影を描画
    let shadow = Block::default()
        .style(Style::default().bg(Color::DarkGray));

    f.render_widget(shadow, shadow_area);

    // 電卓本体
    let calc_title = " Calculator ";
    let calc_block = Block::default()
        .title(Span::styled(
            calc_title,
            Style::default().fg(Color::Black).bg(Color::White).add_modifier(Modifier::BOLD)
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Black))
        .style(Style::default().bg(Color::White).fg(Color::Black));

    f.render_widget(calc_block.clone(), calc_area);
    let inner_area = calc_block.inner(calc_area);

    // 電卓のレイアウト
    let calc_layout = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),   // ディスプレイ部分
            Constraint::Min(15),     // ボタン部分
        ])
        .split(inner_area);

    // ディスプレイ部分
    let display_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Black))
        .style(Style::default().bg(Color::Black).fg(Color::White));

    f.render_widget(display_block.clone(), calc_layout[0]);
    let display_inner = display_block.inner(calc_layout[0]);

    // 表示値を右揃えで表示
    let display_text = Paragraph::new(app.calculator_display.clone())
        .style(Style::default().bg(Color::Black).fg(Color::White).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Right);

    f.render_widget(display_text, display_inner);

    // ボタンエリア全体
    let button_area = calc_layout[1];

    // ボタン部分を5行に均等に分割
    let button_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(20), // 各行20%ずつ
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
        ])
        .split(button_area);

    // 最初の3行の処理
    for row_idx in 0..3 {
        let button_cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25), // 各列25%ずつ
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
            ])
            .split(button_rows[row_idx]);

        // ボタンラベルを設定
        let buttons = match row_idx {
            0 => [CALC_CLEAR, CALC_EQUAL, "/", "*"],
            1 => ["7", "8", "9", CALC_MINUS],
            2 => ["4", "5", "6", CALC_PLUS],
            _ => ["", "", "", ""],
        };

        // 各ボタンを描画
        for col_idx in 0..4 {
            let button_style = Style::default().bg(Color::White).fg(Color::Black);
            let button_block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Black))
                .style(button_style);

            f.render_widget(button_block.clone(), button_cols[col_idx]);

            let button_inner = button_block.inner(button_cols[col_idx]);
            let button_text = Paragraph::new(buttons[col_idx])
                .style(button_style)
                .alignment(Alignment::Center);

            f.render_widget(button_text, button_inner);
        }
    }

    // 4行目の処理（1 2 3）
    let row4_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(button_rows[3]);

    // 1, 2, 3 ボタンを描画
    let row4_buttons = ["1", "2", "3"];
    for col_idx in 0..3 {
        let button_style = Style::default().bg(Color::White).fg(Color::Black);
        let button_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Black))
            .style(button_style);

        f.render_widget(button_block.clone(), row4_cols[col_idx]);

        let button_inner = button_block.inner(row4_cols[col_idx]);
        let button_text = Paragraph::new(row4_buttons[col_idx])
            .style(button_style)
            .alignment(Alignment::Center);

        f.render_widget(button_text, button_inner);
    }

    // 5行目の処理（0 .）
    let row5_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50), // 0ボタンを横に2つ分
            Constraint::Percentage(25), // .ボタン
            Constraint::Percentage(25), // 空欄（=ボタン用）
        ])
        .split(button_rows[4]);

    // 0ボタン（横に2つ分の大きさ、テキストは左寄せ）
    let button_style = Style::default().bg(Color::White).fg(Color::Black);
    let button_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Black))
        .style(button_style);

    f.render_widget(button_block.clone(), row5_cols[0]);

    // 0ボタンのテキストを左寄せに変更（マージン調整法）
    let button_inner_area = button_block.inner(row5_cols[0]);
    // 左側にスペースを追加して左寄せの代わりとする
    let zero_text_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(3), // 左側のスペース
            Constraint::Min(1),    // テキスト部分
        ])
        .split(button_inner_area)[1];

    let button_text = Paragraph::new("0")
        .style(button_style)
        .alignment(Alignment::Left);

    f.render_widget(button_text, zero_text_area);

    // .ボタン（3の下）
    let button_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Black))
        .style(button_style);

    f.render_widget(button_block.clone(), row5_cols[1]);

    let button_inner = button_block.inner(row5_cols[1]);
    let button_text = Paragraph::new(CALC_DOT)
        .style(button_style)
        .alignment(Alignment::Center);

    f.render_widget(button_text, button_inner);

    // =ボタン（縦に2行分）
    let equals_area = Rect::new(
        row4_cols[3].x,                                // 4行目の右端
        row4_cols[3].y,                                // 4行目の上端
        row4_cols[3].width,                            // 幅は1マス分
        row4_cols[3].height + row5_cols[2].height      // 高さは2行分
    );

    let equals_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Black))
        .style(button_style);

    f.render_widget(equals_block.clone(), equals_area);

    // =ボタンのテキストを5行目と同じ高さに配置
    // 5行目の中心に合わせるために、上から高さの75%の位置に配置
    let equals_inner_area = equals_block.inner(equals_area);

    let equals_text_area = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(75),  // 上部スペース - 5行目ボタンの中心に合わせる
            Constraint::Percentage(25),  // テキスト部分
        ])
        .split(equals_inner_area)[1];

    let equals_text = Paragraph::new(CALC_EQUAL)
        .style(button_style)
        .alignment(Alignment::Center);

    f.render_widget(equals_text, equals_text_area);

    // 操作説明
    let hint_area = Rect::new(
        calc_area.x,
        calc_area.y + calc_height,
        calc_width,
        1
    );

    let hint_text = Paragraph::new("ESC または q キーで閉じる")
        .style(Style::default().bg(Color::White).fg(Color::Black).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center);

    f.render_widget(hint_text, hint_area);
}

