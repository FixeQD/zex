//! The PipeWire mainloop glue for the volume monitor

use crate::audio::pod::{build_volume_pod, parse_volume_pod};
use crate::audio::volume::{VolumeCommand, VolumeState};
use anyhow::Result;
use pipewire as pw;
use pw::spa::param::ParamType;
use pw::spa::pod::Pod;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use tokio::sync::oneshot;
use tracing::info;

struct Proxies {
    _proxies: Vec<Box<dyn pw::proxy::ProxyT>>,
    _listeners: Vec<Box<dyn pw::proxy::Listener>>,
}

struct MonitorState {
    node: Option<pw::node::Node>,
    _proxies: Proxies,
    ready_tx: Option<oneshot::Sender<()>>,
}

pub fn run_volume_monitor(
    state: Arc<Mutex<VolumeState>>,
    cmd_rx: pw::channel::Receiver<VolumeCommand>,
    ready_tx: oneshot::Sender<()>,
) -> Result<(), pw::Error> {
    pw::init();
    let mainloop = pw::main_loop::MainLoopRc::new(None)?;
    let context = pw::context::ContextRc::new(&mainloop, None)?;
    let core = context.connect_rc(None)?;
    let registry = core.get_registry_rc()?;

    let inner = Rc::new(RefCell::new(MonitorState {
        node: None,
        _proxies: Proxies {
            _proxies: Vec::new(),
            _listeners: Vec::new(),
        },
        ready_tx: Some(ready_tx),
    }));

    let _reg_listener = registry
        .add_listener_local()
        .global({
            let inner = Rc::clone(&inner);
            let state = Arc::clone(&state);
            move |obj| {
                let is_sink = obj
                    .props
                    .as_ref()
                    .and_then(|props| props.get("media.class"))
                    .map(|class| class == "Audio/Sink")
                    .unwrap_or(false);
                if obj.type_ != pw::types::ObjectType::Node || !is_sink {
                    return;
                }
                if inner.borrow().node.is_some() {
                    return;
                }

                if let Ok(registry) = core.get_registry_rc()
                    && let Ok(node) = registry.bind::<pw::node::Node, _>(obj)
                {
                    node.subscribe_params(&[ParamType::Props]);
                    let state = Arc::clone(&state);
                    let ready_tx = Rc::new(RefCell::new(inner.borrow_mut().ready_tx.take()));
                    let param_listener = node
                        .add_listener_local()
                        .param(move |_seq, _id, _index, _next, param| {
                            if let Some(pod) = param
                                && let Some((volume, muted)) = parse_volume_pod(pod)
                            {
                                let mut state = state.lock().unwrap();
                                state.volume = volume;
                                state.muted = muted;
                            }
                            if let Some(tx) = ready_tx.borrow_mut().take() {
                                let _ = tx.send(());
                            }
                        })
                        .register();

                    let listener = Box::new(param_listener) as Box<dyn pw::proxy::Listener>;
                    if let Ok(mut inner) = inner.try_borrow_mut() {
                        inner._proxies._listeners.push(listener);
                    }
                    inner.borrow_mut().node = Some(node);
                }
            }
        })
        .register();

    let node = Rc::clone(&inner);
    let state = Arc::clone(&state);

    let _attached = cmd_rx.attach(mainloop.loop_(), move |cmd| {
        let Ok(inner) = node.try_borrow() else {
            return;
        };
        let Some(ref node) = inner.node else {
            return;
        };
        match cmd {
            VolumeCommand::SetVolume(volume) => {
                let muted = state.lock().unwrap_or_else(|e| e.into_inner()).muted;
                let bytes = build_volume_pod(volume, muted);
                if let Some(pod) = Pod::from_bytes(&bytes) {
                    node.set_param(ParamType::Props, 0, pod);
                }
            }
            VolumeCommand::ToggleMute => {
                let state = state.lock().unwrap_or_else(|e| e.into_inner());
                let bytes = build_volume_pod(state.volume, !state.muted);
                if let Some(pod) = Pod::from_bytes(&bytes) {
                    node.set_param(ParamType::Props, 0, pod);
                }
            }
        }
    });

    info!("audio: volume monitor started (PipeWire registry)");

    mainloop.run();

    Ok(())
}
