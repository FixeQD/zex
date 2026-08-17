//! End-to-end tests against a live PipeWire session
//! These tests are ignored by default (`cargo test -- --ignored`)

use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};
use tokio::sync::oneshot;
use zex_services::audio::volume::{VolumeState, spawn_volume_monitor};

// E2E tests bind the same first `Audio/Sink` global
static E2E_LOCK: Mutex<()> = Mutex::new(());

fn lock_e2e() -> MutexGuard<'static, ()> {
    E2E_LOCK.lock().unwrap()
}

fn wait_for(timeout: Duration, mut check: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if check() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    false
}

fn wait_ready(mut ready_rx: oneshot::Receiver<()>) -> bool {
    wait_for(Duration::from_secs(10), || match ready_rx.try_recv() {
        Ok(()) => true,
        Err(_) => false,
    })
}

fn current(state: &Arc<Mutex<VolumeState>>) -> (f32, bool) {
    let state = state.lock().unwrap();
    (state.volume, state.muted)
}

#[test]
#[ignore = "needs a running pipewire session"]
fn e2e_volume_monitor_tracks_and_controls_sink() {
    let _guard = lock_e2e();
    // First monitor: drives commands, publishes the observed state
    let state = Arc::new(Mutex::new(VolumeState::default()));
    let (ready_tx, ready_rx) = oneshot::channel();
    let control = spawn_volume_monitor(Arc::clone(&state), ready_tx);
    if !wait_ready(ready_rx) {
        eprintln!("skipping: no running pipewire session");
        return;
    }

    // Second monitor: independent observer of the same sink
    let observer = Arc::new(Mutex::new(VolumeState::default()));
    let (observer_ready_tx, observer_ready_rx) = oneshot::channel();
    let _observer_control = spawn_volume_monitor(Arc::clone(&observer), observer_ready_tx);
    if !wait_ready(observer_ready_rx) {
        eprintln!("skipping: observer monitor never attached");
        return;
    }

    // SetVolume must be reflected on the sink, observable by the second monitor
    control.set_volume(0.6);
    assert!(
        wait_for(Duration::from_secs(5), || {
            let (volume, _) = current(&observer);
            (volume - 0.6).abs() < 0.01
        }),
        "sink volume never reached 0.6 (observed {:?})",
        current(&observer),
    );
    assert_eq!(current(&state), current(&observer), "monitors disagree");

    // ToggleMute must flip the sink mute state
    control.toggle_mute();
    assert!(
        wait_for(Duration::from_secs(5), || {
            let (_, muted) = current(&observer);
            muted
        }),
        "sink never got muted",
    );
    assert_eq!(current(&state), current(&observer), "monitors disagree");

    // And toggle back for hygiene
    control.toggle_mute();
    assert!(
        wait_for(Duration::from_secs(5), || {
            let (_, muted) = current(&observer);
            !muted
        }),
        "sink never got unmuted",
    );

    eprintln!("e2e volume check passed: {:?}", current(&observer));
}

#[test]
#[ignore = "needs a running pipewire session"]
fn e2e_volume_state_updates_on_external_change() {
    let _guard = lock_e2e();
    let state = Arc::new(Mutex::new(VolumeState::default()));
    let (ready_tx, ready_rx) = oneshot::channel();
    let control = spawn_volume_monitor(Arc::clone(&state), ready_tx);
    if !wait_ready(ready_rx) {
        eprintln!("skipping: no running pipewire session");
        return;
    }

    // Move the volume through our own control; the publish loop must follow
    for target in [0.2, 0.8, 0.35] {
        control.set_volume(target);
        assert!(
            wait_for(Duration::from_secs(5), || {
                let (volume, _) = current(&state);
                (volume - target).abs() < 0.01
            }),
            "volume never converged to {target} (state {:?})",
            current(&state),
        );
    }
    eprintln!("e2e volume convergence passed: {:?}", current(&state));
}
