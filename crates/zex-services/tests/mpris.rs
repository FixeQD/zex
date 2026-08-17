//! Tests for the MPRIS media service: metadata parsing plus an integration test against a private D-Bus bus with a fake media player

mod common;

use common::TestBus;
use std::collections::HashMap;
use zbus::zvariant::OwnedValue;
use zex_services::mpris::{MediaInfo, Mpris, MprisEvent, PlaybackStatus};

#[test]
fn playback_status_mapping() {
    assert_eq!(PlaybackStatus::from("Playing"), PlaybackStatus::Playing);
    assert_eq!(PlaybackStatus::from("Paused"), PlaybackStatus::Paused);
    assert_eq!(PlaybackStatus::from("Stopped"), PlaybackStatus::Stopped);
    assert_eq!(PlaybackStatus::from("weird"), PlaybackStatus::Stopped);
}

fn metadata(
    title: Option<&str>,
    artist: Vec<&str>,
    art_url: Option<&str>,
    length_us: Option<i64>,
) -> HashMap<String, OwnedValue> {
    type Value = zbus::zvariant::Value<'static>;
    let mut map = HashMap::new();
    if let Some(title) = title {
        map.insert(
            "xesam:title".into(),
            OwnedValue::try_from(Value::from(title.to_string())).unwrap(),
        );
    }
    map.insert(
        "xesam:artist".into(),
        OwnedValue::try_from(Value::from(
            artist.into_iter().map(String::from).collect::<Vec<_>>(),
        ))
        .unwrap(),
    );
    if let Some(art_url) = art_url {
        map.insert(
            "mpris:artUrl".into(),
            OwnedValue::try_from(Value::from(art_url.to_string())).unwrap(),
        );
    }
    if let Some(length_us) = length_us {
        map.insert("mpris:length".into(), OwnedValue::from(length_us));
    }
    map
}

#[test]
fn metadata_parsing() {
    let map = metadata(
        Some("Title"),
        vec!["Artist One", "Artist Two"],
        Some("file:///cover.png"),
        Some(200_000),
    );
    let info = MediaInfo::from_metadata(&map, PlaybackStatus::Playing);
    assert_eq!(info.title, "Title");
    assert_eq!(info.artist, vec!["Artist One", "Artist Two"]);
    assert_eq!(info.album, "");
    assert_eq!(info.art_url.as_deref(), Some("file:///cover.png"));
    assert_eq!(info.length_us, Some(200_000));
    assert_eq!(info.playback_status, PlaybackStatus::Playing);
    assert_eq!(info.label(), "Artist One, Artist Two - Title");
}

#[test]
fn metadata_missing_fields_default() {
    let info = MediaInfo::from_metadata(&HashMap::new(), PlaybackStatus::Stopped);
    assert_eq!(
        info,
        MediaInfo {
            playback_status: PlaybackStatus::Stopped,
            ..MediaInfo::default()
        }
    );
    assert_eq!(info.label(), "");

    let info =
        MediaInfo::from_metadata(&metadata(None, vec![], None, None), PlaybackStatus::Paused);
    assert_eq!(info.title, "");
    assert_eq!(info.artist, Vec::<String>::new());
}

// ---------------------------------------------------------------------------
// Integration tests against a private bus

use std::sync::Mutex;
use zbus::interface;

struct FakeRoot {
    identity: String,
}

#[interface(name = "org.mpris.MediaPlayer2")]
impl FakeRoot {
    #[zbus(property)]
    fn identity(&self) -> String {
        self.identity.clone()
    }
}

struct FakePlayer {
    play_pause_calls: Mutex<u32>,
}

#[interface(name = "org.mpris.MediaPlayer2.Player")]
impl FakePlayer {
    #[zbus(property)]
    fn playback_status(&self) -> String {
        "Paused".into()
    }

    #[zbus(property)]
    fn metadata(&self) -> HashMap<String, OwnedValue> {
        metadata(Some("Fake Track"), vec!["Fake Artist"], None, None)
    }

    fn play_pause(&self) {
        *self.play_pause_calls.lock().unwrap() += 1;
    }

    fn play(&self) {}

    fn pause(&self) {}

    fn next(&self) {}

    fn previous(&self) {}
}

#[tokio::test]
async fn detects_and_controls_player() {
    let Some(bus) = TestBus::spawn().await else {
        eprintln!("skipping: dbus-daemon not available");
        return;
    };
    let bus_conn = bus.conn.clone();
    let player = FakePlayer {
        play_pause_calls: Mutex::new(0),
    };
    bus_conn
        .object_server()
        .at("/org/mpris/MediaPlayer2", player)
        .await
        .unwrap();
    bus_conn
        .object_server()
        .at(
            "/org/mpris/MediaPlayer2",
            FakeRoot {
                identity: "Fake Player".into(),
            },
        )
        .await
        .unwrap();
    zbus::fdo::DBusProxy::new(&bus_conn)
        .await
        .unwrap()
        .request_name(
            zbus::names::WellKnownName::try_from("org.mpris.MediaPlayer2.zextest").unwrap(),
            zbus::fdo::RequestNameFlags::ReplaceExisting.into(),
        )
        .await
        .unwrap();

    let mpris = Mpris::connect(bus_conn.clone()).await.unwrap();
    let added = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        mpris.events().recv_async(),
    )
    .await
    .expect("initial PlayerAdded");
    match added {
        Ok(MprisEvent::PlayerAdded(player)) => {
            assert_eq!(player.name, "zextest");
            assert_eq!(player.identity, "Fake Player");
            assert_eq!(player.info.title, "Fake Track");
            assert_eq!(player.info.artist, vec!["Fake Artist"]);
            assert_eq!(player.info.playback_status, PlaybackStatus::Paused);
        }
        other => panic!("unexpected event: {other:?}"),
    }

    let players = mpris.players().await.unwrap();
    assert_eq!(players.len(), 1);
    assert_eq!(players[0].bus_name(), "org.mpris.MediaPlayer2.zextest");

    mpris.play_pause("zextest").await.unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    let calls = loop {
        let calls = *bus_conn
            .object_server()
            .interface::<_, FakePlayer>("/org/mpris/MediaPlayer2")
            .await
            .unwrap()
            .get()
            .await
            .play_pause_calls
            .lock()
            .unwrap();
        if calls == 1 || std::time::Instant::now() >= deadline {
            break calls;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    };
    assert_eq!(calls, 1);
}
