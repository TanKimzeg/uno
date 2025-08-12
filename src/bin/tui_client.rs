use std::io::{self, BufRead};
use std::net::TcpStream;
use std::thread;
use std::time::{Duration, Instant};

use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event as CEvent, KeyCode, KeyEvent,
        KeyEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use flume::{Receiver, Sender};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color as TColor, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph}, 
    Terminal,
};

use uno::game::cards::{Color as UColor, UnoCard};
use uno::game::events::GameEvent as GE;
use uno::protocol::{Client2Server, Server2Client};

// ---------------- 状态定义 ----------------
#[derive(Default, Clone)]
struct AppState {
    connected: bool,
    player_id: Option<usize>,
    session_id: Option<String>,
    players_cards_count: Vec<(String, usize)>,
    top_card: Option<UnoCard>,
    current_player: usize,
    clockwise: bool,
    hand: Vec<UnoCard>,
    cursor: usize,
    log: Vec<String>,
    input_hint: String,
    mode: UiMode,
    pending_action: Option<PendingPlay>,
    color_pick_index: usize,
}

#[derive(Clone, Copy, Debug, Default)]
enum UiMode {
    #[default]
    Normal,
    ColorPick,
    DrawnCardPlayable {
        card_index: usize,
    },
}
#[derive(Clone, Debug)]
struct PendingPlay {
    card_index: usize,
    call_uno: bool,
}

impl AppState {
    fn push_log<S: Into<String>>(&mut self, s: S) {
        self.log.push(s.into());
    }
}

// ---------------- 工具函数 ----------------
fn map_color(c: UColor) -> TColor {
    match c {
        UColor::RED => TColor::Red,
        UColor::GREEN => TColor::Green,
        UColor::BLUE => TColor::Blue,
        UColor::YELLOW => TColor::Yellow,
    }
}
fn card_line(card: &UnoCard, selected: bool) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    if selected {
        spans.push(Span::styled(
            "▶ ",
            Style::default()
                .fg(TColor::Cyan)
                .add_modifier(Modifier::BOLD),
        ));
    } else {
        spans.push(Span::raw("  "));
    }
    match card {
        UnoCard::NumberCard(color, number) => {
            let fg = map_color(*color);
            spans.push(Span::styled(
                format!("{:<7}", "NUM"),
                Style::default().fg(TColor::Gray),
            ));
            spans.push(Span::styled(
                number.to_string(),
                Style::default().fg(fg).add_modifier(if selected {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                }),
            ));
            spans.push(Span::raw(" "));
            spans.push(Span::styled(
                format!("{:?}", color),
                Style::default().fg(fg),
            ));
        }
        UnoCard::ActionCard(color, action) => {
            let fg = map_color(*color);
            spans.push(Span::styled(
                action.to_string(),
                Style::default().fg(fg).add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::raw(" "));
            spans.push(Span::styled(
                format!("{:?}", color),
                Style::default().fg(fg),
            ));
        }
        UnoCard::WildCard(maybe_color, wt) => {
            let (label, fg) = match (maybe_color, wt) {
                (Some(c), _) => (wt.to_string(), map_color(*c)),
                (None, _) => (wt.to_string(), TColor::White),
            };
            spans.push(Span::styled(
                label,
                Style::default().fg(fg).add_modifier(Modifier::BOLD),
            ));
            if let Some(c) = maybe_color {
                spans.push(Span::raw(" "));
                spans.push(Span::styled(
                    format!("{:?}", c),
                    Style::default().fg(map_color(*c)),
                ));
            }
        }
    }
    Line::from(spans)
}

