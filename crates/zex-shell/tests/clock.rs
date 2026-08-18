//! Clock formatting integration tests

use zex_shell::widgets::{LocalTime, civil_from_days, format_date, format_month, format_time};

fn noon() -> LocalTime {
    LocalTime {
        year: 2026,
        month: 8,
        day: 18,
        weekday: 2, // Tue
        hour: 12,
        minute: 5,
    }
}

#[test]
fn civil_from_days_known_dates() {
    assert_eq!(civil_from_days(0), (1970, 1, 1));
    assert_eq!(civil_from_days(10_957), (2000, 1, 1));
    assert_eq!(civil_from_days(19_782), (2024, 2, 29));
    assert_eq!(civil_from_days(20_454), (2026, 1, 1));
    assert_eq!(civil_from_days(20_683), (2026, 8, 18));
}

#[test]
fn time_formats() {
    let t = noon();
    assert_eq!(format_time(&t, true, false), "12:05");
    assert_eq!(format_time(&t, false, false), "12:05 PM");
    assert_eq!(format_time(&t, true, true), "12\n05");
    assert_eq!(format_time(&t, false, true), "12\n05");

    let midnight = LocalTime {
        hour: 0,
        minute: 0,
        ..noon()
    };
    assert_eq!(format_time(&midnight, false, false), "12:00 AM");
    assert_eq!(format_time(&midnight, true, false), "00:00");

    let late = LocalTime {
        hour: 23,
        minute: 59,
        ..noon()
    };
    assert_eq!(format_time(&late, false, false), "11:59 PM");
}

#[test]
fn date_formats() {
    let t = noon();
    assert_eq!(format_date(&t, false, false), "Tue 18 Aug");
    assert_eq!(format_date(&t, true, false), "Tue Aug 18");
    assert_eq!(format_date(&t, false, true), "18");
    assert_eq!(format_date(&t, true, true), "08");
    assert_eq!(format_month(&t, false), "08");
    assert_eq!(format_month(&t, true), "18");
}
