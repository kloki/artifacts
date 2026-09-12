use std::{
    collections::HashMap,
    sync::{Arc, Mutex as StdMutex},
};

use tokio::sync::broadcast;
use uuid::Uuid;

/// A lightweight event kinds published when an artifact's content changes.
#[derive(Clone, Copy, Debug)]
pub enum EventKind {
    /// A new version was published. Carries the new version number.
    Update(u32),
    /// The artifact was deleted.
    Deleted,
}

/// Per-artifact broadcast bus used by the live-update SSE endpoint.
#[derive(Clone, Default, Debug)]
pub struct EventBus {
    senders: Arc<StdMutex<HashMap<Uuid, broadcast::Sender<EventKind>>>>,
}

impl EventBus {
    pub fn new() -> Self {
        Self::default()
    }

    /// Publish an event to every current subscriber. If no one is listening the
    /// event is silently dropped.
    pub fn publish(&self, id: Uuid, kind: EventKind) {
        let senders = self.senders.lock().expect("event bus lock poisoned");
        if let Some(tx) = senders.get(&id) {
            let _ = tx.send(kind);
        }
    }

    /// Subscribe to future events for an artifact, creating the channel lazily.
    pub fn subscribe(&self, id: Uuid) -> broadcast::Receiver<EventKind> {
        let mut senders = self.senders.lock().expect("event bus lock poisoned");
        senders
            .entry(id)
            .or_insert_with(|| broadcast::channel(8).0)
            .subscribe()
    }

    /// Drop the channel for an artifact. Called after deletion to clean up.
    pub fn remove(&self, id: Uuid) {
        let mut senders = self.senders.lock().expect("event bus lock poisoned");
        senders.remove(&id);
    }
}
