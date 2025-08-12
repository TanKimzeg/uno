// use colored::Colorize;
use std::fmt::Display;
use serde::{Serialize, Deserialize};

#[derive(PartialEq, Clone, Copy, Serialize, Deserialize)]
pub enum UnoCard {
    NumberCard(Color, Number),
    ActionCard(Color, Action),
    WildCard(Option<Color>, WildType),
}

impl UnoCard {
    pub fn get_color(&self) -> Result<&Color, &str> {
        match self {
            UnoCard::NumberCard(color, _) => Ok(color),
            UnoCard::ActionCard(color, _) => Ok(color),
            UnoCard::WildCard(color, _) => color
                .as_ref()
                .ok_or("Wild card must have a color when used!"),
        }
    }

    pub fn get_number(&self) -> Option<&Number> {
        match self {
            UnoCard::NumberCard(_, number) => Some(number),
            UnoCard::ActionCard(_, _) => None,
            UnoCard::WildCard(_, _) => None,
        }
    }

    pub fn get_value(&self) -> i32 {
        // 计算剩余牌所代表的分数作为自己的负分
        // 0-9数字牌计0-9分，功能牌计20分，万能牌计50分
        // 负分最少的为最大赢家。
        match self {
            UnoCard::NumberCard(_, number) => number.to_u8() as i32,
            UnoCard::WildCard(_, _) => 50,
            UnoCard::ActionCard(_, _) => 20,
        }
    }
}

impl Display for UnoCard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnoCard::NumberCard(color, number) => write!(
                f,
                "Number Card: {} {}",
                color.to_string(),
                number.to_string()
            )?,
            UnoCard::ActionCard(color, action) => {
                write!(f, "Action Card: {} {}", color.to_string(), action.to_string())?
            }
            UnoCard::WildCard(color, wild_type) => {
                if let Some(c) = color {
                    write!(f, "  Wild Card: {} {}", c.to_string(), wild_type.to_string())?
                } else {
                    write!(f, "  Wild Card: {}", wild_type.to_string())?
                }
            }
        }
        Ok(())
    }
}

pub fn valid_card(card: &UnoCard, top_card: &Option<UnoCard>) -> bool {
    if let Some(top_card) = top_card {
        if is_wild_card(card) {
            return true;
        }
        if same_color(card, top_card) || same_number(card, top_card) || same_action(card, top_card)
        {
            return true;
        }
    } else {
        // If there is no top card, any card can be played
        return true;
    }
    false
}

fn is_wild_card(card: &UnoCard) -> bool {
    match card {
        UnoCard::WildCard(_, _) => true,
        _ => false,
    }
}

fn same_color(card: &UnoCard, top_card: &UnoCard) -> bool {
    if card.get_color().ok() == top_card.get_color().ok() {
        return true;
    }
    false
}

fn same_number(card: &UnoCard, top_card: &UnoCard) -> bool {
    let card_num = card.get_number();
    let top_card_num = top_card.get_number();
    if card_num == None || top_card_num == None || card.get_number() != top_card.get_number() {
        return false;
    }
    true
}

fn same_action(card: &UnoCard, top_card: &UnoCard) -> bool {
    match (card, top_card) {
        (UnoCard::ActionCard(_, action1), UnoCard::ActionCard(_, action2)) => action1 == action2,
        _ => false,
    }
}

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub enum Color {
    RED,
    GREEN,
    BLUE,
    YELLOW,
}

impl Color {
    pub fn from_u8(val: u8) -> Option<Color> {
        match val {
            0 => Some(Color::RED),
            1 => Some(Color::GREEN),
            2 => Some(Color::BLUE),
            3 => Some(Color::YELLOW),
            _ => None,
        }
    }

    pub fn to_u8(&self) -> u8 {
        match self {
            Color::RED => 0,
            Color::GREEN => 1,
            Color::BLUE => 2,
            Color::YELLOW => 3,
        }
    }
}

impl Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // Color::RED => write!(f, "{:>6}", "RED".red())?,
            // Color::GREEN => write!(f, "{:>6}", "GREEN".green())?,
            // Color::BLUE => write!(f, "{:>6}", "BLUE".blue())?,
            // Color::YELLOW => write!(f, "{:>6}", "YELLOW".yellow())?,
            Color::RED => write!(f, "{:>6}", "RED")?,
            Color::GREEN => write!(f, "{:>6}", "GREEN")?,
            Color::BLUE => write!(f, "{:>6}", "BLUE")?,
            Color::YELLOW => write!(f, "{:>6}", "YELLOW")?,
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub enum Number {
    ZERO,
    ONE,
    TWO,
    THREE,
    FOUR,
    FIVE,
    SIX,
    SEVEN,
    EIGHT,
    NINE,
}

impl Number {
    pub fn from_u8(val: u8) -> Option<Number> {
        match val {
            0 => Some(Number::ZERO),
            1 => Some(Number::ONE),
            2 => Some(Number::TWO),
            3 => Some(Number::THREE),
            4 => Some(Number::FOUR),
            5 => Some(Number::FIVE),
            6 => Some(Number::SIX),
            7 => Some(Number::SEVEN),
            8 => Some(Number::EIGHT),
            9 => Some(Number::NINE),
            _ => None,
        }
    }

