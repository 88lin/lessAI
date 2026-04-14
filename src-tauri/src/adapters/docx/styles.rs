use std::collections::{HashMap, HashSet};

use quick_xml::{
    events::{BytesStart, Event},
    Reader,
};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct PartialNumberingSpec {
    pub num_id: Option<String>,
    pub ilvl: Option<i32>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ParagraphStyles {
    styles_by_id: HashMap<String, ParagraphStyle>,
}

#[derive(Debug, Clone)]
struct ParagraphStyle {
    name: Option<String>,
    based_on: Option<String>,
    numbering: PartialNumberingSpec,
}

#[derive(Debug, Clone, Default)]
struct ResolvedStyle {
    heading: bool,
    numbering: PartialNumberingSpec,
}

pub(crate) fn parse_styles_xml(xml: &str) -> Result<ParagraphStyles, String> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut styles_by_id = HashMap::new();

    loop {
        let event = match reader.read_event_into(&mut buf) {
            Ok(event) => event.into_owned(),
            Err(error) => return Err(format!("解析 styles.xml 失败：{error}")),
        };

        match event {
            Event::Start(e) if is_paragraph_style(&e) => {
                let events = capture_subtree_events(&mut reader, Event::Start(e), &mut buf)?;
                let (style_id, style) = parse_paragraph_style_events(&events)?;
                styles_by_id.insert(style_id, style);
            }
            Event::Empty(e) if is_paragraph_style(&e) => {
                let style_id = style_id(&e)
                    .ok_or_else(|| "styles.xml 中的段落样式缺少 styleId。".to_string())?;
                styles_by_id.insert(style_id, ParagraphStyle::default());
            }
            Event::Eof => break,
            _ => {}
        }

        buf.clear();
    }

    Ok(ParagraphStyles { styles_by_id })
}

impl ParagraphStyles {
    pub(crate) fn resolve_numbering(&self, style_id: &str) -> PartialNumberingSpec {
        self.resolve_style(style_id).numbering
    }

    pub(crate) fn is_heading(&self, style_id: &str) -> bool {
        self.resolve_style(style_id).heading
    }

    fn resolve_style(&self, style_id: &str) -> ResolvedStyle {
        let mut visiting = HashSet::new();
        resolve_style_recursive(&self.styles_by_id, style_id, &mut visiting)
    }
}

impl Default for ParagraphStyle {
    fn default() -> Self {
        Self {
            name: None,
            based_on: None,
            numbering: PartialNumberingSpec::default(),
        }
    }
}

fn resolve_style_recursive(
    styles_by_id: &HashMap<String, ParagraphStyle>,
    style_id: &str,
    visiting: &mut HashSet<String>,
) -> ResolvedStyle {
    if !visiting.insert(style_id.to_string()) {
        return ResolvedStyle::default();
    }

    let resolved = styles_by_id
        .get(style_id)
        .map(|style| merge_style_with_base(styles_by_id, style_id, style, visiting))
        .unwrap_or_default();

    visiting.remove(style_id);
    resolved
}

fn merge_style_with_base(
    styles_by_id: &HashMap<String, ParagraphStyle>,
    style_id: &str,
    style: &ParagraphStyle,
    visiting: &mut HashSet<String>,
) -> ResolvedStyle {
    let mut resolved = style
        .based_on
        .as_deref()
        .map(|base_style_id| resolve_style_recursive(styles_by_id, base_style_id, visiting))
        .unwrap_or_default();
    resolved.heading = resolved.heading
        || looks_like_heading(style_id)
        || style.name.as_deref().is_some_and(looks_like_heading);
    resolved.numbering = merge_numbering(resolved.numbering, style.numbering.clone());
    resolved
}

fn merge_numbering(
    base: PartialNumberingSpec,
    overlay: PartialNumberingSpec,
) -> PartialNumberingSpec {
    PartialNumberingSpec {
        num_id: overlay.num_id.or(base.num_id),
        ilvl: overlay.ilvl.or(base.ilvl),
    }
}

