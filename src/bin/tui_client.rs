use std::io::{self, Write, BufRead};
use std::net::TcpStream;
use std::time::{Duration, Instant};
use std::thread;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CEvent, KeyCode, KeyEvent, KeyModifiers, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};
use flume::{Receiver, Sender};

use uno::protocol::{Client2Server, Server2Client};
use uno::game::cards::{UnoCard, Color as UColor};
use uno::game::events::GameEvent as GE;

#[derive(Default, Clone)]
struct AppState {
    connected: bool,
    player_id: Option<usize>,
    session_id: Option<String>,
    players_cards_count: Vec<(String, usize)>,
    top_card: Option<UnoCard>,
    current_player: usize,
    clockwise: bool,
    hand: Vec<UnoCard>, // 将来从事件中维护；当前可能为空
    cursor: usize,
    log: Vec<String>,
    input_hint: String,
}

impl AppState {
    fn push_log<S: Into<String>>(&mut self, s: S) { self.log.push(s.into()); }
}

fn main() -> io::Result<()> {
    // 连接
    let addr = "127.0.0.1:9000";
    let stream = TcpStream::connect(addr)?;
    stream.set_nodelay(true)?;
    let read_stream = stream.try_clone()?;

    // 通道：网络 -> UI、UI -> 网络
    let (net_to_ui_tx, net_to_ui_rx) = flume::bounded::<Server2Client>(1024);
    let (ui_to_net_tx, ui_to_net_rx) = flume::bounded::<Client2Server>(1024);

    // 启动读、写线程
    thread::spawn(move || net_read_loop(read_stream, net_to_ui_tx));
    thread::spawn(move || net_write_loop(stream, ui_to_net_rx));

    // 启动 TUI
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = AppState::default();
    app.input_hint = "J 加入, S 开局, ←/→ 选牌, Enter 出牌, D 摸牌, P 跳过, U UNO, Q 退出".into();

    // 给自己打个“自动 Join”提示，可按 J 发送 Join
    app.push_log(format!("连接到 {}，按 J 加入游戏", addr));

    let tick_rate = Duration::from_millis(50);
    let mut last_tick = Instant::now();

    loop {
        // 处理网络消息（非阻塞尽量清空队列）
        while let Ok(msg) = net_to_ui_rx.try_recv() {
            handle_server_msg(&mut app, msg, &ui_to_net_tx);
        }

        // 绘制
        terminal.draw(|f| ui(f, &app))?;

        // 键盘或tick
        let timeout = tick_rate.saturating_sub(last_tick.elapsed());
        if event::poll(timeout)? {
            match event::read()? {
                CEvent::Key(key) => {
                    if handle_key(key, &mut app, &ui_to_net_tx)? { break; }
                }
                _ => {}
            }
        }
        if last_tick.elapsed() >= tick_rate { last_tick = Instant::now(); }
    }

    // 退出恢复终端
    disable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;
    Ok(())
}

fn handle_key(key: KeyEvent, app: &mut AppState, tx: &Sender<Client2Server>) -> io::Result<bool> {
    // 仅在 Press 处理命令；左右导航允许 Press/Repeat
    let is_nav = matches!(key.code, KeyCode::Left | KeyCode::Right);
    let allow = if is_nav {
        matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat)
    } else {
        matches!(key.kind, KeyEventKind::Press)
    };
    if !allow { return Ok(false); }

    match (key.code, key.modifiers) {
        (KeyCode::Char('q'), KeyModifiers::NONE) | (KeyCode::Esc, _) => return Ok(true),
        (KeyCode::Char('j'), _) => {
            let name = std::env::var("USERNAME")
                .or_else(|_| std::env::var("USER"))
                .unwrap_or_else(|_| "player".into());
            tx.send(Client2Server::JoinGame { name }).ok();
            app.push_log("发送 Join 请求");
        }
        (KeyCode::Char('s'), _) => {
            if let Some(pid) = app.player_id { tx.send(Client2Server::StartGame { player_id: pid }).ok(); }
        }
        (KeyCode::Left, _) => { if app.cursor > 0 { app.cursor -= 1; } }
        (KeyCode::Right, _) => { app.cursor = app.cursor.saturating_add(1).min(app.hand.len().saturating_sub(1)); }
        (KeyCode::Enter, _) => {
            if let Some(pid) = app.player_id {
                if let Some(card) = app.hand.get(app.cursor).copied() {
                    let color = match card {
                        UnoCard::WildCard(Some(c), _) => c,
                        UnoCard::WildCard(None, _) => color_picker_blocking()?,
                        _ => UColor::RED,
                    };
                    tx.send(Client2Server::PlayCard { player_id: pid, card_index: app.cursor, color, call_uno: false }).ok();
                }
            }
        }
        (KeyCode::Char('d'), _) => { if let Some(pid) = app.player_id { tx.send(Client2Server::DrawCard { player_id: pid, count: 1 }).ok(); } }
        (KeyCode::Char('p'), _) => { if let Some(pid) = app.player_id { tx.send(Client2Server::PassTurn { player_id: pid }).ok(); } }
        (KeyCode::Char('u'), _) => {
            if let Some(pid) = app.player_id {
                // 简化：通过 PlayCard 的 call_uno 字段带上 UNO
                if let Some(card) = app.hand.get(app.cursor).copied() {
                    let color = match card {
                        UnoCard::WildCard(Some(c), _) => c,
                        UnoCard::WildCard(None, _) => color_picker_blocking()?,
                        _ => UColor::RED,
                    };
                    tx.send(Client2Server::PlayCard { player_id: pid, card_index: app.cursor, color, call_uno: true }).ok();
                }
            }
        }
        _ => {}
    }
    Ok(false)
}

