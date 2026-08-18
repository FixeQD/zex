//! Clock module widget

use std::rc::Rc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{Align, Orientation};
use zex_core::Settings;

use super::SharedSettings;
use crate::bar::styles::{BarLike, compact_rank};

const WEEKDAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// Locally formatted wall-clock fields (weekday 0 = Sunday, month 1-12)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalTime {
    pub year: u32,
    pub month: u32,
    pub day: u32,
    pub weekday: u32,
    pub hour: u32,
    pub minute: u32,
}

/// Current local time via `localtime_r`; falls back to UTC when the call fails
pub fn local_time() -> LocalTime {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    let result = unsafe { libc::localtime_r(&secs, &mut tm) };
    if result.is_null() {
        let days = secs.div_euclid(86_400);
        let (year, month, day) = civil_from_days(days);
        let secs_of_day = secs.rem_euclid(86_400);
        LocalTime {
            year,
            month,
            day,
            weekday: (days + 4).rem_euclid(7) as u32,
            hour: (secs_of_day / 3600) as u32,
            minute: ((secs_of_day % 3600) / 60) as u32,
        }
    } else {
        LocalTime {
            year: (tm.tm_year + 1900) as u32,
            month: (tm.tm_mon + 1) as u32,
            day: tm.tm_mday as u32,
            weekday: tm.tm_wday as u32,
            hour: tm.tm_hour as u32,
            minute: tm.tm_min as u32,
        }
    }
}

/// Days since epoch → proleptic Gregorian `(year, month 1-12, day 1-31)`
pub fn civil_from_days(z: i64) -> (u32, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    let year = if month <= 2 { y + 1 } else { y };
    (year as u32, month, day)
}

/// Time label: `%H:%M` / `%I:%M %p` horizontal, `%H%n%M` / `%I%n%M` vertical
pub fn format_time(time: &LocalTime, military: bool, vertical: bool) -> String {
    if vertical {
        format!("{:02}\n{:02}", time.hour, time.minute)
    } else if military {
        format!("{:02}:{:02}", time.hour, time.minute)
    } else {
        let hour12 = ((time.hour + 11) % 12) + 1;
        let period = if time.hour < 12 { "AM" } else { "PM" };
        format!("{hour12:02}:{:02} {period}", time.minute)
    }
}

/// Date label: `%a %d %b` / `%a %b %d` horizontal, `%d` / `%m` vertical
pub fn format_date(time: &LocalTime, swapped: bool, vertical: bool) -> String {
    if vertical {
        if swapped {
            format!("{:02}", time.month)
        } else {
            format!("{:02}", time.day)
        }
    } else if swapped {
        format!(
            "{} {} {}",
            WEEKDAYS[time.weekday as usize],
            MONTHS[(time.month - 1) as usize],
            time.day
        )
    } else {
        format!(
            "{} {} {}",
            WEEKDAYS[time.weekday as usize],
            time.day,
            MONTHS[(time.month - 1) as usize]
        )
    }
}

/// Month label (vertical bars only): `%m` / `%d` depending on the swap
pub fn format_month(time: &LocalTime, swapped: bool) -> String {
    if swapped {
        format!("{:02}", time.day)
    } else {
        format!("{:02}", time.month)
    }
}

/// The bar instance hosting the clock
fn clock_bar(settings: &Settings) -> &dyn BarLike {
    if settings.interface.modules.bar_id.clock == 0 {
        &settings.interface.bar
    } else {
        &settings.interface.bar2
    }
}

pub struct Clock {
    settings: SharedSettings,
    container: gtk4::Box,
    time_label: gtk4::Label,
    separator: gtk4::Label,
    date_label: gtk4::Label,
    date_separator: gtk4::Separator,
    month_label: gtk4::Label,
}

