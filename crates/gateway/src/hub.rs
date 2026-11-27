//! Delivers notifications to connected devices and disconnects revoked ones. One live connection per
//! device: a new connection replaces the old one.

use std::{collections::HashMap, sync::Mutex};

use library_core::DeviceId;
use tokio::sync::mpsc;

/// What the gateway sends down a socket besides responses.
#[derive(Debug, Clone, PartialEq)]
pub enum Push {
    /// A notification such as `sync.delta` or `scope.revoked`.
    Notify {
        name: String,
        payload: serde_json::Value,
    },
    /// Close the connection, for example after the device was revoked.
    Disconnect,
}

struct Entry {
    generation: u64,
    tx: mpsc::Sender<Push>,
}

#[derive(Default)]
pub struct Hub {
    next: Mutex<u64>,
    devices: Mutex<HashMap<DeviceId, Entry>>,
}

impl Hub {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a device. Returns its inbox and a generation number to unregister with.
    /// Any earlier connection of the same device is told to leave.
    pub fn register(&self, device: DeviceId) -> (mpsc::Receiver<Push>, u64) {
        let (tx, rx) = mpsc::channel(256);
        let generation = self.next.lock().map_or(0, |mut n| {
            *n += 1;
            *n
        });
        if let Ok(mut m) = self.devices.lock()
            && let Some(old) = m.insert(device, Entry { generation, tx })
        {
            let _ = old.tx.try_send(Push::Disconnect);
        }
        (rx, generation)
    }

    /// Removes the registration only if it is still the caller's. A newer connection that replaced it stays.
    pub fn unregister(&self, device: DeviceId, generation: u64) {
        if let Ok(mut m) = self.devices.lock()
            && m.get(&device).is_some_and(|e| e.generation == generation)
        {
            m.remove(&device);
        }
    }

    fn sender(&self, device: DeviceId) -> Option<mpsc::Sender<Push>> {
        self.devices
            .lock()
            .ok()
            .and_then(|m| m.get(&device).map(|e| e.tx.clone()))
    }

    /// Returns false when the device is not connected or its inbox is full (the client will resync).
    pub fn notify(&self, device: DeviceId, name: &str, payload: serde_json::Value) -> bool {
        self.sender(device).is_some_and(|tx| {
            tx.try_send(Push::Notify {
                name: name.to_string(),
                payload,
            })
            .is_ok()
        })
    }

    pub fn disconnect(&self, device: DeviceId) -> bool {
        self.sender(device)
            .is_some_and(|tx| tx.try_send(Push::Disconnect).is_ok())
    }

    pub fn connected(&self) -> usize {
        self.devices.lock().map_or(0, |m| m.len())
    }
}
