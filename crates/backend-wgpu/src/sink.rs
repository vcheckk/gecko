use crossbeam_channel::{Receiver, Sender, bounded};
use gecko::host::{GxAction, RenderSink};

/// Capacity of the action channel. Must be large enough to hold at least
/// two full frames of actions without dropping. Complex scenes (IPL cube
/// menu) can emit 4000+ draw calls per frame.
const CHANNEL_CAPACITY: usize = 65536;

pub struct Renderer {
    tx: Sender<GxAction>,
}

pub struct ActionReceiver {
    rx: Receiver<GxAction>,
}

pub fn channel() -> (Renderer, ActionReceiver) {
    let (tx, rx) = bounded(CHANNEL_CAPACITY);
    (Renderer { tx }, ActionReceiver { rx })
}

impl RenderSink for Renderer {
    fn exec(&mut self, action: GxAction) {
        // If the channel is full we drop the action rather than blocking the
        // CPU thread.  In practice, the main thread should drain fast enough.
        let _ = self.tx.try_send(action);
    }
}

impl ActionReceiver {
    /// Drain all pending actions into the provided buffer.
    pub fn drain(&self, buf: &mut Vec<GxAction>) {
        while let Ok(action) = self.rx.try_recv() {
            buf.push(action);
        }
    }

    /// Returns `true` when the emulator-side sender has been dropped.
    pub fn is_disconnected(&self) -> bool {
        // try_recv returning Disconnected means all senders are gone.
        matches!(self.rx.try_recv(), Err(crossbeam_channel::TryRecvError::Disconnected))
    }
}
