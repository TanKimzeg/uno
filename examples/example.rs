fn main() {
    // Uno 游戏的入口点
    use uno::game::UnoGame;

    // 初始化游戏
    let players = vec!["Alice", "Bob", "Charlie"];
    let mut uno_game = UnoGame::new();
    
    // 开始游戏
    uno_game.play(players);
}