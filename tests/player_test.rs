use uno::game::*;

#[cfg(test)]
mod player_test {
    use super::*;

    #[test]
    fn test_player_hand() {
        let mut player = Player::new("Alice");
        assert_eq!(player.display_hand().len(), 0);

        let card = UnoCard::NumberCard(Color::RED, Number::from_u8(5).unwrap());
        player.push_card(card.clone());
        assert_eq!(player.display_hand().len(), 1);
        assert_eq!(player.display_hand()[0], card);
    }
    #[test]
    fn test_player_play_card() {
        let mut player = Player::new("Alice");
        let card = UnoCard::NumberCard(Color::RED, Number::from_u8(5).unwrap());
        player.push_card(card.clone());

        let top_card = UnoCard::NumberCard(Color::RED, Number::from_u8(3).unwrap());
        let played_card = player.can_play_card(0, Some(&top_card)).unwrap();
        assert_eq!(played_card, card); // 牌能打出去
        assert_eq!(player.display_hand().len(), 0);
    }

    #[test]
    #[ignore = "deprecated test"]
    fn play_wild_card_with_color() {
        let mut player = Player::new("Alice");
        let wild_card = WildCard {
            color: None,
            wild_type: WildType::WILD,
        };
        // Player::select_color(&mut wild_card, Color::GREEN);
        let card = UnoCard::WildCard(wild_card.color, wild_card.wild_type);
        player.push_card(card);

        let top_card = UnoCard::NumberCard(Color::RED, Number::from_u8(3).unwrap());
        let played_card = player.can_play_card(0, Some(&top_card)).unwrap();
        assert_eq!(played_card, card); // 万能卡能打出去
        assert_eq!(player.display_hand().len(), 0);
    }
}
