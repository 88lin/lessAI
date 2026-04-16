use quick_xml::{events::Event, Reader};

use super::xml::{attr_value, hyperlink_target, toggle_attr_enabled, underline_enabled};

fn first_tag(xml: &str) -> quick_xml::events::BytesStart<'static> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();

    loop {
        match reader
            .read_event_into(&mut buf)
            .expect("read event")
            .into_owned()
        {
            Event::Start(event) | Event::Empty(event) => return event,
            Event::Eof => panic!("expected start tag"),
            _ => {}
        }
        buf.clear();
    }
}

#[test]
fn attr_value_matches_local_name_for_namespaced_attribute() {
    let event = first_tag(r#"<w:b xmlns:w="urn:test" w:val="false"/>"#);

    assert_eq!(attr_value(&event, b"val").as_deref(), Some("false"));
}

#[test]
fn toggle_attr_enabled_treats_false_like_disabled() {
    let event = first_tag(r#"<w:b xmlns:w="urn:test" w:val="false"/>"#);

    assert!(!toggle_attr_enabled(&event));
}

#[test]
fn underline_enabled_only_disables_none() {
    let off = first_tag(r#"<w:u xmlns:w="urn:test" w:val="none"/>"#);
    let on = first_tag(r#"<w:u xmlns:w="urn:test" w:val="single"/>"#);

    assert!(!underline_enabled(&off));
    assert!(underline_enabled(&on));
}

#[test]
fn hyperlink_target_uses_relationship_id_or_anchor() {
    let relationship = first_tag(r#"<w:hyperlink xmlns:w="urn:test" r:id="rId7"/>"#);
    let anchor = first_tag(r#"<w:hyperlink xmlns:w="urn:test" w:anchor="section-2"/>"#);
    let targets =
        std::collections::HashMap::from([(String::from("rId7"), String::from("https://x"))]);

    assert_eq!(
        hyperlink_target(&relationship, &targets).as_deref(),
        Some("https://x")
    );
    assert_eq!(
        hyperlink_target(&anchor, &std::collections::HashMap::new()).as_deref(),
        Some("#section-2")
    );
}
