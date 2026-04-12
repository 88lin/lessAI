# Docx Article Object Placeholder Design

## Goal

Show article-relevant non-text docx objects as visible locked placeholders instead of rejecting import, as long as their XML can be captured and written back unchanged.

## Product Boundary

This feature exists only to improve in-app reading and preserve article structure. It does not make LessAI a general Word object editor.

## Placeholder Policy

A docx object may be downgraded to a locked placeholder only when both conditions hold:

- The object is part of the body reading flow and should remain visible to help users understand article structure.
- The adapter can capture the complete XML subtree for that object and write it back unchanged without editing its internals.

If either condition fails, import must still fail explicitly.

## Placeholder Classes

Keep existing locked placeholders and locked visible objects unchanged:

- `[图片]`
- `[文本框]`
- `[表格]`
- `[目录]`
- `[分节符]`
- visible locked formulas and page breaks

Add the first batch of semi-generic article-object placeholders:

- `[图表]` for chart-like objects
- `[图形]` for regular shapes, SmartArt-like drawings, and other non-image/non-textbox drawing objects
- `[组合图形]` for grouped drawing objects

These placeholders are always locked, always `skip_rewrite = true`, and may never be edited by AI or the plain-text editor.

## Structural Rules

- Inline objects remain inline as locked placeholder regions inside their paragraph flow.
- Block objects become standalone locked blocks at their original structural position.
- The unified segmentation pipeline stays unchanged. These objects enter the common flow only as ordinary locked `TextRegion`s.

## Architecture

Do not add a second docx writeback system. Reuse the existing docx locked-region pipeline:

`docx XML -> locked placeholder region/block -> session chunks -> merged regions -> existing locked-region writeback`

Implementation should stay inside the docx adapter:

- extend object classification in `src-tauri/src/adapters/docx/simple.rs`
- extend placeholder constants in `src-tauri/src/adapters/docx/placeholders.rs`
- keep writeback on top of existing raw-event locked rendering

The key change is classification, not a new writeback layer.

## Hard Reject Boundary

Import must still fail when:

- object boundaries cannot be captured safely
- object XML cannot be preserved as an unchanged locked subtree
- object and editable text are mixed in a way that cannot be split into stable editable and locked regions
- the object is still unknown after first-level classification and cannot be mapped to chart, shape, or grouped-shape

No silent fallback is allowed.

## Writeback Guarantees

If an object is shown as a placeholder, writeback must remain supported.

- The original object XML must round-trip unchanged.
- Editing placeholder text must be rejected during validation or writeback.
- Placeholder-backed objects must never become AI-rewriteable text.

## Non-Goals

- No editing of object internals
- No conversion between placeholders and other object types
- No compatibility hacks for arbitrary Office-only structures

## Testing

Add backend tests that verify:

- import extracts `[图表]`, `[图形]`, and `[组合图形]` for representative docx objects
- chunk round-trip plus merged-region writeback preserves the original object XML
- editing placeholder text is rejected
- unknown objects that cannot be classified safely still fail import
