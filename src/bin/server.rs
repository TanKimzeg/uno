use rand::{distributions::Alphanumeric, Rng};
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

use uno::game::events::GameEvent as GE;
use uno::game::UnoGame;
use uno::ports::bus::{ConsolerLogger, EventBus, EventHandler};
use uno::protocol::{Client2Server, Server2Client};

type ClientTx = mpsc::Sender<Server2Client>;
type ClientRx = mpsc::Receiver<Server2Client>;

struct SharedState {
    game: UnoGame,
    players: Vec<String>,
    // game_id: String,
    clients: Vec<(ClientTx, Option<usize>)>, // 广播通道
}

// 网络广播处理器：把每个 GameEvent 发送给所有客户端
struct BroadcastHandler {
    state: Arc<Mutex<SharedState>>,
}
impl EventHandler for BroadcastHandler {
    /// Handle a batch of game events and broadcast them to all clients.
    fn handle_events(&self, events: &[GE]) {
        let msg = Server2Client::Events(events.to_vec());
        let mut dead = Vec::new();
        let mut st = self.state.lock().unwrap();
        for (i, (tx, _pid)) in st.clients.iter().enumerate() {
            if tx.send(msg.clone()).is_err() {
                dead.push(i);
            }
        }
        for i in dead.into_iter().rev() {
            st.clients.remove(i);
        }
        let shared_state = Server2Client::SharedState {
            players_cards_count: st.game.get_players_cards_count(),
            top_card: st.game.top_card,
            current_player: st.game.current_player,
            clockwise: st.game.direction,
        };
        for (cl, pid_opt) in st.clients.iter() {
            if let Some(pid) = pid_opt {
                let _ = cl.send(Server2Client::PlayerState {
                    player_id: *pid,
                    hand: st.game.get_player_hand(*pid),
                });
            }
            let _ = cl.send(shared_state.clone());
        }
    }
}

fn main() {
    let addr = "127.0.0.1:9000";
    let listener = TcpListener::bind(addr).expect("bind failed");
    println!("UNO server listening on {}", addr);

    // 共享状态
    let state = Arc::new(Mutex::new(SharedState {
        game: UnoGame::new(),
        players: Vec::new(),
        // game_id: gen_id(10),
        clients: Vec::new(),
    }));

    // 事件总线：注册网络广播处理器
    let mut bus = EventBus::new();
    bus.register_handler(Box::new(BroadcastHandler {
        state: state.clone(),
    }));
    // 注册控制台日志处理器
    bus.register_handler(Box::new(ConsolerLogger {}));
    let bus = Arc::new(bus); // 只读共享，后续不再注册新处理器

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let state_clone = state.clone();
                let bus_clone = bus.clone();
                let (tx, rx): (ClientTx, ClientRx) = mpsc::channel();
                // 插入到clients并记录本连接索引
                let conn_index = {
                    let mut st = state_clone.lock().unwrap();
                    st.clients.push((tx.clone(), None));
                    st.clients.len() - 1
                };
                // 写线程
                let mut write_stream = stream.try_clone().expect("clone stream failed");
                thread::spawn(move || writer_loop(&mut write_stream, rx));
                // 读线程（带 conn_index）
                thread::spawn(move || reader_loop(stream, state_clone, bus_clone, tx, conn_index));
            }
            Err(e) => eprintln!("accept error: {}", e),
        }
    }
}

fn writer_loop(stream: &mut TcpStream, rx: ClientRx) {
    for msg in rx {
        if let Ok(line) = serde_json::to_string(&msg) {
            if let Err(e) = writeln!(stream, "{}", line) {
                eprintln!("write error: {}", e);
                break;
            }
            let _ = stream.flush();
        }
    }
}

fn reader_loop(
    stream: TcpStream,
    state: Arc<Mutex<SharedState>>,
    bus: Arc<EventBus>,
    my_tx: ClientTx,
    conn_index: usize,
) {
    let peer = stream.peer_addr().ok();
    let reader = BufReader::new(stream);
    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                eprintln!("read error from {:?}: {}", peer, e);
                break;
            }
        };
        let msg: Result<Client2Server, _> = serde_json::from_str(&line);
        match msg {
            Ok(m) => handle_message(&state, &bus, &my_tx, m, conn_index),
            Err(e) => {
                let _ = my_tx.send(Server2Client::ServerError {
                    message: format!("bad json: {}", e),
                });
            }
        }
    }
}

