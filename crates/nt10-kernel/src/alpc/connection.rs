//! `NtAlpcConnectPort` — client/server connection state (stub).

use super::port::AlpcPortId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlpcConnectionState {
    Disconnected,
    Listening,
    Connected,
}

#[derive(Debug)]
pub struct AlpcConnection {
    pub port: AlpcPortId,
    pub state: AlpcConnectionState,
}

impl AlpcConnection {
    #[must_use]
    pub fn new(port: AlpcPortId) -> Self {
        Self {
            port,
            state: AlpcConnectionState::Disconnected,
        }
    }

    pub fn connect(&mut self) -> Result<(), ()> {
        if self.state != AlpcConnectionState::Disconnected {
            return Err(());
        }
        self.state = AlpcConnectionState::Connected;
        Ok(())
    }

    pub fn listen(&mut self) -> Result<(), ()> {
        if self.state != AlpcConnectionState::Disconnected {
            return Err(());
        }
        self.state = AlpcConnectionState::Listening;
        Ok(())
    }

    /// Server side: move from listening to connected when a client attaches (bring-up: single step).
    pub fn accept(&mut self) -> Result<(), ()> {
        if self.state != AlpcConnectionState::Listening {
            return Err(());
        }
        self.state = AlpcConnectionState::Connected;
        Ok(())
    }

    pub fn disconnect(&mut self) -> Result<(), ()> {
        match self.state {
            AlpcConnectionState::Connected | AlpcConnectionState::Listening => {
                self.state = AlpcConnectionState::Disconnected;
                Ok(())
            }
            AlpcConnectionState::Disconnected => Err(()),
        }
    }
}
