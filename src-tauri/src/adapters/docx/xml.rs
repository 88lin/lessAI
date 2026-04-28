use std::collections::HashMap;

use quick_xml::{
    events::{BytesStart, Event},
    Reader,
};

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

pub(super) fn capture_subtree_events(
    reader: &mut Reader<&[u8]>,
    first_event: Event<'static>,
    buf: &mut Vec<u8>,
    source_name: &str,
    unclosed_message: &str,
) -> Result<Vec<Event<'static>>, String> {
    let mut depth = 1usize;
    let mut events = vec![first_event];

    while depth > 0 {
        let event = match reader.read_event_into(buf) {
            Ok(event) => event.into_owned(),
            Err(error) => return Err(format!("解析 {source_name} 失败：{error}")),
        };
        match &event {
            Event::Start(_) => depth += 1,
            Event::End(_) => depth -= 1,
            Event::Eof => return Err(format!("解析 {source_name} 失败：{unclosed_message}")),
            _ => {}
        }
        events.push(event);
        buf.clear();
    }

    Ok(events)
}

pub(super) fn capture_subtree_events_from_slice(
    events: &[Event<'static>],
    start_index: usize,
) -> Result<(Vec<Event<'static>>, usize), String> {
    let Some(first) = events.get(start_index) else {
        return Err("解析 docx 写回模板失败：子树起点越界。".to_string());
    };

    match first {
        Event::Empty(event) => Ok((vec![Event::Empty(event.clone())], start_index + 1)),
        Event::Start(event) => {
            let mut depth = 1usize;
            let mut out = vec![Event::Start(event.clone())];
            let mut index = start_index + 1;
            while index < events.len() {
                let event = events[index].clone();
                match &event {
                    Event::Start(_) => depth += 1,
                    Event::End(_) => {
                        depth -= 1;
                        out.push(event);
                        index += 1;
                        if depth == 0 {
                            return Ok((out, index));
                        }
                        continue;
                    }
                    _ => {}
                }
                out.push(event);
                index += 1;
            }
            Err("解析 docx 写回模板失败：子树未正常闭合。".to_string())
        }
        _ => Err("解析 docx 写回模板失败：非法子树起点。".to_string()),
    }
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
