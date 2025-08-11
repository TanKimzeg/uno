use crate::game::cards::UnoCard;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameEvent {
    PlayerJoined {
        player_id: usize,
        name: String,
    },

    CardPlayed {
        player_id: usize,
        card: UnoCard,
    },
    CardDraw {
        player_id: usize,
        card: UnoCard,
    },

    // Player has no cards to play and has drawn a card
    DrawnCardPlayable {
        player_id: usize,
    },
    PlayerPassed {
        player_id: usize,
    },

    UnoCalled {
        player_id: usize,
    },
    DirectionChanged {
        clockwise: bool,
    },
    TopCardChanged {
        top_card: UnoCard,
    },
    PlayerTurn {
        player_id: usize,
    },
    PlayerSkipped {
        player_id: usize,
    },
    DrawFourApplied {
        target_player_id: usize,
    },
    DrawTwoApplied {
        target_player_id: usize,
    },

    PlayerChallenged {
        challenger_id: usize,
        challenged_id: usize,
    },
    ChallengedFailed {
        challenger_id: usize,
        challenged_id: usize,
    },
    ChallengedSuccess {
        challenger_id: usize,
        challenged_id: usize,
    },
    UnoPenalty {
        player_id: usize,
    },


    GameOver {
        winner: usize,
        scores: Vec<(String, i32)>,
    },
    GameError {
        message: String,
    },
}
