//! MPRIS players row: cover, title/artist, play-pause overlay

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;
use zex_services::mpris::{MprisPlayer, PlaybackStatus};

use super::SharedMpris;

const ART_FALLBACK: &str = "audio-volume-high";
const MAX_TEXT_CHARS: usize = 52;

struct PlayerRow {
    box_: gtk4::Box,
    cover: gtk4::Image,
    cover_box: gtk4::Box,
    title: gtk4::Label,
    artist: gtk4::Label,
    overlay: gtk4::Box,
    name: String,
}

#[derive(Debug, Clone, PartialEq)]
struct State {
    players: Vec<String>,
    vertical: bool,
    centered: bool,
    dense: bool,
}

pub struct Media {
    container: gtk4::Box,
    rows: RefCell<Vec<PlayerRow>>,
    state: RefCell<State>,
    has_players: RefCell<bool>,
    control: SharedMpris,
}

impl Media {
    pub fn new(control: SharedMpris) -> Rc<Self> {
        let container = gtk4::Box::new(gtk4::Orientation::Horizontal, 4);
        container.set_css_classes(&["media"]);
        Rc::new(Self {
            container,
            rows: RefCell::new(Vec::new()),
            state: RefCell::new(State {
                players: Vec::new(),
                vertical: false,
                centered: false,
                dense: false,
            }),
            has_players: RefCell::new(false),
            control,
        })
    }

    pub fn widget(&self) -> gtk4::Widget {
        self.container.clone().upcast()
    }

    /// Rebuild rows when the player set changed, otherwise just refresh their content
    pub fn update(&self, players: &[MprisPlayer], vertical: bool, centered: bool, density: i8) {
        let dense = density < 0;
        let names: Vec<String> = players.iter().map(|p| p.name.clone()).collect();
        let next = State {
            players: names,
            vertical,
            centered,
            dense,
        };
        let mut state = self.state.borrow_mut();
        if state.players != next.players {
            *state = next.clone();
            self.rebuild_rows(players, &next);
        } else {
            *state = next;
        }
        drop(state);
        let rows = self.rows.borrow();
        for (index, row) in rows.iter().enumerate() {
            if let Some(player) = players.iter().find(|p| p.name == row.name) {
                apply_player(row, player, index, players.len(), &self.state.borrow());
            }
        }
        drop(rows);
        *self.has_players.borrow_mut() = !players.is_empty();
    }

    /// Whether any player is connected; the bar window folds the widget when not
    pub fn has_players(&self) -> bool {
        *self.has_players.borrow()
    }

    fn rebuild_rows(&self, players: &[MprisPlayer], state: &State) {
        self.container.set_css_classes(&["media"]);
        if state.vertical {
            self.container.add_css_class("vertical");
        }
        if state.centered {
            self.container.add_css_class("centered");
        }
        while let Some(child) = self.container.first_child() {
            self.container.remove(&child);
        }
        let mut rows = self.rows.borrow_mut();
        rows.clear();
        for (index, player) in players.iter().enumerate() {
            let row = build_row(
                player,
                index,
                players.len(),
                state,
                Rc::clone(&self.control),
            );
            self.container.append(&row.box_);
            rows.push(row);
        }
    }
}

/// One player's clickable row; labels only on the last player
fn build_row(
    player: &MprisPlayer,
    index: usize,
    total: usize,
    state: &State,
    control: SharedMpris,
) -> PlayerRow {
    let cover_size = if state.dense { 16 } else { 24 };
    let cover = gtk4::Image::new();
    cover.set_pixel_size(cover_size);
    cover.set_css_classes(&["media-cover"]);
    let cover_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    cover_box.set_css_classes(&["media-cover-box"]);
    cover_box.append(&cover);

    let title = gtk4::Label::new(None);
    title.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    title.set_single_line_mode(true);
    title.set_css_classes(&["media-title"]);
    let artist = gtk4::Label::new(None);
    artist.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    artist.set_single_line_mode(true);
    artist.set_css_classes(&["media-artist"]);
    let labels = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    labels.append(&title);
    labels.append(&artist);

    let overlay = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    overlay.set_css_classes(&["media-overlay"]);
    let pause = gtk4::Image::from_icon_name("pause");
    pause.set_pixel_size(16);
    let play = gtk4::Image::from_icon_name("play_arrow");
    play.set_pixel_size(16);
    overlay.append(&pause);
    overlay.append(&play);

    let row = gtk4::Box::new(
        if state.vertical {
            gtk4::Orientation::Vertical
        } else {
            gtk4::Orientation::Horizontal
        },
        4,
    );
    row.set_css_classes(&["media-row"]);
    row.append(&cover_box);
    row.append(&labels);
    row.append(&overlay);

    let show_labels = !state.vertical && index == total - 1;
    labels.set_visible(show_labels);
    if state.centered {
        row.set_width_request(150);
    }

    let gesture = gtk4::GestureClick::new();
    let name = player.name.clone();
    gesture.connect_released(move |_, _, _, _| control.play_pause(&name));
    row.add_controller(gesture);

    PlayerRow {
        box_: row,
        cover,
        cover_box,
        title,
        artist,
        overlay,
        name: player.name.clone(),
    }
}

fn apply_player(row: &PlayerRow, player: &MprisPlayer, index: usize, total: usize, state: &State) {
    row.title.set_text(&truncate(&player.info.title));
    let artist_text = player.info.artist.join(", ");
    row.artist.set_text(&truncate(&artist_text));
    let show_artist = !state.vertical && index == total - 1 && !state.dense;
    row.artist.set_visible(show_artist);
    row.title.set_visible(!state.vertical);

    let icon_size = if state.dense { 16 } else { 24 };
    row.cover.set_pixel_size(icon_size);
    if let Some(texture) = player
        .info
        .art_url
        .as_deref()
        .and_then(texture_from_art_url)
    {
        row.cover.set_paintable(Some(&texture));
    } else {
        row.cover.set_icon_name(Some(ART_FALLBACK));
    }
    row.cover
        .set_opacity(if player.info.playback_status == PlaybackStatus::Playing {
            1.0
        } else {
            0.7
        });
    let playing = player.info.playback_status == PlaybackStatus::Playing;
    row.cover_box
        .set_css_classes(&["media-cover-box", if playing { "playing" } else { "" }]);
    let paused = player.info.playback_status == PlaybackStatus::Paused;
    row.overlay.set_visible(playing || paused);
    row.overlay.set_css_classes(&[
        "media-overlay",
        if paused { "is-paused" } else { "is-playing" },
    ]);
}

fn texture_from_art_url(art_url: &str) -> Option<gtk4::gdk::Texture> {
    let path = percent_decode(art_url.strip_prefix("file://")?)?;
    gtk4::gdk::Texture::from_filename(&path).ok()
}

pub fn percent_decode(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hi = hex_val(bytes[i + 1])?;
            let lo = hex_val(bytes[i + 2])?;
            out.push(hi << 4 | lo);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn truncate(s: &str) -> String {
    if s.chars().count() <= MAX_TEXT_CHARS {
        s.to_owned()
    } else {
        let cut: String = s.chars().take(MAX_TEXT_CHARS).collect();
        format!("{cut}…")
    }
}
