use crate::game::cards::*;
use crate::game::player::Player;
use crate::game::events::GameEvent as GE;

pub struct UnoGame {
    deck: UnoDeck,
    players: Vec<Player>,
    pub current_player: usize,
    pub top_card: Option<UnoCard>,
    pub direction: bool, // true for clockwise, false for counter-clockwise
    pub started: bool,
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
            started: false,
        }
    }

    pub fn get_player_hand(&self, player_id: usize) -> Vec<UnoCard> {
        if let Some(player) = self.players.get(player_id) {
            player.display_hand().clone()
        } else {
            Vec::new()
        }
    }

    pub fn get_players_cards_count(&self) -> Vec<(String, usize)> {
        self.players.iter()
            .map(|p| (p.name.clone(), p.display_hand().len()))
            .collect()
    }

    pub fn init_game(&mut self, players: Vec<String>) -> Vec<GE>{
        let mut ev = Vec::new();
        if self.started {
            ev.push(GE::GameError { message: "Game already started!".to_string() });
            return ev;
        }
        self.add_players(players, &mut ev);
        // Distribute initial cards to players
        for i in 0..self.players.len() {
            ev.extend(self.cards_distribution(i, 7));
        }
        // Draw the first card from the deck to start the game
        loop {
            if let Some(card) = self.deck.cards.pop() {
                if !card.get_number().is_none(){
                    self.top_card = Some(card);
                    ev.push(GE::TopCardChanged { top_card: self.top_card.
                        expect("Top card should be set").clone() });
                    ev.push(GE::PlayerTurn { player_id: self.current_player });
                    break;
                }
                self.deck.cards.push(card); // 如果抽到的不是数字牌，放回去继续抽
                self.deck.shuffle(); // 重新洗牌
            } else {
                ev.push(GE::GameError { message: 
                    "No more cards in the deck to start the game!".to_string() });
                break;
            }
        }
        self.started = true;
        ev.push(GE::GameStarted { game_id: 0 }); // 这里可以设置一个实际的游戏ID
        ev
    }

    fn add_players(&mut self, players: Vec<String>, ev: &mut Vec<GE>) {
        for (index, name) in players.into_iter().enumerate() {
            let player = Player::new(&name, index);
            self.players.push(player);
            ev.push(GE::PlayerJoined {
                player_id: index,
                name: name.clone(),
            });
        }
    }

    fn change_direction(&mut self) {
        self.direction = !self.direction;
    }

    fn cards_distribution(&mut self, player_index: usize, num_cards: usize) -> Vec<GE>{
        let mut ev = Vec::new();
        for _ in 0..num_cards {
            if let Some(card) = self.deck.cards.pop() {
                self.players[player_index].push_card(card);
                ev.push(GE::CardDraw { 
                    player_id: player_index, card });
            }
            else {
                ev.push(GE::GameError { message: 
                    "No more cards in the deck to draw!".to_string() });
                break;
            }
        }
        ev
    }

    fn no_card_to_play(&mut self) -> Result<UnoCard, String> {
        // 如果没有牌可以打，抽一张牌
        if let Some(card) = self.deck.cards.pop() {
            // println!("{} draws a card.", self.players[self.current_player].name);
            return Ok(card);
        } else {
            // eprintln!("No more cards in the deck to draw!");
            return Err("No more cards in the deck to draw!".to_string());
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

    pub fn play_card(
        &mut self, 
        player_id: usize,
        card_idx: usize,
        call_uno: bool,
        color: Color
    ) -> Vec<GE> {
        let mut ev = Vec::new();

        // 校验玩家, 已在服务端校验
        // if self.players.get(self.current_player).map(|p| p.id) != Some(player_id) {
        //     ev.push(GE::GameError { message: format!("It's not Player {}'s turn!", player_id).to_string() });
        //     return ev;
        // }

        let hand = self.players[self.current_player].display_hand();
        if card_idx >= hand.len() {
            ev.push(GE::GameError { message: 
                "Invalid card index".to_string() });
            return ev;
        }
        let card = hand[card_idx].clone();
        let card = match card {
            UnoCard::WildCard(_, wt) => UnoCard::WildCard(Some(color), wt),
            _ => card,
        };
        if !valid_card(&card, &self.top_card) {
            ev.push(GE::GameError { message: 
                "Cannot play this card".to_string() });
            return ev;
        }
        
        // 出牌
        let _ = self.players[self.current_player].remove_card(card_idx).unwrap();
        self.top_card = Some(card.clone());
        ev.push(GE::CardPlayed { 
            player_id: player_id, card: card.clone() });
        ev.push(GE::TopCardChanged { top_card: self.top_card.
            expect("Top card should be set").clone() });


        // 牌生效
        match card {
            UnoCard::ActionCard(_, act ) => {
                match act {
                    Action::SKIP => {
                        let skipped_player = self.next_player();
                        ev.push(GE::PlayerSkipped { player_id: 
                            self.players[skipped_player].id });
                        self.current_player = self.next_player();
                    },

                    Action::REVERSE => {
                        self.change_direction();
                        ev.push(GE::DirectionChanged { clockwise: self.direction });
                    }, 

                    Action::DRAWTWO => {
                        let affected_player = self.next_player();
                        self.current_player = self.next_player();
                        ev.push(GE::DrawTwoApplied { 
                            target_player_id: self.players[affected_player].id });
                        ev.extend( self.cards_distribution(affected_player, 2) );
                    }
                }
            }

            UnoCard::WildCard(_, wt) => {
                match wt {
                    WildType::DRAWFOUR => {
                        let affected_player = self.next_player();
                        ev.push(GE::DrawFourApplied { 
                            target_player_id: self.players[affected_player].id });
                        ev.extend( self.cards_distribution(affected_player, 4) );
                        self.current_player = self.next_player();
                    }
                    _ => { }
                }
            }

            _ => { }
        }
        self.current_player = self.next_player();

        // 检查是否有玩家获胜,并切换到下一个玩家
        if self.players[player_id].display_hand().is_empty() {
            ev.push(GE::GameOver { winner: player_id, scores: self.calculate_scores() });
            self.started = false; // 游戏结束，重置状态
        }
        
        // 检查玩家是否需要叫UNO, 并进行惩罚
        if call_uno ^ (self.players[player_id].display_hand().len() == 1) {
            ev.push(GE::UnoPenalty { player_id: player_id });
            ev.extend(self.cards_distribution(player_id, 2));
        } else if call_uno{
            ev.push(GE::UnoCalled { player_id: player_id });
        }

        ev.push(GE::PlayerTurn { player_id: 
            self.players[self.current_player].id });
        
        ev
    }

    pub fn draw_card(&mut self, player_id: usize) -> Vec<GE> {
        let mut ev = Vec::new();
        
        // 校验玩家
        if self.players.get(self.current_player).map(|p| p.id) != Some(player_id) {
            ev.push(GE::GameError { message: "It's not your turn!".to_string() });
            return ev;
        }

        // 抽一张牌
        let drawn_card = self.no_card_to_play();
        match drawn_card {
            Ok(drawn_card) => {
                self.players[self.current_player].push_card(drawn_card.clone());
                ev.push(GE::CardDraw { player_id, card: drawn_card });
                if valid_card(&drawn_card, &self.top_card) {
                    ev.push(GE::DrawnCardPlayable { player_id: player_id });
                }
                else {
                    ev.extend(self.player_pass(player_id));
                }
            }
            Err(e) => {
                ev.push(GE::GameError { message: e });
            }
        }
        ev
    }

    pub fn player_pass(&mut self, player_id: usize) -> Vec<GE> {
        let mut ev = Vec::new();
        ev.push(GE::PlayerPassed { player_id: player_id });
        self.current_player = self.next_player();
        ev.push(GE::PlayerTurn { player_id: 
            self.players[self.current_player].id });

        ev
    }

    fn calculate_scores(&self) -> Vec<(String, i32)> {
        // 游戏结束，计算每个玩家的分数并公布排名
        let mut scores = Vec::new();
        for player in &self.players {
            let score: i32 = player.display_hand().iter().map(|card| card.get_value()).sum();
            scores.push((player.name.clone(), score));
        }
        scores.sort_by(|a, b| a.1.cmp(&b.1)); // 按分数升序排序
        scores

        // 打印美观的分数表
        // println!("\n================ Final Scores ================");
        // println!("{:<20} | {:>10}", "Player", "Score");
        // println!("---------------------------------------------");
        // for (name, score) in scores {
        //     println!("{:<20} | {:>10}", name, score);
        // }
        // println!("=============================================");
    }

    // 挑战逻辑
    // 如果玩家发现对手没有叫“UNO”，可以挑战
    // 如果挑战成功，对手需要抽取两张牌
    // 如果挑战失败，挑战者需要抽取两张牌
    // 这里可以实现更复杂的逻辑
    fn challenge(&mut self, challenger_id: usize, challenged_id: usize) -> Vec<GE> {
        let mut ev = Vec::new();
        let challenge_successful = false;
        if challenge_successful {
            ev.push(GE::ChallengedSuccess { challenger_id, challenged_id });
            ev.extend(self.cards_distribution(challenged_id, 2));
        } else {
            ev.extend(self.cards_distribution(challenger_id, 2));
        }
        ev
    }
}