impl Clock {
    pub fn new(settings: SharedSettings) -> Self {
        let container = gtk4::Box::new(Orientation::Horizontal, 0);
        container.set_css_classes(&["clock"]);
        container.set_hexpand(true);
        container.set_vexpand(true);

        let time_label = gtk4::Label::new(None);
        time_label.set_css_classes(&["time"]);
        time_label.set_justify(gtk4::Justification::Center);

        let separator = gtk4::Label::new(Some("•"));
        separator.set_css_classes(&["separator"]);
        separator.set_visible(false);
        separator.set_hexpand(false);
        separator.set_vexpand(false);

        let date_label = gtk4::Label::new(None);
        date_label.set_css_classes(&["date"]);
        date_label.set_justify(gtk4::Justification::Center);

        let date_separator = gtk4::Separator::new(Orientation::Horizontal);
        date_separator.set_hexpand(false);
        date_separator.set_vexpand(false);

        let month_label = gtk4::Label::new(None);
        month_label.set_css_classes(&["month"]);
        month_label.set_justify(gtk4::Justification::Center);
        month_label.set_visible(false);

        container.append(&time_label);
        container.append(&separator);
        container.append(&date_label);
        container.append(&date_separator);
        container.append(&month_label);

        let clock = Self {
            settings,
            container,
            time_label,
            separator,
            date_label,
            date_separator,
            month_label,
        };
        clock.update_layout();
        clock.spawn_poll();
        clock
    }

    /// Refresh the labels every second while the widget lives
    fn spawn_poll(&self) {
        let settings = Rc::clone(&self.settings);
        let time_label = self.time_label.clone();
        let date_label = self.date_label.clone();
        let month_label = self.month_label.clone();
        glib::timeout_add_local(Duration::from_secs(1), move || {
            let snapshot = settings.lock().expect("settings mutex poisoned").clone();
            let now = local_time();
            let bar = clock_bar(&snapshot);
            time_label.set_label(&format_time(
                &now,
                snapshot.interface.modules.options.military_time,
                bar.vertical(),
            ));
            date_label.set_label(&format_date(
                &now,
                snapshot.interface.modules.options.day_month_swapped,
                bar.vertical(),
            ));
            month_label.set_label(&format_month(
                &now,
                snapshot.interface.modules.options.day_month_swapped,
            ));
            glib::ControlFlow::Continue
        });
    }

    /// Recompute visibility, orientation, spacing and alignment from settings
    /// Called on settings changes and at construction
    pub fn update_layout(&self) {
        let snapshot = self
            .settings
            .lock()
            .expect("settings mutex poisoned")
            .clone();
        let options = &snapshot.interface.modules.options;
        let bar = clock_bar(&snapshot);
        let vertical = bar.vertical();
        let rank = compact_rank(bar.density());
        let show_date = options.show_date;

        self.container.set_halign(Align::Fill);
        self.container.set_valign(Align::Fill);
        self.container.set_orientation(if vertical {
            Orientation::Vertical
        } else {
            Orientation::Horizontal
        });

        if show_date {
            self.date_label.set_visible(true);
            if vertical {
                self.separator.set_visible(true);
            } else if rank == 0 {
                self.separator.set_visible(false);
            } else {
                self.separator.set_visible(true);
            }
            if vertical {
                self.date_separator.set_visible(true);
                self.month_label.set_visible(true);
            } else {
                self.date_separator.set_visible(false);
                self.month_label.set_visible(false);
            }
        } else {
            self.separator.set_visible(false);
            self.date_label.set_visible(false);
            self.date_separator.set_visible(false);
            self.month_label.set_visible(false);
        }

        if vertical {
            self.container.set_spacing(0);
            self.container.set_homogeneous(false);
            self.time_label.set_valign(Align::Center);
            self.date_label.set_valign(Align::Center);
        } else if rank == 0 {
            self.container.set_spacing(0);
            self.container.set_homogeneous(true);
            self.container.set_orientation(Orientation::Vertical);
            if show_date {
                self.time_label.set_valign(Align::End);
                self.date_label.set_valign(Align::Start);
            } else {
                self.time_label.set_valign(Align::Center);
                self.date_label.set_valign(Align::Center);
            }
        } else {
            self.container.set_homogeneous(false);
            self.time_label.set_valign(Align::Center);
            self.date_label.set_valign(Align::Center);
            self.container.set_spacing(match rank {
                1 => 10,
                2 => 6,
                _ => 4,
            });
        }
    }

    pub fn widget(&self) -> gtk4::Box {
        self.container.clone()
    }
}