    pub fn to_u8(&self) -> u8 {
        match self {
            Number::ZERO => 0,
            Number::ONE => 1,
            Number::TWO => 2,
            Number::THREE => 3,
            Number::FOUR => 4,
            Number::FIVE => 5,
            Number::SIX => 6,
            Number::SEVEN => 7,
            Number::EIGHT => 8,
            Number::NINE => 9,
        }
    }
}

impl Display for Number {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // Number::ZERO => write!(f, "{}", "0".white())?,
            // Number::ONE => write!(f, "{}", "1".bright_blue())?,
            // Number::TWO => write!(f, "{}", "2".bright_green())?,
            // Number::THREE => write!(f, "{}", "3".bright_red())?,
            // Number::FOUR => write!(f, "{}", "4".bright_yellow())?,
            // Number::FIVE => write!(f, "{}", "5".bright_cyan())?,
            // Number::SIX => write!(f, "{}", "6".bright_magenta())?,
            // Number::SEVEN => write!(f, "{}", "7".blue())?,
            // Number::EIGHT => write!(f, "{}", "8".green())?,
            // Number::NINE => write!(f, "{}", "9".red())?,
            Number::ZERO => write!(f, "{}", "0")?,
            Number::ONE => write!(f, "{}", "1")?,
            Number::TWO => write!(f, "{}", "2")?,
            Number::THREE => write!(f, "{}", "3")?,
            Number::FOUR => write!(f, "{}", "4")?,
            Number::FIVE => write!(f, "{}", "5")?,
            Number::SIX => write!(f, "{}", "6")?,
            Number::SEVEN => write!(f, "{}", "7")?,
            Number::EIGHT => write!(f, "{}", "8")?,
            Number::NINE => write!(f, "{}", "9")?,
        }
        Ok(())
    }
}

pub struct NumberCard {
    pub color: Color,
    pub number: Number,
}

pub struct ActionCard {
    pub color: Color,
    pub action: Action,
}

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub enum Action {
    SKIP,
    REVERSE,
    DRAWTWO,
}

impl Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Action::SKIP => write!(f, "{:<7}", "SKIP")?,
            Action::REVERSE => write!(f, "{:<7}", "REVERSE")?,
            Action::DRAWTWO => write!(f, "{:<7}", "DRAWTWO")?,
        }
        Ok(())
    }
    
}

pub struct WildCard {
    pub color: Option<Color>,
    pub wild_type: WildType,
}

#[derive(PartialEq, Clone, Copy, Serialize, Deserialize)]
pub enum WildType {
    WILD,
    DRAWFOUR,
}

impl Display for WildType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WildType::WILD => write!(f, "{:<7}", "WILD")?,
            WildType::DRAWFOUR => write!(f, "{:<7}", "DRAWFOUR")?,
        }
        Ok(())
    }
    
}

pub struct UnoDeck {
    // number_cards: Vec<NumberCard>,
    // action_cards: Vec<ActionCard>,
    // wild_cards: Vec<WildCard>,
    pub cards: Vec<UnoCard>,
}

impl UnoDeck {
    pub fn new() -> UnoDeck {
        // Initialize the deck with standard Uno cards

        // 76 Number Cards
        let mut number_cards = Vec::new();
        for &color in [Color::RED, Color::GREEN, Color::BLUE, Color::YELLOW].iter() {
            for number in 0..10 {
                if let Some(num) = Number::from_u8(number) {
                    number_cards.push(NumberCard {
                        color: color.clone(),
                        number: num,
                    });
                }
            }
            // Add two of each number card except zero
            for number in 1..10 {
                if let Some(num) = Number::from_u8(number) {
                    number_cards.push(NumberCard {
                        color: color.clone(),
                        number: num,
                    });
                }
            }
        }

        // 24 Action Cards
        let mut action_cards = Vec::new();
        for &color in [Color::RED, Color::GREEN, Color::BLUE, Color::YELLOW].iter() {
            for _ in 0..2 {
                action_cards.push(ActionCard {
                    color: color.clone(),
                    action: Action::SKIP,
                });
                action_cards.push(ActionCard {
                    color: color.clone(),
                    action: Action::REVERSE,
                });
                action_cards.push(ActionCard {
                    color: color.clone(),
                    action: Action::DRAWTWO,
                });
            }
        }

        // 8 Wild Cards
        let mut wild_cards = Vec::new();
        for _ in 0..4 {
            wild_cards.push(WildCard {
                color: None,
                wild_type: WildType::WILD,
            });
            wild_cards.push(WildCard {
                color: None,
                wild_type: WildType::DRAWFOUR,
            });
        }

        let mut cards = Vec::new();
        // Combine all cards into the main deck
        for card in number_cards {
            cards.push(UnoCard::NumberCard(card.color, card.number));
        }
        for card in action_cards {
            cards.push(UnoCard::ActionCard(card.color, card.action));
        }
        for card in wild_cards {
            cards.push(UnoCard::WildCard(card.color, card.wild_type));
        }

        UnoDeck { cards }
    }

    pub fn shuffle(&mut self) {
        use rand::seq::SliceRandom;
        use rand::thread_rng;

        let mut rng = thread_rng();

        // Shuffle number cards
        self.cards.shuffle(&mut rng);
    }
}
