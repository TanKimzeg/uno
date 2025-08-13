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
struct GameState {
    player_id: Option<usize>,
    session_id: Option<String>,
    players_cards_count: Vec<(String, usize)>,
    top_card: Option<UnoCard>,
    current_player: usize,
    clockwise: bool,
    hand: Vec<UnoCard>,
}
#[derive(Default, Clone)]
struct AppState {
    connected: bool,
    game_state: GameState,
    cursor: usize,
    log: Vec<String>,
    input_hint: Vec<Line<'static>>,
    mode: UiMode,
    pending_action: Option<PendingPlay>,
    color_pick_index: usize,
    room_input: String,
    name_input: String,
    input_focus: InputFocus,
    scoreboard: Option<Vec<ScoreEntry>>,
    room_id: Option<String>, // 新增: 当前房间ID
}
#[derive(Clone, Copy, Debug, Default)]
enum UiMode {
    #[default]
    Normal,
    ColorPick,
    DrawnCardPlayable {
        card_index: usize,
    },
    NameInput,
    Scoreboard,
}
#[derive(Clone, Debug)]
struct PendingPlay {
    card_index: usize,
    call_uno: bool,
}
#[derive(Clone, Debug)]
struct ScoreEntry {
    name: String,
    score: i32,
    rank: usize,
    is_winner: bool,
}
#[derive(Clone, Copy, Debug)]
enum InputFocus { Room, Name }
impl Default for InputFocus { fn default() -> Self { InputFocus::Room } }

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
    app.input_hint.push(Line::from("J 加入"));
    app.push_log(format!("连接到 {}，按 J 加入游戏", addr));
    let tick_rate = Duration::from_millis(500);
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
        UiMode::NameInput => handle_key_name_input(key, app, tx)?,
        UiMode::Scoreboard => handle_key_scoreboard(key, app, tx)?,
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
            app.mode = UiMode::NameInput;
            app.room_input.clear();
            app.name_input.clear();
            app.input_focus = InputFocus::Room;
            app.push_log("输入房间与昵称，Tab 切换，Enter 提交，Esc 取消");
            app.input_hint = vec![Line::from("S 开局")];
        }
        KeyCode::Char('s') => {
            if let Some(pid) = app.game_state.player_id {
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
                .min(app.game_state.hand.len().saturating_sub(1));
        }
        KeyCode::Enter => {
            try_play_selected(false, app, tx)?;
        }
        KeyCode::Char('u') => {
            try_play_selected(true, app, tx)?;
        }
        KeyCode::Char('d') => {
            if let Some(pid) = app.game_state.player_id {
                tx.send(Client2Server::DrawCard {
                    player_id: pid,
                    count: 1,
                })
                .ok();
            }
        }
        KeyCode::Char('p') => {
            if let Some(pid) = app.game_state.player_id {
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
                if let Some(pid) = app.game_state.player_id {
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
            if let Some(pid) = app.game_state.player_id {
                play_card_with_color_resolution(app, tx, pid, card_index, false)?;
            }
            app.mode = UiMode::Normal;
        }
        KeyCode::Char('u') => {
            if let Some(pid) = app.game_state.player_id {
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
    if let Some(pid) = app.game_state.player_id {
        if app.game_state.hand.get(app.cursor).is_some() {
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
    if let Some(card) = app.game_state.hand.get(card_index).copied() {
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
            app.game_state.player_id = Some(player_id);
            app.game_state.session_id = Some(session_id);
            app.connected = true;
            app.push_log(format!("Welcome! 你的 id 是 {}", player_id));
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
            app.game_state.players_cards_count = players_cards_count;
            app.game_state.top_card = top_card;
            app.game_state.current_player = current_player;
            app.game_state.clockwise = clockwise;
        }
        Server2Client::PlayerState { player_id, hand } => {
            if Some(player_id) == app.game_state.player_id {
                app.game_state.hand = hand;
                if app.cursor >= app.game_state.hand.len() {
                    app.cursor = app.game_state.hand.len().saturating_sub(1);
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
        UiMode::NameInput => draw_name_input_popup(f, size, app),
        UiMode::Scoreboard => draw_scoreboard_popup(f, size, app),
        UiMode::Normal => {}
    }
}

fn draw_status(f: &mut ratatui::Frame<'_>, area: Rect, app: &AppState) {
    let title = format!(
        "UNO | 房间:{} | 玩家:{} | 当前:{} | 方向:{}",
        app.room_id.as_deref().unwrap_or("-"),
        app.game_state
            .player_id
            .map(|v| v.to_string())
            .unwrap_or_else(|| "-".into()),
        app.game_state.current_player,
        if app.game_state.clockwise { "顺时针" } else { "逆时针" }
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
    for (i, (name, n)) in app.game_state.players_cards_count.iter().enumerate() {
        let turn = if i == app.game_state.current_player {
            " ←"
        } else {
            ""
        };
        players_text.push(Line::from(format!("{}: {:>2}{}", name, n, turn)));
    }
    let players = Paragraph::new(Text::from(players_text))
        .block(Block::default().borders(Borders::ALL).title("玩家"));
    f.render_widget(players, cols[0]);
    // 中：桌面
    let mut lines = vec![Line::from("顶部牌:")];
    match &app.game_state.top_card {
        None => lines.push(Line::from("无")),
        Some(c) => lines.push(card_line(c, false)),
    };
    lines.push(Line::from(""));
    lines.extend(app.input_hint.clone());
    let desk = Paragraph::new(Text::from(lines))
        .block(Block::default().borders(Borders::ALL).title("桌面"));
    f.render_widget(desk, cols[1]);
    // 右：手牌
    let hand_lines: Vec<Line> = app
        .game_state
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
fn handle_key_name_input(key: KeyEvent, app: &mut AppState, tx: &Sender<Client2Server>) -> io::Result<()> {
    match key.code {
        KeyCode::Esc => { app.mode = UiMode::Normal; }
        KeyCode::Tab => { app.input_focus = match app.input_focus { InputFocus::Room => InputFocus::Name, InputFocus::Name => InputFocus::Room }; }
        KeyCode::Enter => {
            if app.room_input.trim().is_empty() { app.push_log("房间ID不能为空"); }
            else if app.name_input.trim().is_empty() { app.push_log("昵称不能为空"); }
            else {
                let room_id = app.room_input.trim().to_string();
                let name = app.name_input.trim().to_string();
                tx.send(Client2Server::JoinGame { room_id: room_id.clone(), name: name.clone() }).ok();
                app.room_id = Some(room_id.clone());
                app.push_log(format!("发送 JoinGame room={} name={}", room_id, name));
                app.mode = UiMode::Normal;
                app.input_hint = vec![Line::from("S 开局"), Line::from("↑/↓ 选牌 ...")];
            }
        }
        KeyCode::Backspace => {
            match app.input_focus { InputFocus::Room => { app.room_input.pop(); } InputFocus::Name => { app.name_input.pop(); } }
        }
        KeyCode::Left => {}
        KeyCode::Right => {}
        KeyCode::Char(c) => {
            if !c.is_control() {
                match app.input_focus {
                    InputFocus::Room => if app.room_input.len() < 24 { app.room_input.push(c); },
                    InputFocus::Name => if app.name_input.len() < 24 { app.name_input.push(c); },
                }
            }
        }
        _ => {}
    }
    Ok(())
}
fn draw_name_input_popup(f: &mut ratatui::Frame<'_>, area: Rect, app: &AppState) {
    let popup = centered_rect(60, 40, area);
    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from("输入房间与昵称 (Tab 切换, Enter 确认 / Esc 取消)"));
    lines.push(Line::from(""));
    let room_style = if matches!(app.input_focus, InputFocus::Room) { Style::default().fg(TColor::Yellow).add_modifier(Modifier::BOLD | Modifier::UNDERLINED) } else { Style::default().fg(TColor::White) };
    let name_style = if matches!(app.input_focus, InputFocus::Name) { Style::default().fg(TColor::Yellow).add_modifier(Modifier::BOLD | Modifier::UNDERLINED) } else { Style::default().fg(TColor::White) };
    lines.push(Line::from(vec![Span::styled("房间: ", Style::default().fg(TColor::Cyan)), Span::styled(if app.room_input.is_empty() { "<空>".into() } else { app.room_input.clone() }, room_style)]));
    lines.push(Line::from(vec![Span::styled("昵称: ", Style::default().fg(TColor::Cyan)), Span::styled(if app.name_input.is_empty() { "<空>".into() } else { app.name_input.clone() }, name_style)]));
    if let Some(r) = &app.room_id { lines.push(Line::from(format!("已加入房间: {}", r))); }
    let block = Paragraph::new(Text::from(lines)).block(Block::default().borders(Borders::ALL).title("加入游戏"));
    f.render_widget(block, popup);
}
fn handle_key_scoreboard(
    key: KeyEvent,
    app: &mut AppState,
    tx: &Sender<Client2Server>,
) -> io::Result<()> {
    match key.code {
        KeyCode::Esc | KeyCode::Enter => {
            app.mode = UiMode::Normal;
        }
        KeyCode::Char('n') => {
            if let Some(pid) = app.game_state.player_id {
                tx.send(Client2Server::JoinGame { 
                    room_id: app.room_id.clone().unwrap().to_string(),
                    name: app.name_input.clone(),
                } ).ok();
                app.push_log(format!("{} 想再来一局", app.name_input));
            }
        }
        _ => {}
    }
    Ok(())
}
fn draw_scoreboard_popup(f: &mut ratatui::Frame<'_>, area: Rect, app: &AppState) {
    if let Some(entries) = &app.scoreboard {
        let popup = centered_rect(60, 60, area);
        let mut lines: Vec<Line> = Vec::new();
        lines.push(Line::from("本局结果 (Enter/Esc 关闭, N 再来一局)"));
        lines.push(Line::from(""));
        for e in entries {
            let style = if e.is_winner {
                Style::default()
                    .fg(TColor::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(TColor::White)
            };
            let line = Line::from(vec![
                Span::styled(
                    format!("#{:<2} ", e.rank),
                    Style::default().fg(TColor::Yellow),
                ),
                Span::styled(format!("{:<12}", e.name), style),
                Span::styled(
                    format!(" 分数: {:>4}", e.score),
                    Style::default().fg(TColor::Cyan),
                ),
                Span::raw(if e.is_winner { "  <- WIN" } else { "" }),
            ]);
            lines.push(line);
        }
        let block = Paragraph::new(Text::from(lines))
            .block(Block::default().borders(Borders::ALL).title("比分"));
        f.render_widget(block, popup);
    }
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
            GE::GameStarted { game_id } => {
                app.push_log(format!("Game started: {}", game_id));
                app.mode = UiMode::Normal;
                app.input_hint = vec![
                    Line::from("↑/↓ 选牌"),
                    Line::from("Enter 出牌"),
                    Line::from("D 摸牌"),
                    Line::from("P 跳过"),
                    Line::from("U UNO"),
                    Line::from("Q 退出"),
                ];
            }
            GE::CardPlayed { player_id, card } => {
                app.push_log(format!("Player {} played {}", player_id, card.to_string()))
            }
            GE::GameError { message } => app.push_log(format!("Error: {}", message)),
            GE::CardDraw { player_id, card } => {
                if Some(*player_id) == app.game_state.player_id {
                    app.push_log(format!("You drew: {}", card.to_string()));
                } else {
                    app.push_log(format!("Player {} drew", player_id));
                }
            }
            GE::DrawnCardPlayable { player_id } => {
                if Some(*player_id) == app.game_state.player_id {
                    if !app.game_state.hand.is_empty() {
                        let idx = app.game_state.hand.len() - 1;
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
                app.push_log(format!("Game over! Winner {} scores {:?}", winner, scores));
                // 构建比分表：UNO 规则中分数越低（负分绝对值越小）谁赢？假设 winner 已经由服务器判断
                let mut entries: Vec<ScoreEntry> = scores
                    .iter()
                    .enumerate()
                    .map(|(i, (name, sc))| ScoreEntry {
                        name: name.clone(),
                        score: *sc,
                        rank: 0,
                        is_winner: i == 0, // 服务端已经排好序,第一个就是赢家
                    })
                    .collect();
                // 排序：按分数升序
                entries.sort_by_key(|e| e.score);
                // 赋 rank
                for (idx, e) in entries.iter_mut().enumerate() {
                    e.rank = idx + 1;
                }
                app.scoreboard = Some(entries);
                app.mode = UiMode::Scoreboard;
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
                app.game_state.top_card = Some(*top_card);
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
