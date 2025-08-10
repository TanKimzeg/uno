use uno::game::UnoGame;
use uno::ports::bus::{EventBus, ConsolerLogger};

#[cfg(test)]
mod bus_test {
    use super::*;
    #[test]
    fn test_log_bus() {
        let mut bus = EventBus::new();
        bus.register_handler(Box::new(ConsolerLogger));

        let mut game = UnoGame::new();
        let events = game.init_game(vec!["Alice".into(), "Bob".into()]);
        bus.publish(events);

    // 后续每次调用 play_card / draw_card 后，同样把返回的 Vec<GameEvent> 发布出去

    }
}
