use std::collections::HashMap;

use quick_xml::events::BytesStart;

pub(super) fn local_name(name: &[u8]) -> &[u8] {
    match name.iter().rposition(|b| *b == b':') {
        Some(pos) if pos + 1 < name.len() => &name[pos + 1..],
        _ => name,
    }
}

pub(super) fn local_name_owned(name: &[u8]) -> Vec<u8> {
    local_name(name).to_vec()
}

pub(super) fn attr_value(bytes: &BytesStart<'_>, key: &[u8]) -> Option<String> {
    for attr in bytes.attributes().flatten() {
        if local_name(attr.key.as_ref()) != key {
            continue;
        }
        if let Ok(value) = attr.unescape_value() {
            return Some(value.into_owned());
        }
        if let Ok(value) = std::str::from_utf8(attr.value.as_ref()) {
            return Some(value.to_string());
        }
    }
    None
}

pub(super) fn toggle_attr_enabled(event: &BytesStart<'_>) -> bool {
    !matches!(
        attr_value(event, b"val")
            .as_deref()
            .map(|value| value.trim().to_ascii_lowercase()),
        Some(value) if matches!(value.as_str(), "0" | "false" | "off" | "none")
    )
}

pub(super) fn underline_enabled(event: &BytesStart<'_>) -> bool {
    !matches!(
        attr_value(event, b"val")
            .as_deref()
            .map(|value| value.trim().to_ascii_lowercase()),
        Some(value) if value == "none"
    )
}

pub(super) fn hyperlink_target(
    event: &BytesStart<'_>,
    hyperlink_targets: &HashMap<String, String>,
) -> Option<String> {
    attr_value(event, b"id")
        .and_then(|id| hyperlink_targets.get(&id).cloned())
        .or_else(|| attr_value(event, b"anchor").map(|anchor| format!("#{anchor}")))
}
