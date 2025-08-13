use futures::StreamExt;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{collections::HashMap, sync::Arc};
use tokio::{
    io::AsyncWriteExt,
    net::TcpListener,
    sync::{mpsc, RwLock},
    time::{Duration, Instant},
};
use tokio_util::codec::{FramedRead, LinesCodec};
use uno::game::events::GameEvent as GE;
use uno::game::UnoGame;
use uno::protocol::{Client2Server, Server2Client};

// ===== 房间与命令定义 =====
type RoomId = String;
type ConnId = u64;

#[derive(Debug)]
enum RoomCmd {
    Join {
        conn_id: ConnId,
        name: String,
        tx_client: mpsc::Sender<Server2Client>,
    },
    Leave {
        conn_id: ConnId,
    },
    GameMsg {
        conn_id: ConnId,
        msg: Client2Server,
    },
}

#[derive(Clone)]
struct RoomHandle {
    tx: mpsc::Sender<RoomCmd>,
}

#[derive(Clone)]
struct Rooms {
    inner: Arc<RwLock<HashMap<RoomId, RoomHandle>>>,
}
impl Rooms {
    fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    async fn get_or_create(&self, id: &str) -> RoomHandle {
        if let Some(h) = self.inner.read().await.get(id).cloned() {
            return h;
        }
        let (tx, rx) = mpsc::channel(256);
        let handle = RoomHandle { tx: tx.clone() };
        self.inner
            .write()
            .await
            .insert(id.to_string(), handle.clone());
        tokio::spawn(room_task(id.to_string(), rx, self.clone()));
        handle
    }
    async fn remove(&self, id: &str) {
        self.inner.write().await.remove(id);
    }
}

#[derive(Clone)]
struct PlayerSlot {
    conn_id: ConnId,
    pid: usize,
    name: String,
    tx: mpsc::Sender<Server2Client>,
}

macro_rules! log_ts { ($($arg:tt)*) => {{
    let dur = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let secs = dur.as_secs();
    let h = (secs / 3600) % 24;
    let m = (secs / 60) % 60;
    let s = secs % 60;
    eprintln!("[{h:02}:{m:02}:{s:02}] {}", format!($($arg)*));
}} }
// 简单房间级别日志辅助
fn room_log(room: &str, msg: &str) {
    log_ts!("room={} {}", room, msg);
}

async fn room_task(room_id: RoomId, mut rx: mpsc::Receiver<RoomCmd>, rooms: Rooms) {
    let mut game = UnoGame::new();
    let mut players: Vec<PlayerSlot> = Vec::new();
    let mut started = false;
    let mut last_active = Instant::now();
    let mut ticker = tokio::time::interval(Duration::from_secs(15));
    room_log(&room_id, "task started");
    loop {
        tokio::select! {
            _ = ticker.tick() => {
                if players.is_empty() && last_active.elapsed() > Duration::from_secs(30) {
                   room_log(&room_id, "idle timeout -> removing room");
                   rooms.remove(&room_id).await;
                    break;
                }
                let _ = rx.try_recv(); // 触发下轮select
            }
            cmd = rx.recv() => {
                let Some(cmd) = cmd else { break }; last_active = Instant::now();
                match cmd {
                    RoomCmd::Join { conn_id, name, tx_client } => {
                       room_log(&room_id, &format!("join conn={} name={} (players before={})", conn_id, name, players.len()));
                        let pid = players.len();
                        players.push(PlayerSlot { conn_id, pid, name: name.clone(), tx: tx_client.clone() });
                        let _ = tx_client.send(Server2Client::Welcome { player_id: pid, session_id: format!("{}-{}", room_id, pid) }).await;
                        broadcast_events(&players, vec![GE::PlayerJoined { player_id: pid, name }]).await;
                        sync_state(&players, &game).await;
                    }
                    RoomCmd::Leave { conn_id } => {
                       room_log(&room_id, &format!("leave conn={}", conn_id));
                        players.retain(|p| p.conn_id != conn_id);
                    }
                    RoomCmd::GameMsg { conn_id, msg } => {
                        handle_game_msg(&mut game, &mut started, &mut players, conn_id, msg).await;
                    }
                }
            }
        }
    }
    room_log(&room_id, "task ended");
}

