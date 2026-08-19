use std::sync::Arc;

use flume::Receiver;
use tokio::task::JoinHandle;

use super::super::client::facade::ClientCommand;
use super::super::engine::core::Core;

pub fn spawn(rx: Receiver<ClientCommand>, core: Arc<Core>) -> JoinHandle<()> {
    tokio::spawn(async move {
        while let Ok(command) = rx.recv_async().await {
            match command {
                ClientCommand::Close(id) => {
                    let _ = core.close(id).await;
                }
                ClientCommand::CloseAll => {
                    let _ = core.close_all().await;
                }
                ClientCommand::InvokeAction(id, key) => {
                    let _ = core.invoke_action(id, &key).await;
                }
                ClientCommand::Snapshot(reply) => {
                    let _ = reply.send(core.notifications());
                }
                ClientCommand::Dnd(reply) => {
                    let _ = reply.send(core.dnd());
                }
                ClientCommand::SetDnd(dnd) => core.set_dnd(dnd),
                ClientCommand::ApplyConfig(config) => core.apply_config(config),
            }
        }
    })
}