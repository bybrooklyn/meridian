//! Compatibility facade for Meridian's modular retained UI framework.
//!
//! New code may depend on the narrow owning crate. Existing consumers can
//! migrate incrementally through this facade without observing adapter types.

pub use meridian_ui_core::*;
pub use meridian_ui_render::*;
pub use meridian_ui_runtime::*;
pub use meridian_ui_semantics::*;
pub use meridian_ui_text::{
    UiClipboardRequest, UiGlyphBitmap, UiTextCursorDirection, UiTextInputSnapshot, UiTextLayout,
    UiTextRaster, UiTextSelection,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compatibility_facade_preserves_owning_crate_types() {
        let facade_id = UiNodeId::new(7);
        let owning_id: meridian_ui_core::UiNodeId = facade_id;
        let document = runtime_overlay_document().expect("compatibility fixture");
        let mut runtime = UiRuntime::new(document);
        let output = runtime.reconcile(UiFrameInput::new(UiSize::new(320.0, 180.0)));
        assert_eq!(owning_id, facade_id);
        assert!(!output.display_list.primitives.is_empty());
    }
}
