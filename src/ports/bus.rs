use crate::game::events::GameEvent;

pub trait EventHandler: Send + Sync {
    fn handle_event(&self, event: &GameEvent);
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

    pub fn publish_event(&self, event: &GameEvent) {
        for handler in &self.handlers {
            handler.handle_event(event);
        }
    }

    pub fn publish<I>(&self, events: I)
    where
        I: IntoIterator<Item = GameEvent>,
    {
        for event in events {
            self.publish_event(&event);
        }
    }

    // pub fn clear_handlers(&mut self) {
    //     self.handlers.clear();
    // }
}

pub struct ConsolerLogger;

impl EventHandler for ConsolerLogger {
    fn handle_event(&self, event: &GameEvent) {
        println!("[ConsoleLogger] Event: {:?}", event);
    }
}
