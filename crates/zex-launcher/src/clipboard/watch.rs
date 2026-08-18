use crate::clipboard::Content;
use crate::clipboard::content::Entry;
use crate::clipboard::history::Settings;
use anyhow::{Context, Result};
use tracing::{debug, warn};
use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols_wlr::data_control::v1::client::zwlr_data_control_manager_v1;

pub struct Watcher {
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    join: Option<std::thread::JoinHandle<()>>,
}

impl Watcher {
    /// Returns `None` when the session has no `wlr-data-control` global or no seat to attach to
    #[must_use]
    pub fn spawn(settings: Settings, out: flume::Sender<Entry>) -> Option<Self> {
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let watch = stop.clone();
        let join = std::thread::Builder::new()
            .name("zex-clipboard-watcher".into())
            .spawn(move || {
                if let Err(e) = run(watch, settings, out) {
                    warn!("clipboard watcher stopped: {e}");
                }
            })
            .ok()?;
        Some(Self {
            stop,
            join: Some(join),
        })
    }

    pub fn stop(&mut self) {
        self.stop.store(true, std::sync::atomic::Ordering::Relaxed);
        if let Some(handle) = self.join.take() {
            let _ = handle.join();
        }
    }
}

fn run(
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    settings: Settings,
    out: flume::Sender<Entry>,
) -> Result<()> {
    let conn = Connection::connect_to_env().context("connect to wayland")?;
    let mut queue = conn.new_event_queue();
    let handle = queue.handle();
    let _registry = conn.display().get_registry(&handle, ());
    let mut board = Board {
        manager: None,
        seat: None,
        device: None,
        offered: Vec::new(),
        settings,
        out,
    };
    queue.roundtrip(&mut board).context("wayland roundtrip")?;
    let manager = board
        .manager
        .clone()
        .context("wlr-data-control unavailable on this compositor")?;
    let seat = board.seat.clone().context("no wayland seat available")?;
    board.device = Some(manager.get_data_device(&seat, &handle, ()));
    queue
        .roundtrip(&mut board)
        .context("wayland device roundtrip")?;
    debug!("clipboard watcher attached");

    while !stop.load(std::sync::atomic::Ordering::Relaxed) {
        queue
            .blocking_dispatch(&mut board)
            .context("wayland dispatch failed")?;
    }
    Ok(())
}

struct Board {
    manager: Option<zwlr_data_control_manager_v1::ZwlrDataControlManagerV1>,
    seat: Option<wayland_client::protocol::wl_seat::WlSeat>,
    device: Option<wayland_protocols_wlr::data_control::v1::client::zwlr_data_control_device_v1::ZwlrDataControlDeviceV1>,
    offered: Vec<String>,
    settings: Settings,
    out: flume::Sender<Entry>,
}

type Registry = wayland_client::protocol::wl_registry::WlRegistry;
type Seat = wayland_client::protocol::wl_seat::WlSeat;
type Manager = zwlr_data_control_manager_v1::ZwlrDataControlManagerV1;
type Device = wayland_protocols_wlr::data_control::v1::client::zwlr_data_control_device_v1::ZwlrDataControlDeviceV1;
type Offer = wayland_protocols_wlr::data_control::v1::client::zwlr_data_control_offer_v1::ZwlrDataControlOfferV1;
type Source = wayland_protocols_wlr::data_control::v1::client::zwlr_data_control_source_v1::ZwlrDataControlSourceV1;

type RegistryEvent = wayland_client::protocol::wl_registry::Event;
type SeatEvent = wayland_client::protocol::wl_seat::Event;
type ManagerEvent = zwlr_data_control_manager_v1::Event;
type DeviceEvent =
    wayland_protocols_wlr::data_control::v1::client::zwlr_data_control_device_v1::Event;
type OfferEvent =
    wayland_protocols_wlr::data_control::v1::client::zwlr_data_control_offer_v1::Event;
type SourceEvent =
    wayland_protocols_wlr::data_control::v1::client::zwlr_data_control_source_v1::Event;

impl Dispatch<Registry, ()> for Board {
    fn event(
        state: &mut Self,
        registry: &Registry,
        event: RegistryEvent,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let RegistryEvent::Global {
            name,
            interface,
            version,
        } = event
        {
            if interface == "zwlr_data_control_manager_v1" {
                state.manager = Some(registry.bind::<Manager, _, _>(name, version.min(2), qh, ()));
            } else if interface == "wl_seat" && state.seat.is_none() {
                state.seat = Some(registry.bind::<Seat, _, _>(name, version.min(1), qh, ()));
            }
        }
    }
}

impl Dispatch<Seat, ()> for Board {
    fn event(_: &mut Self, _: &Seat, _: SeatEvent, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}

impl Dispatch<Manager, ()> for Board {
    fn event(
        _: &mut Self,
        _: &Manager,
        _: ManagerEvent,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<Offer, ()> for Board {
    fn event(
        state: &mut Self,
        _: &Offer,
        event: OfferEvent,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let OfferEvent::Offer { mime_type } = event {
            state.offered.push(mime_type);
        }
    }
}

impl Dispatch<Source, ()> for Board {
    fn event(
        _: &mut Self,
        _: &Source,
        _: SourceEvent,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<Device, ()> for Board {
    fn event(
        state: &mut Self,
        _: &Device,
        event: DeviceEvent,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            DeviceEvent::Selection { id } => {
                let protected = state
                    .offered
                    .iter()
                    .any(|mime| mime == "x-kde-passwordManagerHint");
                state.offered.clear();
                if id.is_some() && !(protected && !state.settings.keep_passwords) {
                    debug!("selection changed, capturing");
                    capture(&state.out);
                }
            }
            DeviceEvent::PrimarySelection { .. } => {}
            _ => {}
        }
    }

    fn event_created_child(
        opcode: u16,
        qh: &QueueHandle<Self>,
    ) -> std::sync::Arc<dyn wayland_client::backend::ObjectData> {
        match opcode {
            wayland_protocols_wlr::data_control::v1::client::zwlr_data_control_device_v1::EVT_DATA_OFFER_OPCODE => {
                qh.make_data::<Offer, _>(())
            }
            _ => qh.make_data::<Source, _>(()),
        }
    }
}

/// Read whatever the board currently holds: prefer pixels, fall back to text.
fn capture(out: &flume::Sender<Entry>) {
    std::thread::sleep(std::time::Duration::from_millis(60));
    let mut board = match arboard::Clipboard::new() {
        Ok(board) => board,
        Err(e) => {
            warn!("reading clipboard failed: {e}");
            return;
        }
    };
    if let Ok(image) = board.get_image()
        && !image.bytes.is_empty()
    {
        if applied(
            out,
            Entry::new(Content::Image {
                width: image.width,
                height: image.height,
                rgba: image.bytes.to_vec(),
            }),
        ) {
            return;
        }
    }
    if let Ok(text) = board.get_text()
        && !text.is_empty()
    {
        let _ = applied(out, Entry::new(Content::Text(text)));
    }
}

fn applied(out: &flume::Sender<Entry>, entry: Entry) -> bool {
    out.try_send(entry).is_ok()
}