// ---------------- 主入口 ----------------
fn main() -> io::Result<()> {
    let addr = "127.0.0.1:9000";
    let stream = TcpStream::connect(addr)?;
    stream.set_nodelay(true)?;
    let read_stream = stream.try_clone()?;
    let (net_to_ui_tx, net_to_ui_rx) = flume::bounded::<Server2Client>(1024);
    let (ui_to_net_tx, ui_to_net_rx) = flume::bounded::<Client2Server>(1024);
    thread::spawn(move || net_read_loop(read_stream, net_to_ui_tx));
    thread::spawn(move || net_write_loop(stream, ui_to_net_rx));
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut app = AppState::default();
    app.input_hint = "J 加入\nS 开局\n↑/↓ 选牌\nEnter 出牌\nD 摸牌\nP 跳过\nU UNO\nQ 退出".into();
    app.push_log(format!("连接到 {}，按 J 加入游戏", addr));
    let tick_rate = Duration::from_millis(50);
    let mut last_tick = Instant::now();
    let mut quit = false;
    while !quit {
        while let Ok(msg) = net_to_ui_rx.try_recv() {
            handle_server_msg(&mut app, msg, &ui_to_net_tx);
        }
        terminal.draw(|f| ui(f, &app))?;
        let timeout = tick_rate.saturating_sub(last_tick.elapsed());
        if event::poll(timeout)? {
            if let CEvent::Key(key) = event::read()? {
                quit = should_quit(key, &mut app, &ui_to_net_tx)?;
            }
        }
        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }
    disable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;
    Ok(())
}

// ---------------- 键盘处理 ----------------
fn should_quit(key: KeyEvent, app: &mut AppState, tx: &Sender<Client2Server>) -> io::Result<bool> {
    let is_nav = matches!(
        key.code,
        KeyCode::Left | KeyCode::Right | KeyCode::Up | KeyCode::Down
    );
    let allow = if is_nav {
        matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat)
    } else {
        matches!(key.kind, KeyEventKind::Press)
    };
    if !allow {
        return Ok(false);
    }
    match app.mode {
        UiMode::Normal => {
            if matches!(key.code, KeyCode::Char('q') | KeyCode::Esc) {
                return Ok(true);
            }
            handle_key_normal(key, app, tx)?;
        }
        UiMode::ColorPick => handle_key_colorpick(key, app, tx)?,
        UiMode::DrawnCardPlayable { card_index } => {
            handle_key_drawn_playable(key, app, tx, card_index)?
        }
    };
    Ok(false)
}

fn handle_key_normal(
    key: KeyEvent,
    app: &mut AppState,
    tx: &Sender<Client2Server>,
) -> io::Result<()> {
    match key.code {
        KeyCode::Char('j') => {
            let name = std::env::var("USERNAME")
                .or_else(|_| std::env::var("USER"))
                .unwrap_or_else(|_| "player".into());
            tx.send(Client2Server::JoinGame { name }).ok();
            app.push_log("发送 Join 请求");
        }
        KeyCode::Char('s') => {
            if let Some(pid) = app.player_id {
                tx.send(Client2Server::StartGame { player_id: pid }).ok();
            }
        }
        KeyCode::Up => {
            if app.cursor > 0 {
                app.cursor -= 1;
            }
        }
        KeyCode::Down => {
            app.cursor = app
                .cursor
                .saturating_add(1)
                .min(app.hand.len().saturating_sub(1));
        }
        KeyCode::Enter => {
            try_play_selected(false, app, tx)?;
        }
        KeyCode::Char('u') => {
            try_play_selected(true, app, tx)?;
        }
        KeyCode::Char('d') => {
            if let Some(pid) = app.player_id {
                tx.send(Client2Server::DrawCard {
                    player_id: pid,
                    count: 1,
                })
                .ok();
            }
        }
        KeyCode::Char('p') => {
            if let Some(pid) = app.player_id {
                tx.send(Client2Server::PassTurn { player_id: pid }).ok();
            }
        }
        _ => {}
    }
    Ok(())
}

fn handle_key_colorpick(
    key: KeyEvent,
    app: &mut AppState,
    tx: &Sender<Client2Server>,
) -> io::Result<()> {
    match key.code {
        KeyCode::Esc => {
            app.mode = UiMode::Normal;
            app.pending_action = None;
        }
        KeyCode::Left | KeyCode::Up => {
            if app.color_pick_index == 0 {
                app.color_pick_index = 3;
            } else {
                app.color_pick_index -= 1;
            }
        }
        KeyCode::Right | KeyCode::Down => {
            app.color_pick_index = (app.color_pick_index + 1) % 4;
        }
        KeyCode::Char('r') => app.color_pick_index = 0,
        KeyCode::Char('g') => app.color_pick_index = 1,
        KeyCode::Char('b') => app.color_pick_index = 2,
        KeyCode::Char('y') => app.color_pick_index = 3,
        KeyCode::Enter => {
            if let Some(p) = app.pending_action.take() {
                if let Some(pid) = app.player_id {
                    let color = match app.color_pick_index {
                        0 => UColor::RED,
                        1 => UColor::GREEN,
                        2 => UColor::BLUE,
                        _ => UColor::YELLOW,
                    };
                    tx.send(Client2Server::PlayCard {
                        player_id: pid,
                        card_index: p.card_index,
                        color,
                        call_uno: p.call_uno,
                    })
                    .ok();
                }
            }
            app.mode = UiMode::Normal;
        }
        _ => {}
    }
    Ok(())
}

