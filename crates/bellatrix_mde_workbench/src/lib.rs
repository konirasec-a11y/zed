//! Native Zed UI for the Bellatrix MDE model.
#[path = "UI/workbench.rs"]
mod ui;

pub use ui::{
    MODELING_NAMESPACE, MdeSnapshot, MdeSymbolLibraryItem, MdeToolDrag, MdeUiSource,
    MdeWorkbenchItem, NativeDocumentImporter, NativeDocumentLease, WorkbenchPalette,
    from_document, open_in_workspace, panels, register_mde_workspace,
};
