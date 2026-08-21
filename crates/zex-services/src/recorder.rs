use anyhow::{Context, Result};
use flume::Sender;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use tokio::process::Command as TokioCommand;
use tokio::sync::Mutex;
use tokio::time::{Duration, Instant};
use tracing::{debug, error, info, warn};

pub const RECORDINGS_DIR: &str = "Videos";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordingMode {
    Screen,
    Region,
    Portal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndicatorMode {
    Always,
    Recording,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecorderState {
    Idle,
    Recording { started: Instant, mode: RecordingMode },
    Paused { started: Instant, mode: RecordingMode, paused_at: Instant },
}

impl Default for RecorderState {
    fn default() -> Self {
        Self::Idle
    }
}

#[derive(Debug, Clone)]
pub enum RecorderCmd {
    StartScreen,
    StartRegion,
    StartPortal,
    Stop,
    Pause,
    TogglePause,
    GetState(Sender<RecorderState>),
    SetIndicatorMode(IndicatorMode),
}

#[derive(Debug, Clone)]
pub enum RecorderEvent {
    Started { mode: RecordingMode },
    Stopped { path: PathBuf },
    Paused,
    Resumed,
    Error(String),
    TimerTick { elapsed_secs: u64 },
}

pub struct Recorder {
    state: Arc<Mutex<RecorderState>>,
    indicator_mode: IndicatorMode,
    event_tx: Sender<RecorderEvent>,
    child: Arc<Mutex<Option<tokio::process::Child>>>,
    recording_dir: PathBuf,
}

impl Recorder {
    pub fn new(event_tx: Sender<RecorderEvent>) -> Result<Self> {
        let recording_dir = dirs::home_dir()
            .context("no home directory")?
            .join(RECORDINGS_DIR);
        std::fs::create_dir_all(&recording_dir)
            .with_context(|| format!("creating {}", recording_dir.display()))?;

        Ok(Self {
            state: Arc::new(Mutex::new(RecorderState::default())),
            indicator_mode: IndicatorMode::Recording,
            event_tx,
            child: Arc::new(Mutex::new(None)),
            recording_dir,
        })
    }

    pub async fn run(mut self, mut rx: flume::Receiver<RecorderCmd>) {
        let mut timer_interval = tokio::time::interval(Duration::from_secs(1));
        timer_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                biased;
                cmd = rx.recv_async() => {
                    match cmd {
                        Ok(RecorderCmd::StartScreen) => self.start(RecordingMode::Screen).await,
                        Ok(RecorderCmd::StartRegion) => self.start(RecordingMode::Region).await,
                        Ok(RecorderCmd::StartPortal) => self.start(RecordingMode::Portal).await,
                        Ok(RecorderCmd::Stop) => self.stop().await,
                        Ok(RecorderCmd::Pause) => self.pause().await,
                        Ok(RecorderCmd::TogglePause) => self.toggle_pause().await,
                        Ok(RecorderCmd::GetState(tx)) => {
                            let state = self.state.lock().await.clone();
                            let _ = tx.send(state);
                        }
                        Ok(RecorderCmd::SetIndicatorMode(mode)) => {
                            self.indicator_mode = mode;
                        }
                        Err(_) => break,
                    }
                }
                _ = timer_interval.tick() => {
                    let state = self.state.lock().await.clone();
                    if let RecorderState::Recording { started, .. } = state {
                        let elapsed = started.elapsed().as_secs();
                        let _ = self.event_tx.send(RecorderEvent::TimerTick { elapsed_secs: elapsed });
                    }
                }
            }
        }
    }

    async fn start(&self, mode: RecordingMode) {
        let mut state = self.state.lock().await;
        if !matches!(*state, RecorderState::Idle) {
            warn!("recorder already active");
            return;
        }

        let timestamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S");
        let filename = format!("recording_{timestamp}.mp4");
        let output_path = self.recording_dir.join(&filename);

        let (args, portal_mode) = match mode {
            RecordingMode::Screen => (vec!["-w", "screen"], false),
            RecordingMode::Region => (vec!["-w", "region"], true),
            RecordingMode::Portal => (vec!["-w", "portal"], true),
        };

        let mut cmd = TokioCommand::new("gpu-screen-recorder");
        cmd.args([
            "-a", "default",
            "-o", output_path.to_str().unwrap(),
            "-f", "mp4",
        ]);
        cmd.args(&args);
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        if portal_mode {
            cmd.env("GSP_RECORDER_PORTAL", "1");
        }

        match cmd.spawn() {
            Ok(child) => {
                *self.child.lock().await = Some(child);
                *state = RecorderState::Recording { started: tokio::time::Instant::now(), mode };
                let _ = self.event_tx.send(RecorderEvent::Started { mode });
                info!("recording started: {} ({:?})", output_path.display(), mode);
            }
            Err(e) => {
                error!("failed to start gpu-screen-recorder: {e:#}");
                let _ = self.event_tx.send(RecorderEvent::Error(e.to_string()));
            }
        }
    }

    async fn stop(&self) {
        let mut state = self.state.lock().await;
        let RecorderState::Recording { started, mode } = *state else {
            warn!("recorder not recording");
            return;
        };

        if let Some(mut child) = self.child.lock().await.take() {
            child.kill().await.ok();
            let _ = child.wait().await;
        }

        let elapsed = started.elapsed().as_secs();
        let path = self.recording_dir.join(format!("recording_{}.mp4", chrono::Local::now().format("%Y-%m-%d_%H-%M-%S")));

        *state = RecorderState::Idle;
        let _ = self.event_tx.send(RecorderEvent::Stopped { path: path.clone() });
        info!("recording stopped after {}s: {}", elapsed, path.display());
    }

    async fn pause(&self) {
        let mut state = self.state.lock().await;
        let RecorderState::Recording { started, mode } = *state else {
            return;
        };
        *state = RecorderState::Paused { started, mode, paused_at: Instant::now() };
        let _ = self.event_tx.send(RecorderEvent::Paused);
    }

    async fn toggle_pause(&self) {
        let mut state = self.state.lock().await;
        match *state {
            RecorderState::Recording { started, mode } => {
*state = RecorderState::Paused { started, mode, paused_at: tokio::time::Instant::now() };
                let _ = self.event_tx.send(RecorderEvent::Paused);
            }
            RecorderState::Paused { started, mode, paused_at } => {
                let paused_duration = paused_at.elapsed();
                let new_started = started + paused_duration;
                *state = RecorderState::Recording { started: new_started, mode };
                let _ = self.event_tx.send(RecorderEvent::Resumed);
            }
            _ => {}
        }
    }
}

pub fn spawn_recorder(event_tx: Sender<RecorderEvent>) -> (Sender<RecorderCmd>, tokio::task::JoinHandle<()>) {
    let (cmd_tx, cmd_rx) = flume::unbounded();
    let recorder = Recorder::new(event_tx).expect("recorder init");
    let handle = tokio::spawn(async move {
        recorder.run(cmd_rx).await;
    });
    (cmd_tx, handle)
}