fn handle_key_drawn_playable(
    key: KeyEvent,
    app: &mut AppState,
    tx: &Sender<Client2Server>,
    card_index: usize,
) -> io::Result<()> {
    match key.code {
        KeyCode::Esc => {
            app.mode = UiMode::Normal;
        }
        KeyCode::Enter => {
            if let Some(pid) = app.player_id {
                play_card_with_color_resolution(app, tx, pid, card_index, false)?;
            }
            app.mode = UiMode::Normal;
        }
        KeyCode::Char('u') => {
            if let Some(pid) = app.player_id {
                play_card_with_color_resolution(app, tx, pid, card_index, true)?;
            }
            app.mode = UiMode::Normal;
        }
        _ => {}
    }
    Ok(())
}

fn try_play_selected(
    call_uno: bool,
    app: &mut AppState,
    tx: &Sender<Client2Server>,
) -> io::Result<()> {
    if let Some(pid) = app.player_id {
        if app.hand.get(app.cursor).is_some() {
            play_card_with_color_resolution(app, tx, pid, app.cursor, call_uno)?;
        }
    }
    Ok(())
}

fn play_card_with_color_resolution(
    app: &mut AppState,
    tx: &Sender<Client2Server>,
    pid: usize,
    card_index: usize,
    call_uno: bool,
) -> io::Result<()> {
    if let Some(card) = app.hand.get(card_index).copied() {
        match card {
            UnoCard::WildCard(Some(c), _) => {
                tx.send(Client2Server::PlayCard {
                    player_id: pid,
                    card_index,
                    color: c,
                    call_uno,
                })
                .ok();
            }
            UnoCard::WildCard(None, _) => {
                app.pending_action = Some(PendingPlay {
                    card_index,
                    call_uno,
                });
                app.color_pick_index = 0;
                app.mode = UiMode::ColorPick;
            }
            _ => {
                tx.send(Client2Server::PlayCard {
                    player_id: pid,
                    card_index,
                    color: UColor::RED, // 在uno_game里面只对WildCard有传入颜色要求
                    call_uno,
                })
                .ok();
            }
        }
    }
    Ok(())
}

// ---------------- 网络 IO ----------------
fn net_read_loop(stream: TcpStream, tx: Sender<Server2Client>) {
    let reader = std::io::BufReader::new(stream);
    for line in reader.lines() {
        match line {
            Ok(text) => {
                if let Ok(msg) = serde_json::from_str::<Server2Client>(&text) {
                    let _ = tx.send(msg);
                }
            }
            Err(_) => break,
        }
    }
}
fn net_write_loop(mut stream: TcpStream, rx: Receiver<Client2Server>) {
    while let Ok(msg) = rx.recv() {
        if let Ok(json) = serde_json::to_string(&msg) {
            use std::io::Write;
            if writeln!(stream, "{}", json).is_err() {
                break;
            }
            let _ = stream.flush();
        }
    }
}

// ---------------- 协议消息处理 ----------------
fn handle_server_msg(app: &mut AppState, msg: Server2Client, tx: &Sender<Client2Server>) {
    match msg {
        Server2Client::Welcome {
            player_id,
            session_id,
        } => {
            app.player_id = Some(player_id);
            app.session_id = Some(session_id);
            app.connected = true;
            app.push_log(format!("Welcome! 你的 id 是 {}", player_id));
        }
        Server2Client::GameStarted { game_id, players } => {
            app.push_log(format!("GameStarted {} players={:?}", game_id, players));
        }
        Server2Client::Events(ev) => {
            handle_events(app, &ev, tx);
        }
        Server2Client::ServerError { message } => app.push_log(format!("[Error] {}", message)),
        Server2Client::SharedState {
            players_cards_count,
            top_card,
            current_player,
            clockwise,
        } => {
            app.players_cards_count = players_cards_count;
            app.top_card = top_card;
            app.current_player = current_player;
            app.clockwise = clockwise;
        }
        Server2Client::PlayerState { player_id, hand } => {
            if Some(player_id) == app.player_id {
                app.hand = hand;
                if app.cursor >= app.hand.len() {
                    app.cursor = app.hand.len().saturating_sub(1);
                }
            }
        }
    }
}

