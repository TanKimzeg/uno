use crate::game::cards::Color;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum Client2Server {
    JoinGame {
        name: String,
        
    },
    StartGame {
        player_id: usize,
    },

    PlayCard {
        player_id: usize,
        card_index: usize,
        color: Color,
        call_uno: bool,
    },
    DrawCard {
        player_id: usize,
        count: usize,
    },
    PassTurn {
        player_id: usize,
    },
    ChallengeWildDrawFour {
        challenger_id: usize,
        challenged_id: usize,
    },

    LeaveGame {
        player_id: usize,
    },
}
