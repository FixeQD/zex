//! dbusmenu (`com.canonical.dbusmenu`) `GetLayout` reply parsing

use zbus::zvariant::OwnedValue;

#[derive(Debug, Clone, PartialEq)]
pub struct MenuEntry {
    pub id: i32,
    pub label: String,
    pub enabled: bool,
    pub visible: bool,
    pub children: Vec<MenuEntry>,
}

fn unwrap_value(value: &OwnedValue) -> &zbus::zvariant::Value<'_> {
    match &**value {
        zbus::zvariant::Value::Value(inner) => inner,
        other => other,
    }
}

/// `unwrap_value` for an already-borrowed `Value`
fn unwrap_ref<'a>(value: &'a zbus::zvariant::Value<'a>) -> &'a zbus::zvariant::Value<'a> {
    match value {
        zbus::zvariant::Value::Value(inner) => inner,
        other => other,
    }
}

fn dict_string(props: &zbus::zvariant::Value, key: &str) -> Option<String> {
    let zbus::zvariant::Value::Dict(dict) = unwrap_ref(props) else {
        return None;
    };
    for (k, v) in dict.iter() {
        if let zbus::zvariant::Value::Str(s) = k
            && s == key
        {
            return unwrap_ref(v)
                .downcast_ref::<&str>()
                .ok()
                .map(str::to_string);
        }
    }
    None
}

fn dict_bool(props: &zbus::zvariant::Value, key: &str, default: bool) -> bool {
    let zbus::zvariant::Value::Dict(dict) = unwrap_ref(props) else {
        return default;
    };
    for (k, v) in dict.iter() {
        if let zbus::zvariant::Value::Str(s) = k
            && s == key
        {
            return unwrap_ref(v)
                .downcast_ref::<&bool>()
                .ok()
                .copied()
                .unwrap_or(default);
        }
    }
    default
}

pub fn parse_layout(value: &OwnedValue) -> Vec<MenuEntry> {
    let zbus::zvariant::Value::Array(entries) = unwrap_value(value) else {
        return Vec::new();
    };
    entries.iter().filter_map(parse_entry).collect()
}

fn parse_children(value: &zbus::zvariant::Value) -> Vec<MenuEntry> {
    let zbus::zvariant::Value::Array(children) = unwrap_ref(value) else {
        return Vec::new();
    };
    children.iter().filter_map(parse_entry).collect()
}

fn parse_entry(value: &zbus::zvariant::Value) -> Option<MenuEntry> {
    let zbus::zvariant::Value::Structure(fields) = unwrap_ref(value) else {
        return None;
    };
    let fields = fields.fields();
    let id = *fields.first()?.downcast_ref::<&i32>().ok()?;
    let props = fields.get(1)?;
    let children = match fields.get(2) {
        Some(children) => parse_children(children),
        None => Vec::new(),
    };
    Some(MenuEntry {
        id,
        label: dict_string(props, "label").unwrap_or_default(),
        enabled: dict_bool(props, "enabled", true),
        visible: dict_bool(props, "visible", true),
        children,
    })
}
