use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use rand::{distributions::Alphanumeric, Rng};

use uno::game::UnoGame;
use uno::game::events::GameEvent as GE;
use uno::protocol::{Client2Server, Server2Client};
use uno::ports::bus::{EventBus, EventHandler, ConsolerLogger };

type ClientTx = mpsc::Sender<Server2Client>;
type ClientRx = mpsc::Receiver<Server2Client>;

struct SharedState {
    game: UnoGame,
    players: Vec<String>, 
    started: bool,
    // game_id: String, 
    clients: Vec<ClientTx>, // 广播通道
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
        for (i, tx) in st.clients.iter().enumerate() {
            if tx.send(msg.clone()).is_err() {
                dead.push(i);
            }
        }
        // 清理断开的连接
        for i in dead.into_iter().rev() {
            eprintln!("Client {} disconnected, removing from broadcast list", i);
            st.clients.remove(i);
        }

        // 广播全局共享信息
        let shared_state = Server2Client::SharedState {
            players_cards_count: st.game.get_players_cards_count(),
            top_card: st.game.top_card,
            current_player: st.game.current_player,
            clockwise: st.game.direction,
        };
        for tx in &st.clients {
            let _ = tx.send(shared_state.clone());
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
        started: false,
        // game_id: gen_id(10),
        clients: Vec::new(),
    }));

    // 事件总线：注册网络广播处理器
    let mut bus = EventBus::new();
    bus.register_handler(Box::new(BroadcastHandler { state: state.clone() }));
    // 注册控制台日志处理器
    bus.register_handler(Box::new(ConsolerLogger {}));
    let bus = Arc::new(bus); // 只读共享，后续不再注册新处理器

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let state = state.clone();
                let bus = bus.clone();

                // 每个客户端一个发送队列（服务器→客户端）
                let (tx, rx): (ClientTx, ClientRx) = mpsc::channel();
                {
                    let mut st = state.lock().unwrap();
                    st.clients.push(tx.clone());
                }

                // 写线程
                let mut write_stream = stream.try_clone().expect("clone stream failed");
                thread::spawn(move || writer_loop(&mut write_stream, rx));

                // 读线程
                thread::spawn(move || reader_loop(stream, state, bus, tx));
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

fn reader_loop(stream: TcpStream, state: Arc<Mutex<SharedState>>, bus: Arc<EventBus>, my_tx: ClientTx) {
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
            Ok(m) => handle_message(&state, &bus, &my_tx, m),
            Err(e) => {
                let _ = my_tx.send(Server2Client::ServerError {
                    message: format!("bad json: {}", e),
                });
            }
        }
    }
}

fn handle_message(state: &Arc<Mutex<SharedState>>, bus: &Arc<EventBus>, my_tx: &ClientTx, msg: Client2Server) {
    match msg {
        Client2Server::JoinGame { name } => {
            // 生成欢迎与可能的开局事件（注意先释放锁再 publish）
            let welcome = {
                let mut st = state.lock().unwrap();
                let player_id = st.players.len();
                st.players.push(name.clone());
                let session_id = gen_id(12);
                let welcome = Server2Client::Welcome { player_id, session_id };
                welcome
            };

            // welcome 回给自己
            let _ = my_tx.send(welcome);
        }

        Client2Server::StartGame { player_id } => {
            let _ = my_tx.send(Server2Client::ServerError {
                message: format!("{} starts the game!", player_id).into(),
            });
            let mut st = state.lock().unwrap();
            if st.started {
                let _ = my_tx.send(Server2Client::ServerError {
                    message: "Game already started".into(),
                });
                return;
            }
            let ev = {
                let players = st.players.clone();
                st.started = true;
                st.game.init_game(players)
            };
            bus.publish(ev);
        }

        Client2Server::PlayCard { player_id, card_index, color, call_uno } => {
            let events = {
                let mut st = state.lock().unwrap();
                st.game.play_card(player_id, card_index, call_uno, color)
            };
            bus.publish(events);
        }

        Client2Server::DrawCard { player_id, count } => {
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
            let mut st = state.lock().unwrap();
            if player_id >= st.players.len() {
                let _ = my_tx.send(Server2Client::ServerError {
                    message: "Invalid player ID".into(),
                });
                return;
            }
            st.players.remove(player_id);
            st.clients.remove(player_id);
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