// ---------------- 主 UI 绘制 ----------------
fn ui(f: &mut ratatui::Frame<'_>, app: &AppState) {
    let size = f.size();
    let v = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(8),
        ])
        .split(size);
    draw_status(f, v[0], app);
    draw_main(f, v[1], app);
    draw_log(f, v[2], app);
    match app.mode {
        UiMode::ColorPick => draw_color_picker_popup(f, size, app),
        UiMode::DrawnCardPlayable { .. } => draw_drawn_playable_popup(f, size),
        UiMode::Normal => {}
    }
}

fn draw_status(f: &mut ratatui::Frame<'_>, area: Rect, app: &AppState) {
    let title = format!(
        "UNO | 玩家:{} | 当前:{} | 方向:{}",
        app.player_id
            .map(|v| v.to_string())
            .unwrap_or_else(|| "-".into()),
        app.current_player,
        if app.clockwise {
            "顺时针"
        } else {
            "逆时针"
        }
    );
    let para = Paragraph::new(title).block(Block::default().borders(Borders::ALL).title("状态"));
    f.render_widget(para, area);
}

fn draw_main(f: &mut ratatui::Frame<'_>, area: Rect, app: &AppState) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(34),
            Constraint::Percentage(33),
        ])
        .split(area);
    // 左：玩家
    let mut players_text: Vec<Line> = Vec::new();
    for (i, (name, n)) in app.players_cards_count.iter().enumerate() {
        let turn = if i == app.current_player { " ←" } else { "" };
        players_text.push(Line::from(format!("{}: {:>2}{}", name, n, turn)));
    }
    let players = Paragraph::new(Text::from(players_text))
        .block(Block::default().borders(Borders::ALL).title("玩家"));
    f.render_widget(players, cols[0]);
    // 中：桌面
    let mut lines = vec![Line::from("顶部牌:")];
    match &app.top_card {
        None => lines.push(Line::from("无")),
        Some(c) => lines.push(card_line(c, false)),
    };
    lines.push(Line::from(""));
    lines.push(Line::from(app.input_hint.as_str()));
    let desk = Paragraph::new(Text::from(lines))
        .block(Block::default().borders(Borders::ALL).title("桌面"));
    f.render_widget(desk, cols[1]);
    // 右：手牌
    let hand_lines: Vec<Line> = app
        .hand
        .iter()
        .enumerate()
        .map(|(i, c)| card_line(c, i == app.cursor))
        .collect();
    let hand = Paragraph::new(Text::from(hand_lines))
        .block(Block::default().borders(Borders::ALL).title("手牌"));
    f.render_widget(hand, cols[2]);
}

fn draw_log(f: &mut ratatui::Frame<'_>, area: Rect, app: &AppState) {
    let lines: Vec<Line> = app
        .log
        .iter()
        .rev()
        .take(8)
        .cloned()
        .map(Line::from)
        .collect();
    let para = Paragraph::new(Text::from(lines))
        .block(Block::default().borders(Borders::ALL).title("日志"));
    f.render_widget(para, area);
}