fn color_picker_blocking() -> io::Result<UColor> {
    // 简化版：按 R/G/B/Y 选色
    loop {
        if event::poll(Duration::from_millis(10))? {
            if let CEvent::Key(KeyEvent { code: KeyCode::Char(c), .. }) = event::read()? {
                match c.to_ascii_lowercase() {
                    'r' => return Ok(UColor::RED),
                    'g' => return Ok(UColor::GREEN),
                    'b' => return Ok(UColor::BLUE),
                    'y' => return Ok(UColor::YELLOW),
                    _ => {}
                }
            }
        }
    }
}

fn net_read_loop(stream: TcpStream, tx: Sender<Server2Client>) {
    let reader = std::io::BufReader::new(stream);
    for line in reader.lines() {
        match line {
            Ok(text) => {
                if let Ok(msg) = serde_json::from_str::<Server2Client>(&text) {
                    let _ = tx.send(msg);
                } else {
                    // ignore non-protocol lines
                }
            }
            Err(_) => break,
        }
    }
}

fn net_write_loop(mut stream: TcpStream, rx: Receiver<Client2Server>) {
    while let Ok(msg) = rx.recv() {
        if let Ok(json) = serde_json::to_string(&msg) {
            if writeln!(stream, "{}", json).is_err() { break; }
            let _ = stream.flush();
        }
    }
}

fn handle_server_msg(app: &mut AppState, msg: Server2Client, tx: &Sender<Client2Server>) {
    match msg {
        Server2Client::Welcome { player_id, session_id } => {
            app.player_id = Some(player_id);
            app.session_id = Some(session_id);
            app.connected = true;
            app.push_log(format!("Welcome! 你的 id 是 {}", player_id));
        }
        Server2Client::GameStarted { game_id, players } => {
            app.push_log(format!("GameStarted {} players={:?}", game_id, players));
        }
        Server2Client::Events(ev) => {
            // for e in &ev { app.push_log(format!("Event: {:?}", e)); }
            handle_events(app, &ev, tx);
            
        }
        Server2Client::ServerError { message } => app.push_log(format!("[Error] {}", message)),
        Server2Client::SharedState { players_cards_count, top_card, current_player, clockwise } => {
            app.players_cards_count = players_cards_count;
            app.top_card = top_card;
            app.current_player = current_player;
            app.clockwise = clockwise;
        }
    }
}

fn ui(f: &mut ratatui::Frame<'_>, app: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(8),
        ])
        .split(f.size());

    draw_status(f, chunks[0], app);
    draw_main(f, chunks[1], app);
    draw_log(f, chunks[2], app);
}

fn draw_status(f: &mut ratatui::Frame<'_>, area: Rect, app: &AppState) {
    let title = format!(
        "UNO TUI | 玩家:{} | 当前:{} | 方向:{}",
        app.player_id.map(|v| v.to_string()).unwrap_or_else(|| "-".into()),
        app.current_player,
        if app.clockwise { "顺时针" } else { "逆时针" }
    );
    let para = Paragraph::new(title)
        .block(Block::default().borders(Borders::ALL).title("状态"));
    f.render_widget(para, area);
}

