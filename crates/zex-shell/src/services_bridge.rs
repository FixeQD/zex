use flume::Receiver;
use zex_core::Settings;

use crate::app::{IpcRequest, ServiceEvent, ServiceHandles};

pub fn spawn_all_services(_settings: Settings) -> (ServiceHandles, Receiver<ServiceEvent>) {
    let (event_tx, event_rx) = flume::unbounded();
    let (recorder_tx, _recorder_rx) = flume::unbounded();

    // For now just bridge nothing; real service wiring will be added in later commits
    let _ = event_tx;

    let handles = ServiceHandles { recorder_tx };
    (handles, event_rx)
}

pub fn spawn_ipc_listener(_rx: Receiver<IpcRequest>) {}
