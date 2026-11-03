//! Schedules engine validation without changing the semantic or presentation model.
use crate::ui::{MdeUiCommand, MdeWorkbenchItem};
use gpui::Context;

static PREFERENCE_WRITES: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ValidationMode {
    Manual,
    OnSave,
    OnChange,
}

impl ValidationMode {
    pub(super) fn load(project_id: &str) -> Result<Self, String> {
        let value = db::kvp::GlobalKeyValueStore::global()
            .read_kvp(&Self::key(project_id))
            .map_err(|error| error.to_string())?;
        Ok(match value.as_deref() {
            Some("on_save") => Self::OnSave,
            Some("on_change") => Self::OnChange,
            _ => Self::Manual,
        })
    }
    fn key(project_id: &str) -> String {
        format!("bellatrix.mde.validation_mode.{project_id}")
    }
    fn value(self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::OnSave => "on_save",
            Self::OnChange => "on_change",
        }
    }
}

impl MdeWorkbenchItem {
    pub(super) fn validation_mode_label(&self) -> &'static str {
        match self.validation_mode {
            ValidationMode::Manual => "Validation: manual",
            ValidationMode::OnSave => "Validation: on save",
            ValidationMode::OnChange => "Validation: on change",
        }
    }
    pub(super) fn cycle_validation_mode(&mut self, cx: &mut Context<Self>) {
        self.validation_mode = match self.validation_mode {
            ValidationMode::Manual => ValidationMode::OnSave,
            ValidationMode::OnSave => ValidationMode::OnChange,
            ValidationMode::OnChange => ValidationMode::Manual,
        };
        self.feedback = self.validation_mode_label().to_owned();
        let key = ValidationMode::key(self.project_id().as_str());
        let value = self.validation_mode.value().to_owned();
        cx.spawn(async move |this, cx| {
            let _guard = PREFERENCE_WRITES.lock().await;
            if let Err(error) = db::kvp::GlobalKeyValueStore::global()
                .write_kvp(key, value)
                .await
            {
                if let Err(update_error) = this.update(cx, |item, cx| {
                    item.feedback = format!("Could not save validation preference: {error}");
                    cx.notify();
                }) {
                    eprintln!("Could not report validation preference error: {update_error}");
                }
            }
        })
        .detach();
        cx.notify();
    }
    pub(super) fn validate_after_change(&mut self, cx: &mut Context<Self>) {
        if self.validation_mode == ValidationMode::OnChange {
            self.validate_diagram(cx);
        } else {
            self.validation_status = "Validation is out of date for the current revision".into();
            cx.notify();
        }
    }
    pub(super) fn validate_after_save(&mut self, cx: &mut Context<Self>) {
        let saved = *self.source.saves.borrow();
        if self.validation_mode == ValidationMode::OnSave
            && self
                .source
                .snapshot()
                .ok()
                .is_some_and(|snapshot| Some(snapshot.revision) == saved)
        {
            self.validate_diagram(cx);
        }
    }
    pub(super) fn accept_validation(
        &mut self,
        result: Option<
            Result<
                bellatrix_mde_cordis::DiagramValidationResult,
                bellatrix_mde_cordis::DiagramValidationFailure,
            >,
        >,
        cx: &mut Context<Self>,
    ) {
        let Some(result) = result else {
            return;
        };
        let result = match result {
            Ok(result) => result,
            Err(error) => {
                if self
                    .source
                    .snapshot()
                    .ok()
                    .is_some_and(|snapshot| snapshot.revision == error.revision)
                    && self.active_diagram.as_deref() == Some(error.diagram_id.as_str())
                {
                    self.validation_status = format!(
                        "Validation failed for revision {}: {}",
                        error.revision, error.message
                    );
                    cx.notify();
                }
                return;
            }
        };
        let Ok(snapshot) = self.source.snapshot() else {
            return;
        };
        if result.revision != snapshot.revision
            || self.active_diagram.as_deref() != Some(result.diagram_id.as_str())
        {
            return;
        }
        self.validation_status = if result.diagnostics.is_empty() {
            format!(
                "Revision {}: no issues in implemented checks; full UML conformance is not checked",
                result.revision
            )
        } else {
            let details = result
                .diagnostics
                .iter()
                .map(|diagnostic| {
                    format!(
                        "{:?} {} [{}]: {}",
                        diagnostic.severity,
                        diagnostic.code,
                        diagnostic.subject_id,
                        diagnostic.message
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            format!(
                "Revision {}: {} diagnostic(s)\n{}",
                result.revision,
                result.diagnostics.len(),
                details
            )
        };
        cx.notify();
    }
    pub fn validate_diagram(&mut self, cx: &mut Context<Self>) {
        let Ok(snapshot) = self.source.snapshot() else {
            self.validation_status = "Project snapshot unavailable".into();
            cx.notify();
            return;
        };
        let diagram = self
            .active_diagram
            .as_deref()
            .and_then(|id| {
                snapshot
                    .section
                    .diagrams
                    .iter()
                    .find(|diagram| diagram.id == id)
            })
            .or_else(|| snapshot.section.diagrams.first());
        let Some(diagram) = diagram else {
            self.validation_status = "Project has no diagram to validate".into();
            cx.notify();
            return;
        };
        if let Err(error) =
            bellatrix_mde::StructuralDiagramKind::from_diagram(diagram).ok_or_else(|| {
                format!(
                    "Unsupported diagram presentation: {} {:?}",
                    diagram.uml_di_type, diagram.interaction_kind
                )
            })
        {
            self.validation_status = error;
            cx.notify();
            return;
        }
        if self.source.is_read_only() {
            if let Some(kind) = bellatrix_mde::StructuralDiagramKind::from_diagram(diagram) {
                let model = snapshot
                    .section
                    .to_model(snapshot.project_id.as_str(), &snapshot.name);
                let issues = bellatrix_mde::validate_structural_diagram(&model, &diagram.id, kind);
                self.validation_status = if issues.is_empty() {
                    "Read-only preview: no issues in implemented checks; full UML conformance is not checked".into()
                } else {
                    format!("Read-only preview validation: {issues:?}")
                };
                cx.notify();
            }
            return;
        }
        self.validation_status = format!("Validation pending for revision {}", snapshot.revision);
        if let Err(error) = self.source.submit(MdeUiCommand::ValidateDiagram(
            bellatrix_mde_cordis::ModelingValidateDiagramRequested {
                project_id: self.project_id().clone(),
                diagram_id: diagram.id.clone(),
                expected_revision: snapshot.revision,
            },
        )) {
            self.validation_status = format!("Validation not sent: {error}");
        }
        cx.notify();
    }
}
