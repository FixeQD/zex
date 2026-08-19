//! Fan-out channel: one event stream distributed to any number of subscribers

use std::sync::Mutex;

pub struct Fan<E> {
    subscribers: Mutex<Vec<flume::Sender<E>>>,
}

impl<E> Fan<E> {
    pub fn new() -> Self {
        Self {
            subscribers: Mutex::new(Vec::new()),
        }
    }
}

impl<E> Default for Fan<E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<E: Clone> Fan<E> {
    pub fn subscribe(&self) -> flume::Receiver<E> {
        let (sender, receiver) = flume::unbounded();
        self.subscribers.lock().unwrap().push(sender);
        receiver
    }

    /// Deliver `event` to every live subscriber, pruning dead ones
    pub fn push(&self, event: &E) {
        let mut subscribers = self.subscribers.lock().unwrap();
        subscribers.retain(|sender| sender.send(event.clone()).is_ok());
    }
}