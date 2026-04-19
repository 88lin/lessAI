use crate::rewrite_unit::WritebackSlot;

use super::models::TextTemplate;

pub(crate) fn rebuild_text(
    template: &TextTemplate,
    slots: &[WritebackSlot],
) -> Result<String, String> {
    let expected_anchors = template
        .blocks
        .iter()
        .flat_map(|block| block.regions.iter().map(|region| region.anchor.as_str()))
        .collect::<Vec<_>>();

    let mut rebuilt = String::new();
    let mut expected_index = 0usize;
    for slot in slots {
        let slot_region_anchor = slot_region_anchor(slot)?;
        while expected_index < expected_anchors.len()
            && expected_anchors[expected_index] != slot_region_anchor
        {
            expected_index += 1;
        }
        if expected_index == expected_anchors.len() {
            return Err(format!(
                "模板区域锚点与槽位锚点不一致：actual={:?}。",
                slot.anchor
            ));
        }
        rebuilt.push_str(&slot.text);
        rebuilt.push_str(&slot.separator_after);
    }

    Ok(rebuilt)
}

fn slot_region_anchor(slot: &WritebackSlot) -> Result<&str, String> {
    let anchor = slot
        .anchor
        .as_deref()
        .ok_or_else(|| "槽位缺少 anchor，无法重建文本。".to_string())?;
    anchor
        .rsplit_once(":s")
        .map(|(region_anchor, _)| region_anchor)
        .ok_or_else(|| format!("槽位 anchor 不是 region-slot 形式：{anchor}。"))
}

#[cfg(test)]
mod tests {
    #[test]
    fn rebuild_text_round_trips_single_paragraph_template() {
        let template = crate::textual_template::models::TextTemplate::single_paragraph(
            "plain_text",
            "txt:p0",
            "第一段\n\n",
        );
        let built = crate::textual_template::slots::build_slots(&template);

        let rebuilt = super::rebuild_text(&template, &built.slots).expect("rebuild text");

        assert_eq!(rebuilt, "第一段\n\n");
    }

    #[test]
    fn rebuild_text_round_trips_multi_slot_region() {
        let template =
            crate::adapters::plain_text::PlainTextAdapter::build_template("第一句。第二句。");
        let built = crate::textual_template::slots::build_slots(&template);

        let rebuilt = super::rebuild_text(&template, &built.slots).expect("rebuild text");

        assert_eq!(rebuilt, "第一句。第二句。");
    }
}
