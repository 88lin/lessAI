use std::collections::HashMap;

use quick_xml::{
    events::{BytesStart, Event},
    Reader,
};

use super::styles::{ParagraphStyles, PartialNumberingSpec};

#[derive(Debug, Clone, Default)]
pub(crate) struct NumberingDefinitions {
    levels_by_num_id: HashMap<String, HashMap<i32, NumberingLevel>>,
    style_bindings_by_style_id: HashMap<String, ResolvedParagraphNumbering>,
}

#[derive(Debug, Default)]
pub(crate) struct NumberingTracker {
    current_values: HashMap<(String, i32), i32>,
}

#[derive(Debug, Clone)]
struct NumberingLevel {
    start: i32,
    format: String,
    level_text: String,
    suffix: NumberSuffix,
    paragraph_style_id: Option<String>,
}

#[derive(Debug, Clone, Copy, Default)]
enum NumberSuffix {
    #[default]
    Nothing,
    Space,
    Tab,
}

#[derive(Debug, Default)]
struct PendingLevel {
    ilvl: Option<i32>,
    start: Option<i32>,
    format: Option<String>,
    level_text: Option<String>,
    suffix: NumberSuffix,
    paragraph_style_id: Option<String>,
}

#[derive(Debug, Default)]
struct ParagraphProperties {
    style_id: Option<String>,
    direct_numbering: PartialNumberingSpec,
}

#[derive(Debug, Clone)]
struct ResolvedParagraphNumbering {
    num_id: String,
    ilvl: i32,
}

#[derive(Debug)]
struct NumberingInstanceDefinition {
    num_id: String,
    abstract_id: String,
    level_overrides: HashMap<i32, LevelOverrideDefinition>,
}

#[derive(Debug, Default)]
struct LevelOverrideDefinition {
    start_override: Option<i32>,
    level: Option<NumberingLevel>,
}

pub(crate) fn parse_numbering_xml(xml: &str) -> Result<NumberingDefinitions, String> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut abstract_levels = HashMap::new();
    let mut numbering_instances = Vec::new();

    loop {
        let event = match reader.read_event_into(&mut buf) {
            Ok(event) => event.into_owned(),
            Err(error) => return Err(format!("解析 numbering.xml 失败：{error}")),
        };

        match event {
            Event::Start(e) if local_name(e.name().as_ref()) == b"abstractNum" => {
                let events = capture_subtree_events(&mut reader, Event::Start(e), &mut buf)?;
                let (abstract_id, levels) = parse_abstract_num_events(&events)?;
                abstract_levels.insert(abstract_id, levels);
            }
            Event::Start(e) if local_name(e.name().as_ref()) == b"num" => {
                let events = capture_subtree_events(&mut reader, Event::Start(e), &mut buf)?;
                numbering_instances.push(parse_num_events(&events)?);
            }
            Event::Eof => break,
            _ => {}
        }

        buf.clear();
    }

    build_numbering_definitions(abstract_levels, numbering_instances)
}

