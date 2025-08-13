use std::fmt::Display;

use crate::game::cards::UnoCard;
use serde::{Serialize, Deserialize};

#[derive(Clone, Serialize, Deserialize)]
pub enum GameEvent {
    PlayerJoined {
        player_id: usize,
        name: String,
    },
    GameStarted {
        game_id: usize,
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

impl Display for GameEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GameEvent::PlayerJoined { 
                player_id, name } => 
                write!(f, "PlayerJoined: id={}, name={}", player_id, name)?,
            GameEvent::GameStarted { 
                game_id } => 
                write!(f, "GameStarted: game_id={}", game_id)?,
            GameEvent::CardPlayed { 
                player_id, card } => 
                write!(f, "CardPlayed: id={}, card={}", player_id, card.to_string())?,
            GameEvent::CardDraw { 
                player_id, card } => 
                write!(f, "CardDraw: id={}, card={}", player_id, card.to_string())?,
            GameEvent::DrawnCardPlayable { 
                player_id } => 
                write!(f, "DrawnCardPlayable: id={}", player_id)?,
            GameEvent::PlayerPassed { 
                player_id } => 
                write!(f, "PlayerPassed: id={}", player_id)?,
            GameEvent::UnoCalled { player_id } => 
                write!(f, "UnoCalled: id={}", player_id)?,
            GameEvent::DirectionChanged { clockwise } => 
                write!(f, "DirectionChanged: clockwise={}", clockwise)?,
            GameEvent::TopCardChanged { top_card } => 
                write!(f, "TopCardChanged: top_card={}", top_card.to_string())?,
            GameEvent::PlayerTurn { player_id } => 
                write!(f, "PlayerTurn: id={}", player_id)?,
            GameEvent::PlayerSkipped { player_id } => 
                write!(f, "PlayerSkipped: id={}", player_id)?,
            GameEvent::DrawFourApplied { target_player_id } => 
                write!(f, "DrawFourApplied: target_id={}", target_player_id)?,
            GameEvent::DrawTwoApplied { target_player_id } => 
                write!(f, "DrawTwoApplied: target_id={}", target_player_id)?,
            GameEvent::PlayerChallenged { challenger_id, challenged_id } => 
                write!(f, "PlayerChallenged: challenger_id={}, challenged_id={}", challenger_id, challenged_id)?,
            GameEvent::ChallengedFailed { challenger_id, challenged_id } => 
                write!(f, "ChallengedFailed: challenger_id={}, challenged_id={}", challenger_id, challenged_id)?,
            GameEvent::ChallengedSuccess { challenger_id, challenged_id } => 
                write!(f, "ChallengedSuccess: challenger_id={}, challenged_id={}", challenger_id, challenged_id)?,
            GameEvent::UnoPenalty { player_id } => 
                write!(f, "UnoPenalty: id={}", player_id)?,
            GameEvent::GameOver { winner, scores } => 
                write!(f, "GameOver: winner={}, scores={:?}", winner, scores)?,
            GameEvent::GameError { message } => 
                write!(f, "GameError: message={}", message)?,
        }
        Ok(())
    }
}