async fn handle_game_msg(
    game: &mut UnoGame,
    started: &mut bool,
    players: &mut Vec<PlayerSlot>,
    conn_id: ConnId,
    msg: Client2Server,
) {
    use Client2Server::*;
    let find_pid =
        |v: &Vec<PlayerSlot>, cid: ConnId| v.iter().find(|p| p.conn_id == cid).map(|p| p.pid);
    let mut need_reset_after_sync = false; // GameOver 后同步一次旧状态再重置
    match msg {
        StartGame { player_id } => {
            if *started {
                send_err(players, conn_id, "Game already started").await;
                return;
            }
            if find_pid(players, conn_id) != Some(player_id) {
                send_err(players, conn_id, "Player mismatch").await;
                return;
            }
            let names: Vec<String> = players.iter().map(|p| p.name.clone()).collect();
            let ev = game.init_game(names);
            *started = true;
            if ev.iter().any(|e| matches!(e, GE::GameOver { .. })) {
                *started = false;
                need_reset_after_sync = true;
            }
            log_ts!(
                "start game players={} conn={} pid={}",
                players.len(),
                conn_id,
                player_id
            );
            broadcast_events(players, ev).await;
        }
        PlayCard {
            player_id,
            card_index,
            color,
            call_uno,
        } => {
            if !*started {
                send_err(players, conn_id, "Game not started").await;
                return;
            }
            if find_pid(players, conn_id) != Some(player_id) {
                send_err(players, conn_id, "Player mismatch").await;
                return;
            }
            if player_id != game.current_player {
                send_err(players, conn_id, "Not your turn").await;
                return;
            }
            let ev = game.play_card(player_id, card_index, call_uno, color);
            if ev.iter().any(|e| matches!(e, GE::GameOver { .. })) {
                *started = false;
                need_reset_after_sync = true;
            }
            if ev.iter().any(|e| matches!(e, GE::CardPlayed { .. })) {
                log_ts!(
                    "play conn={} pid={} card_index={} call_uno={}",
                    conn_id,
                    player_id,
                    card_index,
                    call_uno
                );
            }
            broadcast_events(players, ev).await;
        }
        DrawCard { player_id, count } => {
            if !*started {
                send_err(players, conn_id, "Game not started").await;
                return;
            }
            if find_pid(players, conn_id) != Some(player_id) {
                send_err(players, conn_id, "Player mismatch").await;
                return;
            }
            for _ in 0..count.max(1) {
                let ev = game.draw_card(player_id);
                if ev.iter().any(|e| matches!(e, GE::GameOver { .. })) {
                    *started = false;
                    need_reset_after_sync = true;
                }
                broadcast_events(players, ev).await;
            }
        }
        PassTurn { player_id } => {
            if find_pid(players, conn_id) != Some(player_id) {
                send_err(players, conn_id, "Player mismatch").await;
                return;
            }
            let ev = game.player_pass(player_id);
            if ev.iter().any(|e| matches!(e, GE::GameOver { .. })) {
                *started = false;
                need_reset_after_sync = true;
            }
            log_ts!("pass conn={} pid={}", conn_id, player_id);
            broadcast_events(players, ev).await;
        }
        LeaveGame { player_id: _ } => {}
        JoinGame { .. } => {
            send_err(players, conn_id, "Already in room").await;
        }
        ChallengeWildDrawFour { .. } => {
            send_err(players, conn_id, "Challenge not implemented").await
        }
    }
    sync_state(players, game).await;
    if need_reset_after_sync {
        log_ts!(
            "game over -> reset pending new StartGame (players={})",
            players.len()
        );
        *game = UnoGame::new(); // 清空牌局以便下一次 StartGame 重新 init
    }
}

