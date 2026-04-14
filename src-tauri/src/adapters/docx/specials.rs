use quick_xml::events::{BytesStart, Event};

use super::{model::LockedDisplayMode, placeholders};

pub(crate) struct InlineSpecialRegion {
    pub text: String,
    pub kind: &'static str,
    pub display_mode: LockedDisplayMode,
}

pub(crate) fn is_inline_special_name(name: &[u8]) -> bool {
    matches!(
        name,
        b"drawing" | b"pict" | b"AlternateContent" | b"sdt" | b"fldSimple"
    )
}

pub(crate) fn classify_inline_special_region(
    events: &[Event<'static>],
) -> Result<InlineSpecialRegion, String> {
    if contains_local_tag(events, b"sdt") {
        let (text, kind) = classify_sdt_placeholder(events);
        return Ok(InlineSpecialRegion {
            text: text.to_string(),
            kind,
            display_mode: LockedDisplayMode::Inline,
        });
    }

    if contains_local_tag(events, b"fldSimple") {
        return Ok(InlineSpecialRegion {
            text: extract_field_display_text(events)?,
            kind: "field",
            display_mode: LockedDisplayMode::Inline,
        });
    }

    let (text, kind) = classify_locked_object_placeholder(events)?;
    Ok(InlineSpecialRegion {
        text: text.to_string(),
        kind,
        display_mode: locked_object_display_mode(events),
    })
}

pub(crate) fn classify_block_sdt(events: &[Event<'static>]) -> (&'static str, &'static str) {
    classify_sdt_placeholder(events)
}

fn classify_sdt_placeholder(events: &[Event<'static>]) -> (&'static str, &'static str) {
    if is_toc_sdt(events) {
        return (placeholders::DOCX_TOC_PLACEHOLDER, "toc");
    }
    (
        placeholders::DOCX_CONTENT_CONTROL_PLACEHOLDER,
        "content-control",
    )
}

fn is_toc_sdt(events: &[Event<'static>]) -> bool {
    events.iter().any(|event| match event {
        Event::Start(e) | Event::Empty(e) if local_name(e.name().as_ref()) == b"docPartGallery" => {
            attr_value(e, b"val").is_some_and(|value| {
                let lowered = value.trim().to_ascii_lowercase();
                lowered == "table of contents" || lowered == "toc"
            })
        }
        _ => false,
    })
}

fn extract_field_display_text(events: &[Event<'static>]) -> Result<String, String> {
    let text = extract_visible_text(events)?;
    if text.is_empty() {
        return Ok(placeholders::DOCX_FIELD_PLACEHOLDER.to_string());
    }
    Ok(text)
}

fn extract_visible_text(events: &[Event<'static>]) -> Result<String, String> {
    let mut out = String::new();
    let mut text_depth = 0usize;

    for event in events {
        match event {
            Event::Start(e) if local_name(e.name().as_ref()) == b"t" => text_depth += 1,
            Event::End(e) if local_name(e.name().as_ref()) == b"t" && text_depth > 0 => {
                text_depth -= 1;
            }
            Event::Text(e) if text_depth > 0 => {
                let decoded = e
                    .decode()
                    .map_err(|error| format!("解析 docx 特殊行内内容失败：{error}"))?;
                out.push_str(&decoded);
            }
            Event::CData(e) if text_depth > 0 => {
                let decoded = e
                    .decode()
                    .map_err(|error| format!("解析 docx 特殊行内内容失败：{error}"))?;
                out.push_str(&decoded);
            }
            Event::Empty(e) => append_empty_visible_text(&mut out, e),
            _ => {}
        }
    }

    Ok(out)
}

fn append_empty_visible_text(out: &mut String, event: &BytesStart<'_>) {
    match local_name(event.name().as_ref()) {
        b"tab" => out.push('\t'),
        b"br" | b"cr" => out.push('\n'),
        b"noBreakHyphen" => out.push('\u{2011}'),
        b"softHyphen" => out.push('\u{00ad}'),
        _ => {}
    }
}

fn classify_locked_object_placeholder(
    events: &[Event<'static>],
) -> Result<(&'static str, &'static str), String> {
    if contains_local_tag(events, b"txbxContent") || contains_local_tag(events, b"textbox") {
        return Ok((placeholders::DOCX_TEXTBOX_PLACEHOLDER, "textbox"));
    }
    if contains_local_tag(events, b"pic") {
        return Ok((placeholders::DOCX_IMAGE_PLACEHOLDER, "image"));
    }
    if contains_local_tag(events, b"chart") {
        return Ok((placeholders::DOCX_CHART_PLACEHOLDER, "chart"));
    }
    if contains_local_tag(events, b"wgp") || contains_local_tag(events, b"grpSp") {
        return Ok((placeholders::DOCX_GROUP_SHAPE_PLACEHOLDER, "group-shape"));
    }
    if is_vml_shape_object(events)
        || contains_local_tag(events, b"wsp")
        || contains_local_tag(events, b"sp")
        || contains_local_tag(events, b"cxnSp")
        || contains_local_tag(events, b"graphicFrame")
        || contains_local_tag(events, b"relIds")
        || contains_local_tag(events, b"dataModelExt")
    {
        return Ok((placeholders::DOCX_SHAPE_PLACEHOLDER, "shape"));
    }

    Err("当前仅支持文章语义相关的 docx：无法归类正文中的图形对象，无法安全导入。".to_string())
}

fn locked_object_display_mode(events: &[Event<'static>]) -> LockedDisplayMode {
    if contains_local_tag(events, b"anchor") {
        return LockedDisplayMode::AfterParagraph;
    }
    LockedDisplayMode::Inline
}

fn is_vml_shape_object(events: &[Event<'static>]) -> bool {
    contains_local_tag(events, b"pict")
        && (contains_local_tag(events, b"rect")
            || contains_local_tag(events, b"roundrect")
            || contains_local_tag(events, b"shape")
            || contains_local_tag(events, b"shapetype")
            || contains_local_tag(events, b"line")
            || contains_local_tag(events, b"oval"))
}

fn contains_local_tag(events: &[Event<'static>], tag: &[u8]) -> bool {
    events.iter().any(|event| match event {
        Event::Start(e) | Event::Empty(e) => local_name(e.name().as_ref()) == tag,
        Event::End(e) => local_name(e.name().as_ref()) == tag,
        _ => false,
    })
}

fn attr_value(event: &BytesStart<'_>, key: &[u8]) -> Option<String> {
    event.attributes().flatten().find_map(|attr| {
        if local_name(attr.key.as_ref()) != key {
            return None;
        }
        attr.unescape_value()
            .map(|value| value.into_owned())
            .ok()
            .or_else(|| String::from_utf8(attr.value.as_ref().to_vec()).ok())
    })
}

fn local_name(name: &[u8]) -> &[u8] {
    match name.iter().rposition(|byte| *byte == b':') {
        Some(index) if index + 1 < name.len() => &name[index + 1..],
        _ => name,
    }
}
