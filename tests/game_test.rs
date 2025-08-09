use uno::game::*;

mod game_test {
    use super::*;

    #[test]
    fn test_game_initialization() {
        let players = vec!["Alice", "Bob", "Charlie"];
        let mut uno_game: UnoGame = UnoGame::new();
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
