use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::rewrite_unit::WritebackSlot;

use super::models::TextTemplateBlock;

pub(crate) fn compute_template_signature(kind: &str, blocks: &[TextTemplateBlock]) -> String {
    signature_hex(&(kind, blocks))
}

pub(crate) fn compute_slot_structure_signature(slots: &[WritebackSlot]) -> String {
    let normalized = slots
        .iter()
        .map(|slot| {
            (
                slot.order,
                slot.editable,
                &slot.role,
                &slot.presentation,
                slot.anchor.as_deref(),
                slot.separator_after.as_str(),
            )
        })
        .collect::<Vec<_>>();
    signature_hex(&normalized)
}

fn signature_hex<T>(value: &T) -> String
where
    T: Serialize,
{
    let bytes = serde_json::to_vec(value).expect("serialize signature payload");
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn slot_structure_signature_ignores_slot_text_changes() {
        let mut slots = vec![crate::rewrite_unit::WritebackSlot::editable(
            "txt:p0:r0:s0",
            0,
            "第一段",
        )];
        slots[0].anchor = Some("txt:p0:r0:s0".to_string());
        let original = super::compute_slot_structure_signature(&slots);

        slots[0].text = "改写后的第一段".to_string();
        let updated = super::compute_slot_structure_signature(&slots);

        assert_eq!(original, updated);
    }
}
