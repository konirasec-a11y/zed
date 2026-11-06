//! Native Zed UI for the Bellatrix MDE model.
#[path = "pages/home_page.rs"]
pub(crate) mod home_page;

pub use home_page::Page;
pub(crate) use home_page::mde_page as workbench_ui;

pub use workbench_ui::{
    MODELING_NAMESPACE, MdeSnapshot, MdeSymbolLibraryItem, MdeToolDrag, MdeUiSource,
    MdeWorkbenchItem, NativeDocumentImporter, NativeDocumentLease, WorkbenchPalette,
    open_in_workspace, panels, register_mde_workspace,
};
