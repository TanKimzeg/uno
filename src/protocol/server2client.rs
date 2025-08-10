use serde::{Serialize, Deserialize};
use crate::game::events::GameEvent;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum Server2Client {
    Welcome {
        player_id: usize,
        session_id: String,
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