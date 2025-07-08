use uno::cards::*;
use uno::game::*;
use uno::player::*;

#[cfg(test)]
mod card_tests {
    use super::*;

    #[test]
    fn test_number_card() {
        let card: NumberCard = NumberCard {
            color: Color::RED,
            number: Number::from_u8(5).unwrap(),
        };
        assert_eq!(card.color, Color::RED);
        assert_eq!(card.number, Number::FIVE);
    }

    #[test]
    fn test_valid_card() {
        let top_card = UnoCard::NumberCard(Color::RED, Number::from_u8(5).unwrap());
        let card1 = UnoCard::NumberCard(Color::RED, Number::from_u8(3).unwrap());
        let card2 = UnoCard::ActionCard(Color::RED, Action::SKIP);
        let card3 = UnoCard::WildCard(Some(Color::BLUE), WildType::WILD);

        assert!(valid_card(&card1, Some(&top_card)));
        assert!(valid_card(&card2, Some(&top_card)));
        assert!(valid_card(&card3, Some(&top_card)));
        assert!(valid_card(&card1, None));
    }

}

mod game_test {
    use super::*;

    #[test]
    fn test_game_initialization() {
        let players = vec!["Alice", "Bob", "Charlie"];
        let mut uno_game:UnoGame = UnoGame::new();
        uno_game.init_game(players);
        assert_eq!(uno_game.players.len(), 3);
        assert_eq!(uno_game.players[0].hand.len(), 7);
    }

    #[test]
    #[ignore = "This test is for manual play and should not run automatically"]
    fn test_play() {
        let players = vec!["Alice", "Bob", "Charlie"];
        let mut uno_game: UnoGame = UnoGame::new();
        uno_game.play(players);
    }
}

mod player_test {
    use super::*;

    #[test]
    fn test_player_hand() {
        let mut player = Player::new("Alice");
        assert_eq!(player.display_hand().len(), 0);
        
        let card = UnoCard::NumberCard(Color::RED, Number::from_u8(5).unwrap());
        player.hand.push(card.clone());
        assert_eq!(player.display_hand().len(), 1);
        assert_eq!(player.display_hand()[0], card);
    }

    #[test]
    fn test_player_play_card() {
        let mut player = Player::new("Alice");
        let card = UnoCard::NumberCard(Color::RED, Number::from_u8(5).unwrap());
        player.hand.push(card.clone());

        let top_card = UnoCard::NumberCard(Color::RED, Number::from_u8(3).unwrap());
        let played_card = player.can_play_card(0, Some(&top_card)).unwrap();
        assert_eq!(played_card, card); // 牌能打出去
        assert_eq!(player.display_hand().len(), 0);
    }

    #[test]
    #[should_panic]
    fn play_wild_card_without_color() {
        let mut player = Player::new("Alice");
        let wild_card = UnoCard::WildCard(None, WildType::WILD);
        player.hand.push(wild_card.clone());

        // 尝试打出没有颜色的万能卡
        player.can_play_card(0, None).unwrap();
    }

    #[test]
    #[ignore = "deprecated test"]
    fn play_wild_card_with_color() {
        let mut player = Player::new("Alice");
        let mut wild_card = WildCard { color: None, wild_type: WildType::WILD };
        // Player::select_color(&mut wild_card, Color::GREEN);
        let card = UnoCard::WildCard(wild_card.color, wild_card.wild_type);
        player.hand.push(card);

        let top_card = UnoCard::NumberCard(Color::RED, Number::from_u8(3).unwrap());
        let played_card = player.can_play_card(0, Some(&top_card)).unwrap();
        assert_eq!(played_card, card); // 万能卡能打出去
        assert_eq!(player.display_hand().len(), 0);
    }
}