async fn broadcast_events(players: &Vec<PlayerSlot>, events: Vec<GE>) {
    if events.is_empty() {
        return;
    }
    let msg = Server2Client::Events(events);
    for p in players {
        let _ = p.tx.send(msg.clone()).await;
    }
}
async fn sync_state(players: &Vec<PlayerSlot>, game: &UnoGame) {
    let shared = Server2Client::SharedState {
        players_cards_count: game.get_players_cards_count(),
        top_card: game.top_card,
        current_player: game.current_player,
        clockwise: game.direction,
    };
    for p in players {
        let _ = p.tx.send(shared.clone()).await;
        let _ =
            p.tx.send(Server2Client::PlayerState {
                player_id: p.pid,
                hand: game.get_player_hand(p.pid),
            })
            .await;
    }
}
async fn send_err(players: &Vec<PlayerSlot>, conn_id: ConnId, msg: &str) {
    if let Some(p) = players.iter().find(|p| p.conn_id == conn_id) {
        let _ =
            p.tx.send(Server2Client::ServerError {
                message: msg.to_string(),
            })
            .await;
    }
    log_ts!("error conn={} msg={} ", conn_id, msg);
}

// ===== 连接处理 =====
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let addr = "0.0.0.0:9000";
    let listener = TcpListener::bind(addr).await?;
    log_ts!("multi-room async UNO listening on {}", addr);
    let rooms = Rooms::new();
    let mut next_conn: ConnId = 0;
    loop {
        let (stream, peer) = listener.accept().await?;
        next_conn += 1;
        let conn_id = next_conn;
        log_ts!("accept conn={} from {}", conn_id, peer);
        let rooms_cl = rooms.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_connection(stream, rooms_cl, conn_id).await {
                eprintln!("conn {} error: {}", conn_id, e);
            }
        });
    }
}

async fn handle_connection(
    stream: tokio::net::TcpStream,
    rooms: Rooms,
    conn_id: ConnId,
) -> anyhow::Result<()> {
    let (r, w) = stream.into_split();
    let mut lines = FramedRead::new(r, LinesCodec::new());
    let (tx_client, mut rx_client) = mpsc::channel::<Server2Client>(256);
    // 写任务
    tokio::spawn(async move {
        let mut writer = tokio::io::BufWriter::new(w);
        while let Some(msg) = rx_client.recv().await {
            if let Ok(line) = serde_json::to_string(&msg) {
                if writer.write_all(line.as_bytes()).await.is_err() {
                    break;
                }
                if writer.write_all(b"\n").await.is_err() {
                    break;
                }
                let _ = writer.flush().await;
            }
        }
    });
    let mut room_tx: Option<mpsc::Sender<RoomCmd>> = None;
    while let Some(line) = lines.next().await {
        let line = line?;
        let parsed: Result<Client2Server, _> = serde_json::from_str(&line);
        let msg = match parsed {
            Ok(m) => m,
            Err(e) => {
                let _ = tx_client
                    .send(Server2Client::ServerError {
                        message: format!("bad json: {}", e),
                    })
                    .await;
                log_ts!("conn={} bad json error={}", conn_id, e);
                continue;
            }
        };
        match (&room_tx, &msg) {
            (None, Client2Server::JoinGame { room_id, name }) => {
                log_ts!(
                    "conn={} join request room={} name={} ",
                    conn_id,
                    room_id,
                    name
                );
                let handle = rooms.get_or_create(room_id).await;
                room_tx = Some(handle.tx.clone());
                let _ = handle
                    .tx
                    .send(RoomCmd::Join {
                        conn_id,
                        name: name.clone(),
                        tx_client: tx_client.clone(),
                    })
                    .await;
            }
            (None, _) => {
                let _ = tx_client
                    .send(Server2Client::ServerError {
                        message: "First message must be JoinGame {room_id,name}".into(),
                    })
                    .await;
            }
            (Some(_), Client2Server::JoinGame { .. }) => {
                let _ = tx_client
                    .send(Server2Client::ServerError {
                        message: "Already joined".into(),
                    })
                    .await;
            }
            (Some(tx_room), other) => {
                let _ = tx_room
                    .send(RoomCmd::GameMsg {
                        conn_id,
                        msg: other.clone(),
                    })
                    .await;
            }
        }
    }
    if let Some(tx_room) = room_tx {
        let _ = tx_room.send(RoomCmd::Leave { conn_id }).await;
    }
    Ok(())
}
