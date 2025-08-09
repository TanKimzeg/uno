use uno::game::*;

#[cfg(test)]
mod game_test {
    use super::*;

    #[test]
    #[ignore = "This test is for manual play and should not run automatically"]
    fn test_play() {
        let players = vec!["Alice", "Bob", "Charlie"];
        let mut uno_game: UnoGame = UnoGame::new();
        uno_game.start(players);
    }
}