fn parse_paragraph_style_events(
    events: &[Event<'static>],
) -> Result<(String, ParagraphStyle), String> {
    let Some(Event::Start(start)) = events.first() else {
        return Err("styles.xml 中的段落样式事件非法。".to_string());
    };
    let style_id =
        style_id(start).ok_or_else(|| "styles.xml 中的段落样式缺少 styleId。".to_string())?;

    let mut style = ParagraphStyle::default();
    let mut ppr_depth = 0usize;
    let mut numpr_depth = 0usize;

    for event in events.iter().skip(1).take(events.len().saturating_sub(2)) {
        match event {
            Event::Start(e) => {
                handle_style_start(e, &mut style, &mut ppr_depth, &mut numpr_depth);
            }
            Event::Empty(e) => handle_style_empty(e, &mut style, ppr_depth, numpr_depth),
            Event::End(e) => handle_style_end(e, &mut ppr_depth, &mut numpr_depth),
            _ => {}
        }
    }

    Ok((style_id, style))
}

fn handle_style_start(
    event: &BytesStart<'_>,
    style: &mut ParagraphStyle,
    ppr_depth: &mut usize,
    numpr_depth: &mut usize,
) {
    let name_binding = event.name();
    let name = local_name(name_binding.as_ref());
    if name == b"pPr" {
        *ppr_depth += 1;
        return;
    }
    if *ppr_depth > 0 && name == b"numPr" {
        *numpr_depth += 1;
    }
    capture_style_fields(event, style, *numpr_depth > 0);
}

fn handle_style_empty(
    event: &BytesStart<'_>,
    style: &mut ParagraphStyle,
    ppr_depth: usize,
    numpr_depth: usize,
) {
    let name_binding = event.name();
    let name = local_name(name_binding.as_ref());
    if name == b"pPr" || (ppr_depth > 0 && name == b"numPr") {
        return;
    }
    capture_style_fields(event, style, numpr_depth > 0);
}

fn handle_style_end(
    event: &quick_xml::events::BytesEnd<'_>,
    ppr_depth: &mut usize,
    numpr_depth: &mut usize,
) {
    match local_name(event.name().as_ref()) {
        b"numPr" if *numpr_depth > 0 => *numpr_depth -= 1,
        b"pPr" if *ppr_depth > 0 => *ppr_depth -= 1,
        _ => {}
    }
}

fn capture_style_fields(event: &BytesStart<'_>, style: &mut ParagraphStyle, in_numpr: bool) {
    match local_name(event.name().as_ref()) {
        b"name" => style.name = attr_value(event, b"val"),
        b"basedOn" => style.based_on = attr_value(event, b"val"),
        b"numId" if in_numpr => style.numbering.num_id = attr_value(event, b"val"),
        b"ilvl" if in_numpr => {
            style.numbering.ilvl = attr_value(event, b"val").and_then(|value| value.parse().ok());
        }
        _ => {}
    }
}

fn capture_subtree_events(
    reader: &mut Reader<&[u8]>,
    first_event: Event<'static>,
    buf: &mut Vec<u8>,
) -> Result<Vec<Event<'static>>, String> {
    let mut depth = 1usize;
    let mut events = vec![first_event];

    while depth > 0 {
        let event = match reader.read_event_into(buf) {
            Ok(event) => event.into_owned(),
            Err(error) => return Err(format!("解析 styles.xml 失败：{error}")),
        };
        match &event {
            Event::Start(_) => depth += 1,
            Event::End(_) => depth -= 1,
            Event::Eof => return Err("解析 styles.xml 失败：样式节点未正常闭合。".to_string()),
            _ => {}
        }
        events.push(event);
        buf.clear();
    }

    Ok(events)
}

fn is_paragraph_style(event: &BytesStart<'_>) -> bool {
    local_name(event.name().as_ref()) == b"style"
        && attr_value(event, b"type").as_deref() == Some("paragraph")
}

fn style_id(event: &BytesStart<'_>) -> Option<String> {
    attr_value(event, b"styleId")
}

fn looks_like_heading(value: &str) -> bool {
    let lowered = value.trim().to_ascii_lowercase();
    lowered.starts_with("heading") || matches!(lowered.as_str(), "title" | "subtitle")
}

fn attr_value(event: &BytesStart<'_>, key: &[u8]) -> Option<String> {
    for attr in event.attributes().flatten() {
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

fn local_name(name: &[u8]) -> &[u8] {
    match name.iter().rposition(|byte| *byte == b':') {
        Some(index) if index + 1 < name.len() => &name[index + 1..],
        _ => name,
    }
}