fn draw_main(f: &mut ratatui::Frame<'_>, area: Rect, app: &AppState) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(28), Constraint::Min(10), Constraint::Length(30)].as_ref())
        .split(area);

    // 左：玩家与手牌数
    let items: Vec<ListItem> = app
        .players_cards_count
        .iter()
        .enumerate()
        .map(|(i, (name, n))| {
            let turn = if i == app.current_player { " ←" } else { "" };
            ListItem::new(format!("{}: {:>2}{}", name, n, turn))
        })
        .collect();
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("玩家"));
    f.render_widget(list, cols[0]);

    // 中：顶牌与提示
    let mut text = vec![];
    text.push(Line::from("顶部牌:"));
    text.push(Line::from(format!("{:?}", app.top_card)));
    text.push(Line::from(""));
    text.push(Line::from(app.input_hint.as_str()));
    let para = Paragraph::new(Text::from(text))
        .wrap(Wrap { trim: true })
        .block(Block::default().borders(Borders::ALL).title("桌面"));
    f.render_widget(para, cols[1]);

    // 右：手牌（水平展示，简化为文本+光标）
    let mut cards = String::new();
    for (i, c) in app.hand.iter().enumerate() {
        if i == app.cursor { cards.push_str("["); }
        cards.push_str(&format!("{:?}", c));
        if i == app.cursor { cards.push_str("]"); }
        cards.push_str(" ");
    }
    let hand = Paragraph::new(cards)
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
        .block(Block::default().borders(Borders::ALL).title("日志(最近)"));
    f.render_widget(para, area);
}


fn handle_events(app: &mut AppState, events: &[GE], tx: &Sender<Client2Server>) {
     for e in events {
        match e {
            GE::PlayerJoined { player_id, name } => {
                app.log.push(format!("Player {} joined: {}", player_id, name));
            }
            GE::CardPlayed { player_id, card } => {
                app.log.push(format!("Player {} played {:?}", player_id, card));
            }
            GE::GameError { message } => {
                app.log.push(format!("Error: {}", message));
            }
            GE::CardDraw { player_id, card } => {
                if *player_id == app.player_id.unwrap() {
                    app.hand.push(*card);
                }
                else {
                    app.log.push(format!("Player {} drew a card", player_id));
                }
            }
            GE::ChallengedFailed { challenger_id, challenged_id } => {
                app.log.push(format!("Challenge failed: {} challenged {}", 
                                        challenger_id, challenged_id));
            }
            GE::ChallengedSuccess { challenger_id, challenged_id } => {
                app.log.push(format!("Challenge succeeded: {} challenged {}", 
                                        challenger_id, challenged_id));
            }
            GE::DirectionChanged { clockwise } => {
                app.log.push(format!("Game direction changed: {}", 
                                    if *clockwise { "clockwise" } else { "counter-clockwise" }));
            }
            GE::DrawFourApplied { target_player_id } => {
                app.log.push(format!("Draw Four applied to player {} ", 
                                    target_player_id));
            }
            GE::DrawTwoApplied { target_player_id} => {
                app.log.push(format!("Draw Two applied to player {} ", 
                                        target_player_id));
            }
            GE::DrawnCardPlayable { player_id } => {
                if *player_id == app.player_id.unwrap() {
                    app.log.push(format!("You can play drawn card"));
                    todo!(); // 这里可以添加逻辑处理玩家摸牌后可出的牌
                } 
            }
            GE::GameOver { winner, scores } => {
                app.log.push(format!("Game over! Winner: {}. Scores: {:?}", 
                                    winner, scores));
            }
            GE::PlayerChallenged { challenger_id, challenged_id } => {
                app.log.push(format!("Player {} challenged player {}", 
                                    challenger_id, challenged_id));
            }
            GE::PlayerPassed { player_id } => {
                app.log.push(format!("Player {} passed his turn", player_id));
            }
            GE::PlayerSkipped { player_id } => {
                println!("Player {} skipped his turn", player_id);
            }
            GE::PlayerTurn { player_id } => {
                app.log.push(format!("Player {}'s turn", player_id));
                todo!(); // 这里可以添加逻辑处理玩家轮到时的操作
            }
            GE::TopCardChanged { top_card } => {
                app.top_card = Some(*top_card);
                app.log.push(format!("Top card changed to {:?}", top_card));
            }
            GE::UnoCalled { player_id } => {
                app.log.push(format!("Player {} called UNO!", player_id));
            }
            GE::UnoPenalty { player_id } => {
                app.log.push(format!("Player {} received UNO penalty, drawing {} cards", 
                                    player_id, 2));
            }
        }
    }
       
}
