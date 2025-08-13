use uno::game::*;

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

        assert!(valid_card(&card1, &Some(top_card)));
        assert!(valid_card(&card2, &Some(top_card)));
        assert!(valid_card(&card3, &Some(top_card)));
        assert!(valid_card(&card1, &None));
    }
}
