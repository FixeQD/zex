use zex_launcher::apps::{AppInfo, DEFAULT_TERMINAL_TEMPLATE, strip_field_codes};

#[test]
fn strips_field_codes() {
    assert_eq!(strip_field_codes("firefox %u"), "firefox");
    assert_eq!(strip_field_codes("code %F %U"), "code");
    assert_eq!(
        strip_field_codes("gimp %i %c --new-instance"),
        "gimp --new-instance"
    );
    assert_eq!(strip_field_codes("  alacritty  "), "alacritty");
}

#[test]
fn wraps_terminal_entries() {
    let app = AppInfo {
        id: "shell".into(),
        title: "Shell".into(),
        command: "fish".into(),
        icon_name: None,
        icon_file: None,
        summary: None,
        tags: vec![],
        wants_terminal: true,
        source: "/tmp/shell.desktop".into(),
    };
    let wrapped = DEFAULT_TERMINAL_TEMPLATE.replace("%command%", &strip_field_codes(&app.command));
    assert_eq!(wrapped, "ghostty fish");
}