pub(crate) fn list_marker_for_paragraph(
    definitions: &NumberingDefinitions,
    styles: &ParagraphStyles,
    tracker: &mut NumberingTracker,
    paragraph_property_events: &[Event<'static>],
) -> Option<String> {
    let properties = parse_paragraph_properties(paragraph_property_events);
    let spec = resolve_paragraph_numbering(styles, definitions, properties)?;
    let level = definitions.level(&spec.num_id, spec.ilvl)?;
    let current_value = tracker.next_value(&spec.num_id, spec.ilvl, level.start);
    let marker = render_level_text(definitions, tracker, &spec, current_value)?;
    tracker.commit(&spec.num_id, spec.ilvl, current_value);
    Some(marker)
}

impl NumberingDefinitions {
    fn level(&self, num_id: &str, ilvl: i32) -> Option<&NumberingLevel> {
        self.levels_by_num_id
            .get(num_id)
            .and_then(|levels| levels.get(&ilvl))
    }

    fn style_binding(&self, style_id: &str) -> Option<ResolvedParagraphNumbering> {
        self.style_bindings_by_style_id.get(style_id).cloned()
    }
}

impl NumberingTracker {
    fn next_value(&self, num_id: &str, ilvl: i32, start: i32) -> i32 {
        self.current_values
            .get(&(num_id.to_string(), ilvl))
            .map(|value| value + 1)
            .unwrap_or(start)
    }

    fn commit(&mut self, num_id: &str, ilvl: i32, value: i32) {
        self.current_values
            .insert((num_id.to_string(), ilvl), value);
        self.current_values
            .retain(|(existing_num_id, existing_ilvl), _| {
                existing_num_id != num_id || *existing_ilvl <= ilvl
            });
    }

    fn render_value(
        &self,
        num_id: &str,
        reference_ilvl: i32,
        start: i32,
        current_ilvl: i32,
        current_value: i32,
    ) -> i32 {
        if reference_ilvl == current_ilvl {
            return current_value;
        }
        self.current_values
            .get(&(num_id.to_string(), reference_ilvl))
            .copied()
            .unwrap_or(start)
    }
}

impl PendingLevel {
    fn finish(self) -> Option<(i32, NumberingLevel)> {
        Some((
            self.ilvl?,
            NumberingLevel {
                start: self.start.unwrap_or(1),
                format: self.format.unwrap_or_else(|| "decimal".to_string()),
                level_text: self.level_text.unwrap_or_else(|| "%1.".to_string()),
                suffix: self.suffix,
                paragraph_style_id: self.paragraph_style_id,
            },
        ))
    }
}

impl NumberSuffix {
    fn from_attr(value: Option<String>) -> Self {
        match value.as_deref() {
            Some("space") => Self::Space,
            Some("tab") => Self::Tab,
            _ => Self::Nothing,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Nothing => "",
            Self::Space => " ",
            Self::Tab => "\t",
        }
    }
}

fn build_numbering_definitions(
    abstract_levels: HashMap<String, HashMap<i32, NumberingLevel>>,
    numbering_instances: Vec<NumberingInstanceDefinition>,
) -> Result<NumberingDefinitions, String> {
    let mut levels_by_num_id = HashMap::new();
    let mut style_bindings_by_style_id = HashMap::new();

    for instance in numbering_instances {
        let mut levels = abstract_levels
            .get(&instance.abstract_id)
            .cloned()
            .ok_or_else(|| {
                format!(
                    "numbering.xml 中的 numId={} 引用了缺失的 abstractNumId={}。",
                    instance.num_id, instance.abstract_id
                )
            })?;
        apply_level_overrides(&mut levels, instance.level_overrides);
        register_style_bindings(&mut style_bindings_by_style_id, &instance.num_id, &levels);
        levels_by_num_id.insert(instance.num_id, levels);
    }

    Ok(NumberingDefinitions {
        levels_by_num_id,
        style_bindings_by_style_id,
    })
}

fn apply_level_overrides(
    levels: &mut HashMap<i32, NumberingLevel>,
    overrides: HashMap<i32, LevelOverrideDefinition>,
) {
    for (ilvl, level_override) in overrides {
        if let Some(level) = level_override.level {
            levels.insert(ilvl, level);
        }
        if let Some(start_override) = level_override.start_override {
            if let Some(level) = levels.get_mut(&ilvl) {
                level.start = start_override;
            }
        }
    }
}

fn register_style_bindings(
    bindings: &mut HashMap<String, ResolvedParagraphNumbering>,
    num_id: &str,
    levels: &HashMap<i32, NumberingLevel>,
) {
    for (ilvl, level) in levels {
        let Some(style_id) = level.paragraph_style_id.as_deref() else {
            continue;
        };
        bindings
            .entry(style_id.to_string())
            .or_insert_with(|| ResolvedParagraphNumbering {
                num_id: num_id.to_string(),
                ilvl: *ilvl,
            });
    }
}

fn parse_abstract_num_events(
    events: &[Event<'static>],
) -> Result<(String, HashMap<i32, NumberingLevel>), String> {
    let Some(Event::Start(start)) = events.first() else {
        return Err("numbering.xml 中的 abstractNum 事件非法。".to_string());
    };
    let abstract_id = attr_value(start, b"abstractNumId")
        .ok_or_else(|| "numbering.xml 中的 abstractNum 缺少 abstractNumId。".to_string())?;
    let mut levels = HashMap::new();
    let mut index = 1usize;

    while index + 1 < events.len() {
        if !matches!(events.get(index), Some(Event::Start(e)) if local_name(e.name().as_ref()) == b"lvl")
        {
            index += 1;
            continue;
        }
        let (level_events, next_index) = capture_nested_events(events, index)?;
        let (ilvl, level) = parse_level_events(&level_events)?;
        levels.insert(ilvl, level);
        index = next_index;
    }

    Ok((abstract_id, levels))
}

fn parse_num_events(events: &[Event<'static>]) -> Result<NumberingInstanceDefinition, String> {
    let Some(Event::Start(start)) = events.first() else {
        return Err("numbering.xml 中的 num 事件非法。".to_string());
    };
    let num_id = attr_value(start, b"numId")
        .ok_or_else(|| "numbering.xml 中的 num 缺少 numId。".to_string())?;
    let abstract_id = events.iter().find_map(|event| match event {
        Event::Start(e) | Event::Empty(e) if local_name(e.name().as_ref()) == b"abstractNumId" => {
            attr_value(e, b"val")
        }
        _ => None,
    });

    let abstract_id = abstract_id
        .ok_or_else(|| format!("numbering.xml 中的 numId={num_id} 缺少 abstractNumId。"))?;
    let mut level_overrides = HashMap::new();
    let mut index = 1usize;

    while index + 1 < events.len() {
        if !matches!(events.get(index), Some(Event::Start(e)) if local_name(e.name().as_ref()) == b"lvlOverride")
        {
            index += 1;
            continue;
        }
        let (override_events, next_index) = capture_nested_events(events, index)?;
        let (ilvl, level_override) = parse_level_override_events(&override_events)?;
        level_overrides.insert(ilvl, level_override);
        index = next_index;
    }

    Ok(NumberingInstanceDefinition {
        num_id,
        abstract_id,
        level_overrides,
    })
}

fn parse_level_events(events: &[Event<'static>]) -> Result<(i32, NumberingLevel), String> {
    let Some(Event::Start(start)) = events.first() else {
        return Err("numbering.xml 中的 lvl 事件非法。".to_string());
    };
    let mut level = PendingLevel {
        ilvl: attr_value(start, b"ilvl").and_then(|value| value.parse().ok()),
        ..PendingLevel::default()
    };

    for event in events.iter().skip(1).take(events.len().saturating_sub(2)) {
        let (Event::Start(e) | Event::Empty(e)) = event else {
            continue;
        };
        match local_name(e.name().as_ref()) {
            b"start" => level.start = attr_value(e, b"val").and_then(|value| value.parse().ok()),
            b"numFmt" => level.format = attr_value(e, b"val"),
            b"lvlText" => level.level_text = attr_value(e, b"val"),
            b"suff" => level.suffix = NumberSuffix::from_attr(attr_value(e, b"val")),
            b"pStyle" => level.paragraph_style_id = attr_value(e, b"val"),
            _ => {}
        }
    }

    level
        .finish()
        .ok_or_else(|| "numbering.xml 中的 lvl 缺少 ilvl。".to_string())
}

fn parse_level_override_events(
    events: &[Event<'static>],
) -> Result<(i32, LevelOverrideDefinition), String> {
    let Some(Event::Start(start)) = events.first() else {
        return Err("numbering.xml 中的 lvlOverride 事件非法。".to_string());
    };
    let ilvl = attr_value(start, b"ilvl")
        .and_then(|value| value.parse().ok())
        .ok_or_else(|| "numbering.xml 中的 lvlOverride 缺少 ilvl。".to_string())?;
    let mut level_override = LevelOverrideDefinition::default();
    let mut index = 1usize;

    while index + 1 < events.len() {
        match events.get(index) {
            Some(Event::Start(e)) if local_name(e.name().as_ref()) == b"lvl" => {
                let (level_events, next_index) = capture_nested_events(events, index)?;
                let (_, level) = parse_level_events(&level_events)?;
                level_override.level = Some(level);
                index = next_index;
            }
            Some(Event::Start(e) | Event::Empty(e))
                if local_name(e.name().as_ref()) == b"startOverride" =>
            {
                level_override.start_override =
                    attr_value(e, b"val").and_then(|value| value.parse().ok());
                index += 1;
            }
            _ => index += 1,
        }
    }

    Ok((ilvl, level_override))
}

fn parse_paragraph_properties(events: &[Event<'static>]) -> ParagraphProperties {
    let mut properties = ParagraphProperties::default();

    for event in events {
        let (Event::Start(e) | Event::Empty(e)) = event else {
            continue;
        };
        match local_name(e.name().as_ref()) {
            b"pStyle" => properties.style_id = attr_value(e, b"val"),
            b"numId" => properties.direct_numbering.num_id = attr_value(e, b"val"),
            b"ilvl" => {
                properties.direct_numbering.ilvl =
                    attr_value(e, b"val").and_then(|value| value.parse().ok());
            }
            _ => {}
        }
    }

    properties
}

fn resolve_paragraph_numbering(
    styles: &ParagraphStyles,
    definitions: &NumberingDefinitions,
    properties: ParagraphProperties,
) -> Option<ResolvedParagraphNumbering> {
    let style_numbering = properties
        .style_id
        .as_deref()
        .map(|style_id| styles.resolve_numbering(style_id))
        .unwrap_or_default();
    if let Some(num_id) = properties
        .direct_numbering
        .num_id
        .clone()
        .or(style_numbering.num_id.clone())
    {
        let ilvl = properties
            .direct_numbering
            .ilvl
            .or(style_numbering.ilvl)
            .unwrap_or(0);
        return Some(ResolvedParagraphNumbering { num_id, ilvl });
    }

    properties
        .style_id
        .as_deref()
        .and_then(|style_id| definitions.style_binding(style_id))
}

fn render_level_text(
    definitions: &NumberingDefinitions,
    tracker: &NumberingTracker,
    spec: &ResolvedParagraphNumbering,
    current_value: i32,
) -> Option<String> {
    let level = definitions.level(&spec.num_id, spec.ilvl)?;
    if level.format == "bullet" {
        return Some(format!("{}{}", level.level_text, level.suffix.as_str()));
    }

    let mut text = level.level_text.clone();
    for placeholder_index in 1..=9 {
        let token = format!("%{placeholder_index}");
        if !text.contains(&token) {
            continue;
        }
        let reference_ilvl = placeholder_index - 1;
        let reference_level = definitions.level(&spec.num_id, reference_ilvl)?;
        let value = tracker.render_value(
            &spec.num_id,
            reference_ilvl,
            reference_level.start,
            spec.ilvl,
            current_value,
        );
        text = text.replace(&token, &render_counter(&reference_level.format, value));
    }

    Some(format!("{}{}", text, level.suffix.as_str()))
}

fn render_counter(format: &str, value: i32) -> String {
    match format {
        "lowerLetter" => alpha_counter(value, false),
        "upperLetter" => alpha_counter(value, true),
        "lowerRoman" => roman_counter(value).to_ascii_lowercase(),
        "upperRoman" => roman_counter(value),
        "none" => String::new(),
        _ => value.to_string(),
    }
}

fn alpha_counter(value: i32, uppercase: bool) -> String {
    if value <= 0 {
        return value.to_string();
    }

    let mut number = value;
    let mut out = Vec::new();
    while number > 0 {
        number -= 1;
        out.push((b'a' + (number % 26) as u8) as char);
        number /= 26;
    }
    out.reverse();

    let text = out.into_iter().collect::<String>();
    if uppercase {
        text.to_ascii_uppercase()
    } else {
        text
    }
}

fn roman_counter(value: i32) -> String {
    if value <= 0 {
        return value.to_string();
    }

    let numerals = [
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];
    let mut number = value;
    let mut out = String::new();
    for (unit, symbol) in numerals {
        while number >= unit {
            out.push_str(symbol);
            number -= unit;
        }
    }
    out
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
            Err(error) => return Err(format!("解析 numbering.xml 失败：{error}")),
        };
        match &event {
            Event::Start(_) => depth += 1,
            Event::End(_) => depth -= 1,
            Event::Eof => return Err("解析 numbering.xml 失败：编号节点未正常闭合。".to_string()),
            _ => {}
        }
        events.push(event);
        buf.clear();
    }

    Ok(events)
}

fn capture_nested_events(
    events: &[Event<'static>],
    start_index: usize,
) -> Result<(Vec<Event<'static>>, usize), String> {
    let Some(first) = events.get(start_index) else {
        return Err("解析 numbering.xml 失败：编号子树起点越界。".to_string());
    };
    match first {
        Event::Start(start) => {
            let mut depth = 1usize;
            let mut index = start_index + 1;
            let mut out = vec![Event::Start(start.clone())];
            while index < events.len() {
                let event = events[index].clone();
                match &event {
                    Event::Start(_) => depth += 1,
                    Event::End(_) => depth -= 1,
                    _ => {}
                }
                out.push(event);
                index += 1;
                if depth == 0 {
                    return Ok((out, index));
                }
            }
            Err("解析 numbering.xml 失败：编号子树未正常闭合。".to_string())
        }
        _ => Err("解析 numbering.xml 失败：非法的编号子树起点。".to_string()),
    }
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
