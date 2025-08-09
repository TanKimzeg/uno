use std::fmt::Display;
use std::io;
use colored::Colorize;

use crate::cards::*;

pub struct Player {
    pub name: String,
    pub hand: Vec<UnoCard>,
}

impl Player {
    pub fn new(name: &str) -> Player {
        Player {
            name: name.to_string(),
            hand: Vec::new(),
        }
    }

    pub fn display_hand(&self) -> &Vec<UnoCard> {
        &self.hand
    }

    pub fn can_play_card(&mut self, card_idx: usize, top_card: Option<&UnoCard>) -> Result<UnoCard, String> {
        if card_idx >= self.hand.len() {
            return Err("Invalid card index".to_string());
        }
        let card = self.hand.get(card_idx).unwrap();
        if valid_card(&card, top_card) {
            let card = card.clone();
            self.hand.remove(card_idx);
            Ok(card)
        } else {
            Err("Cannot play this card".to_string())
        }
    }

    pub fn select_color(&self, uno_card: &UnoCard) -> UnoCard {
        match uno_card {
            UnoCard::WildCard(_, wild_type) => {
                let mut wild_card = WildCard {
                    color: None,
                    wild_type: *wild_type,
                };
                println!(
                    "Select a color ({}, {}, {}, {}): ",
                    "0: Red".red(),
                    "1: Green".green(),
                    "2: Blue".blue(),
                    "3: Yellow".yellow()
                );
                let mut input = String::new();
                loop {
                    match io::stdin().read_line(&mut input) {
                        Ok(_) => if let Ok(color_idx) = input.trim().parse::<usize>() {
                            match color_idx {
                                0 => {wild_card.color = Some(Color::RED); break;}
                                1 => {wild_card.color = Some(Color::GREEN); break;}
                                2 => {wild_card.color = Some(Color::BLUE); break;}
                                3 => {wild_card.color = Some(Color::YELLOW); break;}
                                _ => continue, // 无效输入，重新输入
                            }
                        } 
                        Err(_) => {
                            println!("Error reading input, please try again.");
                            input.clear();
                            continue;
                        }
                    }
                }
                UnoCard::WildCard(wild_card.color, wild_card.wild_type)
            }
            _ => panic!("This method should only be called for wild cards."),
        }
    }

    pub fn draw_cards(&mut self, deck: &mut UnoDeck, num_cards: usize) {
        for _ in 0..num_cards {
            if let Some(card) = deck.cards.pop() {
                self.hand.push(card);
            } else {
                // eprintln!("No more cards in the deck to draw!");
                panic!("No more cards in the deck to draw!");
            }
        }
    }

    pub fn want_to_play(&self) -> Option<usize> {
        // 显示当前玩家的手牌
        println!("{}'s turn:", self.name);
        println!("{}",self.to_string());
        loop {
            println!("Which card do you want to play? (0 to {}): ", self.hand.len() - 1);
            let mut input = String::new();
            match io::stdin().read_line(&mut input) {
                Ok(_) => {
                    match input.trim().parse::<i32>() {
                        Ok(idx) => {
                            if idx >= 0 && (idx as usize) < self.hand.len() {
                                return Some(idx as usize);
                            } else {
                                return None;
                            }
                        }
                        _ => println!("Invalid input, please enter a valid card index."),
                    }
                }
                Err(_) => println!("Error reading input, please try again."),
            }
        }
    }

}

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