fn handle_message(
    state: &Arc<Mutex<SharedState>>,
    bus: &Arc<EventBus>,
    my_tx: &ClientTx,
    msg: Client2Server,
    conn_index: usize,
) {
    match msg {
        Client2Server::JoinGame { name } => {
            {
                // 已开始校验
                let st = state.lock().unwrap();
                if st.game.started {
                    let _ = my_tx.send(Server2Client::ServerError {
                        message: "Game already started".into(),
                    });
                    return;
                }
            }
            let welcome = {
                let mut st = state.lock().unwrap();
                // 已加入则拒绝
                if let Some((_tx, Some(_))) = st.clients.get(conn_index) {
                    let _ = my_tx.send(Server2Client::ServerError {
                        message: "Already joined".into(),
                    });
                    return;
                }
                let player_id = st.players.len();
                st.players.push(name.clone());
                if let Some((_tx, pid_slot)) = st.clients.get_mut(conn_index) {
                    *pid_slot = Some(player_id);
                }
                let session_id = gen_id(12);
                Server2Client::Welcome {
                    player_id,
                    session_id,
                }
            };
            let _ = my_tx.send(welcome);
        }
        Client2Server::StartGame { player_id } => {
            if !connection_claim_matches(state, conn_index, player_id) {
                let _ = my_tx.send(Server2Client::ServerError {
                    message: "Player ID mismatch or not joined".into(),
                });
                return;
            }
            {
                let st = state.lock().unwrap();
                if st.game.started {
                    let _ = my_tx.send(Server2Client::ServerError {
                        message: "Game already started".into(),
                    });
                    return;
                }
            }
            let _ = my_tx.send(Server2Client::ServerError {
                message: format!("You start the game!").into(),
            });
            let ev = {
                let mut st = state.lock().unwrap();
                let players = st.players.clone();
                st.game.init_game(players)
            };
            bus.publish(ev);
        }
        Client2Server::PlayCard {
            player_id,
            card_index,
            color,
            call_uno,
        } => {
            {
                let st = state.lock().unwrap();
                if !st.game.started {
                    let _ = my_tx.send(Server2Client::ServerError {
                        message: "Game not started yet".into(),
                    });
                    return;
                }
                if player_id != st.game.current_player {
                    let _ = my_tx.send(Server2Client::ServerError {
                        message: "Not your turn".into(),
                    });
                    return;
                }
            }
            if !connection_claim_matches(state, conn_index, player_id) {
                let _ = my_tx.send(Server2Client::ServerError {
                    message: "Player ID mismatch or not joined".into(),
                });
                return;
            }
            let events = {
                let mut st = state.lock().unwrap();
                st.game.play_card(player_id, card_index, call_uno, color)
            };
            {
                let st = state.lock().unwrap();
                let _ = my_tx.send(Server2Client::PlayerState {
                    player_id,
                    hand: st.game.get_player_hand(player_id),
                });
            }
            bus.publish(events);
        }
        Client2Server::DrawCard { player_id, count } => {
            {
                let st = state.lock().unwrap();
                if !st.game.started {
                    let _ = my_tx.send(Server2Client::ServerError {
                        message: "Game not started yet".into(),
                    });
                }
            }
            if !connection_claim_matches(state, conn_index, player_id) {
                let _ = my_tx.send(Server2Client::ServerError {
                    message: "Player ID mismatch or not joined".into(),
                });
                return;
            }
            let n = count.max(1);
            for _ in 0..n {
                let ev = {
                    let mut st = state.lock().unwrap();
                    st.game.draw_card(player_id)
                };
                bus.publish(ev);
            }
        }
        Client2Server::PassTurn { player_id } => {
            {
                let st = state.lock().unwrap();
                if !st.game.started {
                    let _ = my_tx.send(Server2Client::ServerError {
                        message: "Game not started yet".into(),
                    });
                }
            }
            if !connection_claim_matches(state, conn_index, player_id) {
                let _ = my_tx.send(Server2Client::ServerError {
                    message: "Player ID mismatch or not joined".into(),
                });
                return;
            }
            let ev = {
                let mut st = state.lock().unwrap();
                st.game.player_pass(player_id)
            };
            bus.publish(ev);
        }
        Client2Server::ChallengeWildDrawFour { .. } => {
            let _ = my_tx.send(Server2Client::ServerError {
                message: "Challenge (+4) 尚未实现".into(),
            });
        }
        Client2Server::LeaveGame { player_id } => {
            if !connection_claim_matches(state, conn_index, player_id) {
                let _ = my_tx.send(Server2Client::ServerError {
                    message: "Player ID mismatch or not joined".into(),
                });
                return;
            }
            let mut st = state.lock().unwrap();
            if player_id >= st.players.len() {
                let _ = my_tx.send(Server2Client::ServerError {
                    message: "Invalid player ID".into(),
                });
                return;
            }
            st.players.remove(player_id);
            // 不立即移除clients以免打乱索引，可标记None
            if let Some((_tx, pid_slot)) = st.clients.get_mut(conn_index) {
                *pid_slot = None;
            }
        }
    }
}

/// 生成随机字符串 ID, 用于游戏 ID 或会话 ID
/// 目前, 基于ID的校验机制还未实现
fn gen_id(len: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}

fn connection_claim_matches(
    state: &Arc<Mutex<SharedState>>,
    conn_index: usize,
    claimed: usize,
) -> bool {
    let st = state.lock().unwrap();
    if let Some((_, Some(pid))) = st.clients.get(conn_index) {
        *pid == claimed
    } else {
        false
    }
}
