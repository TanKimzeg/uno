use crate::game::cards::*;
use crate::game::player::Player;

pub struct UnoGame {
    deck: UnoDeck,
    players: Vec<Player>,
    current_player: usize,
    top_card: Option<UnoCard>,
    direction: bool, // true for clockwise, false for counter-clockwise
}

impl UnoGame {
    pub fn new() -> UnoGame{
        let mut d = UnoDeck::new();
        d.shuffle();
        UnoGame {
            deck: d,
            players: Vec::new(),
            top_card: None,
            direction: true,
            current_player: 0,
        }
    }

    pub fn init_game(&mut self, players: Vec<&str>) {
        self.add_players(players);
        // Distribute initial cards to players
        for i in 0..self.players.len() {
            self.cards_distribution(i, 7);
        }
        // Draw the first card from the deck to start the game
        loop {
            if let Some(card) = self.deck.cards.pop() {
                if !card.get_number().is_none(){
                    self.top_card = Some(card);
                    break;
                }
                self.deck.cards.push(card); // 如果抽到的不是数字牌，放回去继续抽
                self.deck.shuffle(); // 重新洗牌
            } else {
                eprintln!("No cards left in the deck to start the game!");
                panic!("No cards left in the deck to start the game!");
            }
        }
    }

    fn add_players(&mut self, players: Vec<&str>) {
        for name in players {
            self.players.push(Player::new(name));
        }
    }

    fn change_direction(&mut self) {
        self.direction = !self.direction;
    }

    fn cards_distribution(&mut self, player_index: usize, num_cards: usize) {
        for _ in 0..num_cards {
            if let Some(card) = self.deck.cards.pop() {
                self.players[player_index].push_card(card);
            }
            else {
                // eprintln!("No more cards in the deck to distribute!");
                panic!("No more cards in the deck to distribute!");
            }
        }
    }

    fn no_card_to_play(&mut self) -> UnoCard {
        // 如果没有牌可以打，抽一张牌
        if let Some(card) = self.deck.cards.pop() {
            println!("{} draws a card.", self.players[self.current_player].name);
            return card;
        } else {
            // eprintln!("No more cards in the deck to draw!");
            panic!("No more cards in the deck to draw!");
        }
    }

    fn next_player(&self) -> usize {
        if self.direction {
            (self.current_player + 1) % self.players.len()
        } else {
            (self.current_player + self.players.len() - 1) % self.players.len()
        }
    }

    fn previous_player(&self) -> usize {
        if self.direction {
            (self.current_player + self.players.len() - 1) % self.players.len()
        } else {
            (self.current_player + 1) % self.players.len()
        }
    }

    // 有人获胜:true, 否则false
    fn play_card(&mut self, card: &UnoCard, is_uno: bool) -> bool {
        let current_player = &self.players[self.current_player];
        match card {
            UnoCard::WildCard(_, wild_type) => {
                // 如果是万能卡，要求玩家选择颜色
                println!(
                    "{} played a wild card, please select a color.",
                    current_player.name
                );
                let card = current_player.select_color(&card);
                self.top_card = Some(card);
                match wild_type {
                    WildType::WILD => {
                    },
                    WildType::DRAWFOUR => {
                        // 如果是抽四张牌的万能卡，要求下家抽四张牌
                        self.cards_distribution(self.next_player(), 4);
                        println!("{} draws 4 cards.", self.players[self.next_player()].name);
                    }
                }
            }
            UnoCard::ActionCard(_, action) => {
                match action {
                    Action::SKIP => {
                        println!(
                            "{} skips their turn.",
                            self.players[self.next_player()].name
                        );
                    }
                    Action::DRAWTWO => {
                        self.cards_distribution(self.next_player(), 2);
                        println!("{} draws 2 cards.", self.players[self.next_player()].name);
                    }
                    Action::REVERSE => {
                        self.change_direction();
                        println!("Direction changed!");
                    }
                }
                self.top_card = Some(*card);
            }
            UnoCard::NumberCard(_, _) => {
                self.top_card = Some(*card);
            }
        }
        println!(
            "{} played: {:}",
            self.players[self.current_player].name,
            &self.top_card.unwrap().to_string()
        );

        // 检查是否有玩家获胜,并切换到下一个玩家
        if self.players[self.current_player].display_hand().is_empty() {
            println!("{} wins!", self.players[self.current_player].name);
            return true;
        }
        
        // 检查玩家是否需要叫UNO, 并进行惩罚
        if self.players[self.current_player].display_hand().len() == 1 && !is_uno {
            println!("{} did not call UNO! Drawing 2 penalty cards.", self.players[self.current_player].name);
            self.cards_distribution(self.current_player, 2);
        }
        match card {
            UnoCard::ActionCard(_, action) => {
                if *action == Action::SKIP {
                    // 下家不能出牌
                    self.current_player = self.next_player();
                    self.current_player = self.next_player();
                }
                else {
                    self.current_player = self.next_player();
                }
            }
            UnoCard::WildCard(_, wild_type) => {
                // 下家不能出牌
                if *wild_type == WildType::DRAWFOUR {
                    self.current_player = self.next_player();
                    self.current_player = self.next_player();
                }
                else {
                    self.current_player = self.next_player();
                }
            }
            _ => {
                // 下家出牌
                self.current_player = self.next_player();
            }
        }
        false
    }

