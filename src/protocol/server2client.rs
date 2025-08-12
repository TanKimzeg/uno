use serde::{Serialize, Deserialize};
use crate::game::{events::GameEvent, UnoCard};

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum Server2Client {
    Welcome {
        player_id: usize,
        session_id: String,
    },
    SharedState {
        players_cards_count: Vec<(String, usize)>, // (name, cards_count)
        top_card: Option<UnoCard>,
        current_player: usize,
        clockwise: bool,
    },
    PlayerState {
        player_id: usize,
        hand: Vec<UnoCard>,
    },
    
    GameStarted {
        game_id: String,
        players: Vec<String>,
    },
    Events(Vec<GameEvent>),
    ServerError {
        message: String,
    },

}