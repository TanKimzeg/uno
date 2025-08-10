use std::fmt::Display;
use crate::game::cards::UnoCard;

pub struct Player {
    pub name: String,
    hand: Vec<UnoCard>,
    pub id: usize,
}


impl Player {
    pub fn new(name: &str, id: usize) -> Player {
        Player {
            name: name.to_string(),
            hand: Vec::new(),
            id,
        }
    }

    pub fn push_card(&mut self, card: UnoCard) {
        self.hand.push(card);
    }

    pub fn display_hand(&self) -> &Vec<UnoCard> {
        &self.hand
    }

    pub fn remove_card(&mut self, card_idx: usize) -> Result<UnoCard, String> {
        if card_idx >= self.hand.len() {
            return Err("Invalid card index".to_string());
        }
        Ok(self.hand.remove(card_idx))
    }
}
// pub struct Player {
//     pub name: String,
//     hand: Vec<UnoCard>,
//     client: Box<dyn Client>,
// }

// impl Player {
//     pub fn new(name: &str, client: Box<dyn Client>) -> Player {
//         Player {
//             name: name.to_string(),
//             hand: Vec::new(),
//             client,
//         }
//     }

//     pub fn display_hand(&self) -> &Vec<UnoCard> {
//         &self.hand
//     }

//     pub fn push_card(&mut self, card: UnoCard) {
//         self.hand.push(card);
//     }

//     // pub fn can_play_card(&mut self, card_idx: usize, top_card: Option<&UnoCard>) -> Result<UnoCard, String> {
//     //     if card_idx >= self.hand.len() {
//     //         return Err("Invalid card index".to_string());
//     //     }
//     //     let card = self.hand.get(card_idx).unwrap();
//     //     if valid_card(&card, top_card) {
//     //         let card = card.clone();
//     //         self.hand.remove(card_idx);
//     //         Ok(card)
//     //     } else {
//     //         Err("Cannot play this card".to_string())
//     //     }
//     // }

//     pub fn select_color(&self, uno_card: &UnoCard) -> UnoCard {
//         match uno_card {
//             UnoCard::WildCard(_, wild_type) => {
//                 let mut wild_card = WildCard {
//                     color: None,
//                     wild_type: *wild_type,
//                 };
//                 wild_card.color = Some(self.client.select_color());
//                 UnoCard::WildCard(wild_card.color, wild_card.wild_type)
//             }
//             _ => panic!("This method should only be called for wild cards."),
//         }
//     }

//     pub fn draw_cards(&mut self, deck: &mut UnoDeck, num_cards: usize) {
//         for _ in 0..num_cards {
//             if let Some(card) = deck.cards.pop() {
//                 self.hand.push(card);
//             } else {
//                 // eprintln!("No more cards in the deck to draw!");
//                 panic!("No more cards in the deck to draw!");
//             }
//         }
//     }

//     pub fn want_to_play(&self) -> (Option<usize>, bool) {
//         // 显示当前玩家的手牌
//         println!("{}'s turn:", self.name);
//         println!("{}",self.to_string());
//         self.client.want_to_play(self.display_hand().len())
//     }

// }

impl Display for Player {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "====================================")?;
        writeln!(f, "Player: {:<20}", self.name)?;
        writeln!(f, "------------------------------------")?;
        writeln!(f, "{:<5} | {:<15}", "Index", "Card")?;
        writeln!(f, "------------------------------------")?;
        writeln!(f, "{:>5} | {:<15}","-1", "  Draw Card")?;
        for (idx, card) in self.hand.iter().enumerate() {
            writeln!(f, "{:>5} | {:<15}", idx, card)?;
        }
        writeln!(f, "====================================")
    }
}

