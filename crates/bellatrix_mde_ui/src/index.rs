//! Native Zed UI for the Bellatrix MDE model.
#[path = "pages/home_page.rs"]
pub(crate) mod workbench_ui;

pub use workbench_ui::{
    MODELING_NAMESPACE, MdeSnapshot, MdeSymbolLibraryItem, MdeToolDrag, MdeUiSource,
    MdeWorkbenchItem, NativeDocumentImporter, NativeDocumentLease, WorkbenchPalette,
    open_in_workspace, panels, register_mde_workspace,
};