    pub fn start(&mut self, player_list: Vec<&str>) {
        // Uno 游戏主体逻辑

        // 1. 初始化游戏
        self.init_game(player_list);

        // 2. 玩家轮流出牌
        loop {
            let top_card = self.top_card.as_ref();
            // 展示当前牌堆上的牌
            if let Some(card) = top_card {
                println!("Top card: {}", card.to_string());
            } else {
                println!("No cards on the table yet.");
            }
            let current_player = &mut self.players[self.current_player];
            // 玩家出牌逻辑
            // 获取玩家想打的牌
            let (card_to_play, is_uno) = current_player.want_to_play();
            match card_to_play {
                None => {
                    // 如果无法出牌，则抽卡
                    println!(
                        "{} cannot start a card, drawing a card.",
                        current_player.name
                    );
                    let drawn_card = self.no_card_to_play();
                    if valid_card(&drawn_card, self.top_card.as_ref()) {
                        println!("valid card to start: {:}", drawn_card.to_string());
                        println!("Do you want to start this card? (yes/no)");
                        let mut response = String::new();
                        loop {
                            match std::io::stdin().read_line(&mut response) {
                                Ok(_) => {
                                    let response: Vec<&str> = response.trim().split_whitespace().collect();
                                    let is_uno = match response.len() {
                                        2 => {
                                            response[1].eq_ignore_ascii_case("uno")
                                        },
                                        _ => false,
                                    };
                                    if response[0].eq_ignore_ascii_case("yes") {
                                        self.play_card(&drawn_card, is_uno);
                                        break;
                                    } else if response[0].eq_ignore_ascii_case("no"){
                                        println!("You chose not to start the drawn card.");
                                        self.players[self.current_player].push_card(drawn_card);
                                        self.current_player = self.next_player();
                                        break;
                                    }
                                }
                                Err(_) => {
                                    println!("Error reading input, please try again.");
                                    continue;
                                }
                            }
                        }
                    } else {
                        println!("The drawn card is not valid to start, adding it to hand.");
                        self.players[self.current_player].push_card(drawn_card);
                        self.current_player = self.next_player();
                    }
                    continue;
                }
                Some(card) => {
                    // 如果玩家选择了一张牌,检查是否合法
                    if let Some(card) = current_player.can_play_card(card, top_card).ok() {
                        if self.play_card(&card, is_uno) {
                            break;
                        }
                    } else {
                        println!("The selected card is not valid for the top card, please choose a valid card.");
                        continue;
                    }
                }
            }
        }
        // 3. 游戏结束,计分
        self.calculate_scores();
        println!("Game over!");
    }

    fn calculate_scores(&self) {
        // 游戏结束，计算每个玩家的分数并公布排名
        let mut scores = Vec::new();
        for player in &self.players {
            let score: i32 = player.display_hand().iter().map(|card| card.get_value()).sum();
            scores.push((player.name.clone(), score));
        }
        scores.sort_by(|a, b| a.1.cmp(&b.1)); // 按分数升序排序

        // 打印美观的分数表
        println!("\n================ Final Scores ================");
        println!("{:<20} | {:>10}", "Player", "Score");
        println!("---------------------------------------------");
        for (name, score) in scores {
            println!("{:<20} | {:>10}", name, score);
        }
        println!("=============================================");
    }

    fn challenge(&mut self) {
        // 挑战逻辑
        // 如果玩家认为对手打出的牌不合法，可以挑战
        // 如果挑战成功，对手需要抽取两张牌
        // 如果挑战失败，挑战者需要抽取两张牌
        // 这里可以实现更复杂的逻辑
        let challenge_successful = false;
        if challenge_successful {
            self.cards_distribution(self.previous_player(), 2);
            println!("Challenge successful! Opponent draws 2 cards.");
        } else {
            self.cards_distribution(self.previous_player(), 2);
            println!("Challenge failed! You draw 2 cards.");
        }
    }
}
