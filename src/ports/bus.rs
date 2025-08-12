use crate::game::events::GameEvent;

pub trait EventHandler: Send + Sync {
    fn handle_events(&self, events: &[GameEvent]);
}

pub struct EventBus {
    handlers: Vec<Box<dyn EventHandler>>,
}

impl EventBus {
    pub fn new() -> Self {
        EventBus {
            handlers: Vec::new(),
        }
    }

    pub fn register_handler(&mut self, h: Box<dyn EventHandler>) {
        self.handlers.push(h);
    }

    pub fn publish_events(&self, events: &[GameEvent]) {
        for handler in &self.handlers {
            handler.handle_events(events);
        }
    }

    pub fn publish<I>(&self, events: I)
    where
        I: IntoIterator<Item = GameEvent>,
    {

        let events: Vec<GameEvent> = events.into_iter().collect();
        for h in &self.handlers {
            h.handle_events(&events);
        }
    }

    // pub fn clear_handlers(&mut self) {
    //     self.handlers.clear();
    // }
}

pub struct ConsolerLogger;

impl EventHandler for ConsolerLogger {
    fn handle_events(&self, events: &[GameEvent]) {
        for event in events {
            eprintln!("[ConsoleLogger] Event: {}", event);
        }
    }
}
