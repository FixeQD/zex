use gtk4::prelude::*;
use tokio::time::Instant;
use zex_services::recorder::{IndicatorMode, RecorderEvent, RecorderState, RecordingMode};

/// Plain GTK widget for recording indicator (not a relm4 component)
pub struct RecordingIndicator {
    root: gtk4::Button,
    timer_label: gtk4::Label,
    state: RecorderState,
    indicator_mode: IndicatorMode,
    event_tx: flume::Sender<RecorderEvent>,
    _event_rx: flume::Receiver<RecorderEvent>,
}

impl RecordingIndicator {
    pub fn new() -> Self {
        let (event_tx, event_rx) = flume::unbounded();

        let root = gtk4::Button::builder()
            .visible(false)
            .css_classes(["recording-indicator", "idle"])
            .tooltip_text("Screen recording")
            .build();

        let icon = gtk4::Image::builder()
            .icon_name("media-record")
            .pixel_size(16)
            .build();

        let timer_label = gtk4::Label::builder()
            .label("00:00")
            .css_classes(["timer"])
            .build();

        let box_ = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(4)
            .build();
        box_.append(&icon);
        box_.append(&timer_label);

        root.set_child(Some(&box_));

        let widget = Self {
            root,
            timer_label,
            state: RecorderState::Idle,
            indicator_mode: IndicatorMode::Recording,
            event_tx,
            _event_rx: event_rx,
        };

        widget.poll_events();
        widget
    }

    pub fn event_sender(&self) -> flume::Sender<RecorderEvent> {
        self.event_tx.clone()
    }

    pub fn widget(&self) -> gtk4::Widget {
        self.root.clone().upcast()
    }

    fn poll_events(&self) {
        let root = self.root.clone();
        let timer_label = self.timer_label.clone();
        let event_rx = self._event_rx.clone();
        let indicator_mode = self.indicator_mode;

        glib::spawn_future_local(async move {
            let mut state = RecorderState::Idle;

            while let Ok(event) = event_rx.recv_async().await {
                match event {
                    RecorderEvent::Started { .. } => {
                        state = RecorderState::Recording {
                            started: Instant::now(),
                            mode: RecordingMode::Screen,
                        };
                        update_ui(&root, &state);
                    }
                    RecorderEvent::Stopped { .. } => {
                        state = RecorderState::Idle;
                        update_ui(&root, &state);
                    }
                    RecorderEvent::Paused => {
                        if let RecorderState::Recording { started, mode } = state {
                            state = RecorderState::Paused { started, mode, paused_at: Instant::now() };
                        }
                        update_ui(&root, &state);
                    }
                    RecorderEvent::Resumed => {
                        update_ui(&root, &state);
                    }
                    RecorderEvent::TimerTick { elapsed_secs } => {
                        let mins = elapsed_secs / 60;
                        let secs = elapsed_secs % 60;
                        timer_label.set_label(&format!("{:02}:{:02}", mins, secs));
                    }
                    RecorderEvent::Error(e) => {
                        tracing::warn!("recorder error: {}", e);
                    }
                }
                update_visibility(&root, indicator_mode, &state);
            }
        });
    }
}

fn update_ui(root: &gtk4::Button, state: &RecorderState) {
    root.remove_css_class("recording");
    root.remove_css_class("paused");
    root.remove_css_class("idle");

    match state {
        RecorderState::Idle => root.add_css_class("idle"),
        RecorderState::Recording { .. } => root.add_css_class("recording"),
        RecorderState::Paused { .. } => root.add_css_class("paused"),
    }
}

fn update_visibility(root: &gtk4::Button, indicator_mode: IndicatorMode, state: &RecorderState) {
    let visible = match (indicator_mode, state) {
        (IndicatorMode::Always, _) => true,
        (IndicatorMode::Recording, RecorderState::Idle) => false,
        (IndicatorMode::Recording, _) => true,
    };
    root.set_visible(visible);
}