// ---------------- 弹窗 ----------------
fn draw_color_picker_popup(f: &mut ratatui::Frame<'_>, area: Rect, app: &AppState) {
    let popup = centered_rect(40, 30, area);
    let colors = ["RED", "GREEN", "BLUE", "YELLOW"];
    let mut lines: Vec<Line> = vec![Line::from(
        "选择颜色 (←/→ 或 R/G/B/Y, Enter 确认, Esc 取消)",
    )];
    let spans: Vec<Span> = colors
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let style = if i == app.color_pick_index {
                Style::default()
                    .fg(TColor::Yellow)
                    .add_modifier(Modifier::BOLD | Modifier::REVERSED)
            } else {
                Style::default().fg(TColor::White)
            };
            Span::styled(format!(" {} ", c), style)
        })
        .collect();
    lines.push(Line::from(spans));
    let block = Paragraph::new(Text::from(lines))
        .block(Block::default().borders(Borders::ALL).title("颜色"));
    f.render_widget(block, popup);
}
fn draw_drawn_playable_popup(f: &mut ratatui::Frame<'_>, area: Rect) {
    let popup = centered_rect(50, 20, area);
    let lines = vec![Line::from(
        "刚摸的牌可立即出 (Enter 出牌 / U 出牌并 UNO / Esc 放弃)",
    )];
    let block = Paragraph::new(Text::from(lines))
        .block(Block::default().borders(Borders::ALL).title("摸牌可出"));
    f.render_widget(block, popup);
}
fn centered_rect(pct_x: u16, pct_y: u16, r: Rect) -> Rect {
    let vert = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - pct_y) / 2),
            Constraint::Percentage(pct_y),
            Constraint::Percentage((100 - pct_y) / 2),
        ])
        .split(r);
    let horz = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - pct_x) / 2),
            Constraint::Percentage(pct_x),
            Constraint::Percentage((100 - pct_x) / 2),
        ])
        .split(vert[1]);
    horz[1]
}

// ---------------- 事件处理 ----------------
fn handle_events(app: &mut AppState, events: &[GE], _tx: &Sender<Client2Server>) {
    for e in events {
        match e {
            GE::PlayerJoined { player_id, name } => {
                app.push_log(format!("Player {} joined: {}", player_id, name))
            }
            GE::CardPlayed { player_id, card } => {
                app.push_log(format!("Player {} played {}", player_id, card.to_string()))
            }
            GE::GameError { message } => app.push_log(format!("Error: {}", message)),
            GE::CardDraw { player_id, card } => {
                if Some(*player_id) == app.player_id {
                    app.push_log(format!("You drew: {}", card.to_string()));
                } else {
                    app.push_log(format!("Player {} drew", player_id));
                }
            }
            GE::DrawnCardPlayable { player_id } => {
                if Some(*player_id) == app.player_id {
                    if !app.hand.is_empty() {
                        let idx = app.hand.len() - 1;
                        app.mode = UiMode::DrawnCardPlayable { card_index: idx };
                        app.push_log("你刚摸的牌可立即出");
                    }
                }
            }
            GE::DirectionChanged { clockwise } => app.push_log(format!(
                "Direction: {}",
                if *clockwise { "CW" } else { "CCW" }
            )),
            GE::DrawFourApplied { target_player_id } => {
                app.push_log(format!("+4 -> Player {}", target_player_id))
            }
            GE::DrawTwoApplied { target_player_id } => {
                app.push_log(format!("+2 -> Player {}", target_player_id))
            }
            GE::GameOver { winner, scores } => {
                app.push_log(format!("Game over! Winner {} scores {:?}", winner, scores))
            }
            GE::PlayerChallenged {
                challenger_id,
                challenged_id,
            } => app.push_log(format!(
                "Player {} challenged {}",
                challenger_id, challenged_id
            )),
            GE::PlayerPassed { player_id } => app.push_log(format!("Player {} passed", player_id)),
            GE::PlayerSkipped { player_id } => {
                app.push_log(format!("Player {} skipped", player_id))
            }
            GE::PlayerTurn { player_id } => app.push_log(format!("Turn: Player {}", player_id)),
            GE::TopCardChanged { top_card } => {
                app.top_card = Some(*top_card);
                app.push_log("Top card changed");
            }
            GE::UnoCalled { player_id } => app.push_log(format!("Player {} UNO!", player_id)),
            GE::UnoPenalty { player_id } => {
                app.push_log(format!("UNO penalty -> Player {}", player_id))
            }
            GE::ChallengedFailed {
                challenger_id,
                challenged_id,
            } => app.push_log(format!(
                "Challenge failed {} -> {}",
                challenger_id, challenged_id
            )),
            GE::ChallengedSuccess {
                challenger_id,
                challenged_id,
            } => app.push_log(format!(
                "Challenge success {} -> {}",
                challenger_id, challenged_id
            )),
        }
    }
}
