//! A typed, lifecycle-owned source for Zed Items and Panels.
use crate::pages::mde_page::MdeSnapshot;
use bellatrix_mde_cordis::MdeSession;
use bellatrix_mde_cordis::MdeUiBridge;
use bellatrix_mde_cordis::MdeUiCommand;
use bellatrix_project::ProjectDocument;
use bellatrix_project::ProjectId;
use bellatrix_project::ProjectService;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use tokio::sync::watch;

#[derive(Clone)]
pub struct MdeUiSource {
    pub projects: Arc<ProjectService>,
    pub project_id: ProjectId,
    pub revisions: watch::Receiver<u64>,
    pub projections: watch::Receiver<Result<Arc<MdeSnapshot>, String>>,
    pub move_previews: Option<bellatrix_mde_cordis::MovePreviewClient>,
    pub feedback: watch::Receiver<String>,
    pub validations: watch::Receiver<
        Option<
            Result<
                bellatrix_mde_cordis::DiagramValidationResult,
                bellatrix_mde_cordis::DiagramValidationFailure,
            >,
        >,
    >,
    pub saves: watch::Receiver<Option<u64>>,
    pub journal: Arc<bellatrix_mde_cordis::ExecutionJournal>,
    session: MdeSession,
    native_blocked: Arc<AtomicBool>,
    presentation_suspended: Arc<AtomicBool>,
}

impl MdeUiSource {
    /// The owner of `bridge` must outlive all Items and Panels.
    pub fn from_bridge(projects: Arc<ProjectService>, bridge: &MdeUiBridge) -> Self {
        Self {
            projects,
            project_id: bridge.project_id().clone(),
            revisions: bridge.changes(),
            projections: bridge.projections(),
            move_previews: Some(bridge.move_previews()),
            feedback: bridge.feedback(),
            validations: bridge.validations(),
            saves: bridge.saves(),
            journal: bridge.execution_journal(),
            session: bridge.session(),
            native_blocked: Arc::new(AtomicBool::new(false)),
            presentation_suspended: Arc::new(AtomicBool::new(false)),
        }
    }

    pub(crate) fn block_native_edits(&self, blocked: bool) {
        self.native_blocked.store(blocked, Ordering::Release);
    }

    pub fn allows_creation(&self) -> bool {
        !self.is_read_only()
    }

    pub fn document(&self) -> Result<Arc<ProjectDocument>, String> {
        self.session.snapshot()
    }

    pub(crate) fn activity(&self) -> watch::Receiver<()> {
        self.session.activity()
    }

    pub fn is_pending(&self) -> bool {
        self.session.is_pending()
    }

    pub fn snapshot_is_current(&self) -> bool {
        self.session.snapshot().is_ok()
    }

    pub fn is_read_only(&self) -> bool {
        self.is_presentation_suspended()
            || self.native_blocked.load(Ordering::Acquire)
            || !self.snapshot_is_current()
    }

    pub(crate) fn for_workspace(&self) -> Self {
        let mut source = self.clone();
        source.presentation_suspended = Arc::new(AtomicBool::new(false));
        source
    }

    pub(crate) fn set_preview_context(&self, preview: Option<String>) -> bool {
        let suspended = preview.is_some();
        self.presentation_suspended
            .swap(suspended, Ordering::AcqRel)
            != suspended
    }

    pub(crate) fn is_presentation_suspended(&self) -> bool {
        self.presentation_suspended.load(Ordering::Acquire)
    }

    pub(crate) fn presentation_snapshot(&self) -> Result<MdeSnapshot, String> {
        let document = self.session.snapshot()?;
        let projection = self.projections.borrow().clone()?;
        if !projection.matches_session(&document) {
            return Err("The project projection belongs to another document session".into());
        }
        // Revision publication precedes asynchronous projection. Keep the last
        // complete frame visible; editing still requires snapshot() below.
        Ok((*projection).clone())
    }

    pub fn snapshot(&self) -> Result<MdeSnapshot, String> {
        let document = self.session.snapshot()?;
        if let Some(previews) = &self.move_previews {
            previews.set_document(Arc::clone(&document))?;
        }
        let projection = self.projections.borrow().clone()?;
        if !projection.matches_document(&document) {
            return Err("Waiting for the current project projection".into());
        }
        Ok((*projection).clone())
    }

    pub(crate) fn preview_move(
        &self,
        snapshot: &MdeSnapshot,
        subject_id: &str,
        parent_id: Option<&str>,
        parent_occurrence: Option<&bellatrix_mde_cordis::ViewOccurrence>,
        diagram: bool,
    ) -> Option<Result<(), String>> {
        if self.is_read_only() || self.is_presentation_suspended() {
            return Some(Err("The active document is not editable".into()));
        }
        let Some(previews) = &self.move_previews else {
            return Some(Err("Move preview is unavailable for this source".into()));
        };
        let document = self.session.snapshot();
        let document = match document {
            Ok(document) => document,
            Err(error) => return Some(Err(error)),
        };
        if !snapshot.matches_document(&document) {
            return Some(Err("The model changed while preparing this move".into()));
        }
        previews.check(bellatrix_mde_cordis::ModelingPreviewMoveRequested {
            document,
            subject_id: subject_id.to_owned(),
            parent_id: parent_id.map(str::to_owned),
            parent_occurrence: parent_occurrence.cloned(),
            diagram,
        })
    }

    pub fn record_interaction(
        &self,
        action: &'static str,
        identity: Option<&str>,
        outcome: bellatrix_mde_cordis::TraceOutcome,
    ) {
        self.journal
            .record_interaction(self.interaction_revision(), action, identity, outcome);
    }

    fn interaction_revision(&self) -> u64 {
        self.session
            .snapshot()
            .map(|document| document.revision())
            .unwrap_or_else(|_| *self.revisions.borrow())
    }

    fn editable_snapshot(&self, expected_revision: u64) -> Result<MdeSnapshot, String> {
        if self.is_read_only() {
            return Err("The active document is not editable".into());
        }
        let snapshot = self.snapshot()?;
        if snapshot.revision != expected_revision {
            return Err("The model changed; try again with the current selection".into());
        }
        Ok(snapshot)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_element(
        &self,
        diagram_id: &str,
        kind: bellatrix_mde_cordis::ModelingElementKind,
        selected_owner: Option<&str>,
        position: (f64, f64),
        visual_draft: bool,
        expected_revision: u64,
    ) -> Result<(), String> {
        let snapshot = self.editable_snapshot(expected_revision)?;
        if !snapshot.section.tools.allows_tool(
            Some(diagram_id),
            bellatrix_mde_cordis::ModelingTool::Element(kind),
        ) {
            return Err("This tool is not available for the target diagram's domain".into());
        }
        let metadata = snapshot
            .section
            .tools
            .element(kind)
            .ok_or("Element tool metadata unavailable")?;
        let request = bellatrix_mde_cordis::ModelingCreateElementRequested {
            project_id: self.project_id.clone(),
            diagram_id: diagram_id.into(),
            owner_id: metadata
                .requires_owner
                .then(|| selected_owner.map(str::to_owned))
                .flatten(),
            kind,
            name: None,
            x: if visual_draft {
                position.0
            } else {
                position.0.max(0.0)
            },
            y: if visual_draft {
                position.1
            } else {
                position.1.max(0.0)
            },
            expected_revision,
        };
        self.submit(if visual_draft {
            MdeUiCommand::CreateElementVisualDraft(request)
        } else {
            MdeUiCommand::CreateElementOrVisualDraft(request)
        })
    }

    pub fn create_interface_presentation(
        &self,
        diagram_id: &str,
        tool: bellatrix_mde_cordis::ModelingTool,
        position: (f64, f64),
        expected_revision: u64,
    ) -> Result<(), String> {
        let snapshot = self.editable_snapshot(expected_revision)?;
        let metadata = snapshot
            .section
            .tools
            .interface_forms
            .iter()
            .find(|metadata| metadata.tool == tool)
            .ok_or("Interface presentation metadata unavailable")?;
        if !snapshot.section.tools.allows_tool(Some(diagram_id), tool) {
            return Err("This tool is not available for the target diagram's domain".into());
        }
        // These tools author notation only. A semantic provided/required
        // Interface also needs its InterfaceRealization/Usage relationship.
        self.create_visual_draft(
            diagram_id,
            "uml.interface",
            Some("Interface".into()),
            bellatrix_mde_cordis::Geometry::Rect {
                x: position.0,
                y: position.1,
                width: 112.0,
                height: 68.0,
            },
            std::collections::BTreeMap::from([(
                "interface_presentation".into(),
                serde_json::json!(metadata.presentation),
            )]),
            None,
            None,
            None,
            expected_revision,
        )
    }

    pub fn create_model_element(
        &self,
        kind: bellatrix_mde_cordis::ModelingElementKind,
        owner_id: Option<&str>,
        expected_revision: u64,
    ) -> Result<(), String> {
        self.editable_snapshot(expected_revision)?;
        self.submit(MdeUiCommand::CreateModelElement(
            bellatrix_mde_cordis::ModelingCreateModelElementRequested {
                project_id: self.project_id.clone(),
                kind,
                name: None,
                owner_id: owner_id.map(str::to_owned),
                expected_revision,
            },
        ))
    }

    pub fn execute_quick_action(
        &self,
        element_id: &str,
        action_id: &str,
        diagram_id: Option<&str>,
        view_id: Option<&str>,
        expected_revision: u64,
    ) -> Result<(), String> {
        use bellatrix_mde_cordis::{
            ModelingMemberKind, QuickEditAvailability, QuickEditControl, UmlQuickCreate,
        };
        let snapshot = self.editable_snapshot(expected_revision)?;
        let element = snapshot
            .section
            .elements
            .iter()
            .find(|element| element.id == element_id)
            .ok_or("Element was removed from the model")?;
        if !element.quick_actions.iter().any(|action| {
            action.id == action_id
                && action.availability == QuickEditAvailability::Implemented
                && action.control == QuickEditControl::Button
        }) {
            return Err("This element does not support the quick action".into());
        }
        match action_id {
            "uml.delete-parameter" => return self.delete_element(element_id, expected_revision),
            "uml.move-parameter-up" | "uml.move-parameter-down" => {
                return self.move_parameter(
                    element_id,
                    action_id == "uml.move-parameter-up",
                    expected_revision,
                );
            }
            _ => {}
        }
        if !self.allows_creation() {
            return Err("Quick creation is not available for this document".into());
        }
        let member = match action_id {
            "uml.add-attribute" => Some(ModelingMemberKind::Attribute),
            "uml.add-operation" => Some(ModelingMemberKind::Operation),
            "uml.add-reception" => Some(ModelingMemberKind::Reception),
            "uml.add-parameter" => Some(ModelingMemberKind::Parameter),
            _ => None,
        };
        if let Some(kind) = member {
            return self.add_member(element_id, kind, expected_revision);
        }
        let action = match action_id {
            "uml.add-note" => UmlQuickCreate::Note,
            "uml.add-constraint" => UmlQuickCreate::Constraint,
            "uml.add-subinterface" | "uml.add-subclass" => UmlQuickCreate::Subtype,
            "uml.add-superinterface" | "uml.add-superclass" => UmlQuickCreate::Supertype,
            "uml.add-realizing-class" => UmlQuickCreate::RealizingClass,
            "uml.add-template-parameter" => UmlQuickCreate::TemplateParameter,
            "uml.add-provided-interface" => UmlQuickCreate::ProvidedInterface,
            "uml.add-required-interface" => UmlQuickCreate::RequiredInterface,
            "uml.add-associated-class" => UmlQuickCreate::AssociatedClass,
            "uml.add-aggregated-class" => UmlQuickCreate::AggregatedClass,
            "uml.add-composited-class" => UmlQuickCreate::CompositedClass,
            "uml.add-port" => UmlQuickCreate::Port,
            "uml.add-part" => UmlQuickCreate::Part,
            _ => return Err("Quick action has no registered command".into()),
        };
        self.quick_create(
            diagram_id.ok_or("Select a diagram for quick creation")?,
            element_id,
            view_id,
            action,
            expected_revision,
        )
    }

    pub fn quick_create(
        &self,
        diagram_id: &str,
        element_id: &str,
        view_id: Option<&str>,
        action: bellatrix_mde_cordis::UmlQuickCreate,
        expected_revision: u64,
    ) -> Result<(), String> {
        let snapshot = self.editable_snapshot(expected_revision)?;
        let element = snapshot
            .section
            .elements
            .iter()
            .find(|element| element.id == element_id)
            .ok_or("Element was removed from the model")?;
        if !element.quick_create.contains(&action) {
            return Err(format!("{} does not support this action", element.kind));
        }
        if !snapshot.section.tools.allows_new_views(diagram_id) {
            return Err("Quick creation is not available for this diagram's domain".into());
        }
        self.submit(MdeUiCommand::QuickEdit(
            bellatrix_mde_cordis::ModelingQuickEditRequested {
                project_id: self.project_id.clone(),
                diagram_id: diagram_id.into(),
                element_id: element_id.into(),
                view_id: view_id.map(str::to_owned),
                action,
                expected_revision,
            },
        ))
    }

    pub fn create_relation(
        &self,
        diagram_id: &str,
        kind: bellatrix_mde_cordis::ModelingRelationKind,
        source_view_id: &str,
        target_view_id: &str,
        expected_revision: u64,
    ) -> Result<(), String> {
        self.connect_relation(
            diagram_id,
            kind,
            (Some(source_view_id), None),
            (Some(target_view_id), None),
            false,
            expected_revision,
        )
    }

    pub(crate) fn accepts_relation_endpoint(
        metadata: &bellatrix_mde_cordis::RelationToolMetadata,
        source: bool,
        has_view: bool,
    ) -> bool {
        let free_endpoint =
            metadata.message_kind.as_deref() == Some(if source { "found" } else { "lost" });
        has_view != free_endpoint
    }

    pub fn connect_relation(
        &self,
        diagram_id: &str,
        kind: bellatrix_mde_cordis::ModelingRelationKind,
        source: (Option<&str>, Option<bellatrix_mde_cordis::Point>),
        target: (Option<&str>, Option<bellatrix_mde_cordis::Point>),
        visual_draft: bool,
        expected_revision: u64,
    ) -> Result<(), String> {
        use bellatrix_mde_cordis::ModelingRelationKind as Kind;
        let snapshot = self.editable_snapshot(expected_revision)?;
        if !snapshot.section.tools.allows_tool(
            Some(diagram_id),
            bellatrix_mde_cordis::ModelingTool::Relation(kind),
        ) {
            return Err("This tool is not available for the target diagram's domain".into());
        }
        let metadata = snapshot
            .section
            .tools
            .relation(kind)
            .ok_or("Relationship tool metadata unavailable")?;
        if !Self::accepts_relation_endpoint(metadata, true, source.0.is_some())
            || !Self::accepts_relation_endpoint(metadata, false, target.0.is_some())
        {
            return Err(metadata.endpoint_instruction.clone());
        }
        let diagram = snapshot
            .section
            .diagrams
            .iter()
            .find(|diagram| diagram.id == diagram_id)
            .ok_or("diagram not found")?;
        let resolve = |id: Option<&str>| -> Result<_, String> {
            id.map(|id| {
                diagram
                    .all_views()
                    .find(|view| view.id == id)
                    .ok_or_else(|| "The selected View no longer exists in this diagram".to_owned())
            })
            .transpose()
        };
        let source_view = resolve(source.0)?;
        let target_view = resolve(target.0)?;
        if !visual_draft && !metadata.is_visual_only {
            let (Some(source_view), Some(target_view)) = (source_view, target_view) else {
                return Err("Semantic relationships require two endpoint Views".into());
            };
            let owner_id = match kind {
                Kind::Generalization
                | Kind::InterfaceRealization
                | Kind::TemplateBinding
                | Kind::Include
                | Kind::Extend
                | Kind::ElementImport
                | Kind::PackageImport
                | Kind::PackageMerge
                | Kind::Manifestation
                | Kind::Deployment => source_view.subject.clone(),
                // A diagram or root is not an explicit choice of semantic owner.
                _ => None,
            };
            return self.submit(MdeUiCommand::CreateRelationAtViewsOrVisualDraft(
                bellatrix_mde_cordis::ModelingCreateRelationAtViewsRequested {
                    project_id: self.project_id.clone(),
                    diagram_id: diagram_id.into(),
                    owner_id,
                    kind,
                    source_view_id: source_view.id.clone(),
                    target_view_id: target_view.id.clone(),
                    from: source_view.subject.clone(),
                    to: target_view.subject.clone(),
                    expected_revision,
                },
            ));
        }
        let locate = |view: Option<&bellatrix_mde_cordis::projection::View>, point| match view {
            Some(view) => {
                crate::components::visual_draft_canvas::resolve_visual_center(&view.geometry)
            }
            None => point,
        };
        let (Some(start), Some(to)) =
            (locate(source_view, source.1), locate(target_view, target.1))
        else {
            return Err("The selected endpoints do not have usable connection geometry".into());
        };
        let mut properties = std::collections::BTreeMap::new();
        if metadata.uses_occurrence_anchors
            && diagram.interaction_kind.as_deref() != Some("communication")
        {
            for (key, (view_id, point)) in [("source_anchor", source), ("target_anchor", target)] {
                if let Some(view_id) = view_id
                    && let Some(anchor) = point.and_then(|point| {
                        crate::components::visual_draft_canvas::resolve_message_anchor(
                            &snapshot.section,
                            diagram,
                            view_id,
                            point,
                        )
                    })
                {
                    properties.insert(key.into(), serde_json::json!({"x":anchor.x,"y":anchor.y}));
                }
            }
        }
        if let Some(message_kind) = &metadata.message_kind {
            properties.insert("messageKind".into(), serde_json::json!(message_kind));
        }
        let label = metadata.message_sort.as_ref().map(|sort| {
            properties.insert("messageSort".into(), serde_json::json!(sort));
            if diagram.interaction_kind.as_deref() == Some("communication") {
                "1: message()".to_owned()
            } else {
                "message()".to_owned()
            }
        });
        self.create_visual_draft(
            diagram_id,
            &metadata.model_kind,
            label,
            bellatrix_mde_cordis::Geometry::Path {
                start,
                segments: vec![bellatrix_mde_cordis::PathSegment::Line { to }],
            },
            properties,
            None,
            source.0.map(str::to_owned),
            target.0.map(str::to_owned),
            expected_revision,
        )
    }

    pub fn add_member(
        &self,
        owner_id: &str,
        kind: bellatrix_mde_cordis::ModelingMemberKind,
        expected_revision: u64,
    ) -> Result<(), String> {
        self.editable_snapshot(expected_revision)?;
        self.submit(MdeUiCommand::AddMember(
            bellatrix_mde_cordis::ModelingAddMemberRequested {
                project_id: self.project_id.clone(),
                owner_id: owner_id.into(),
                kind,
                expected_revision,
            },
        ))
    }

    pub fn delete_element(&self, element_id: &str, expected_revision: u64) -> Result<(), String> {
        let snapshot = self.editable_snapshot(expected_revision)?;
        let context_diagrams = snapshot
            .section
            .diagrams
            .iter()
            .filter(|diagram| diagram.model_elements.iter().any(|id| id == element_id))
            .map(|diagram| diagram.name.as_str())
            .collect::<Vec<_>>();
        if !context_diagrams.is_empty() {
            return Err(format!(
                "Cannot delete this model element: it is the model context of {}. Remove those diagrams or change their model context first.",
                context_diagrams.join(", ")
            ));
        }
        self.submit(MdeUiCommand::DeleteElement(
            bellatrix_mde_cordis::ModelingDeleteElementRequested {
                project_id: self.project_id.clone(),
                element_id: element_id.into(),
                expected_revision,
            },
        ))
    }

    pub fn move_parameter(
        &self,
        parameter_id: &str,
        up: bool,
        expected_revision: u64,
    ) -> Result<(), String> {
        self.editable_snapshot(expected_revision)?;
        self.submit(MdeUiCommand::MoveParameter(
            bellatrix_mde_cordis::ModelingMoveParameterRequested {
                project_id: self.project_id.clone(),
                parameter_id: parameter_id.into(),
                up,
                expected_revision,
            },
        ))
    }

    pub fn set_property(
        &self,
        element_id: &str,
        path: &str,
        value: serde_json::Value,
        expected_revision: u64,
    ) -> Result<(), String> {
        self.editable_snapshot(expected_revision)?;
        self.submit(MdeUiCommand::SetProperty(
            bellatrix_mde_cordis::ModelingSetPropertyRequested {
                project_id: self.project_id.clone(),
                element_id: element_id.into(),
                path: path.into(),
                value,
                expected_revision,
            },
        ))
    }

    pub fn move_view(
        &self,
        view_id: &str,
        dx: f64,
        dy: f64,
        expected_revision: u64,
    ) -> Result<(), String> {
        self.editable_snapshot(expected_revision)?;
        self.submit(MdeUiCommand::MoveView(
            bellatrix_mde_cordis::ModelingMoveViewRequested {
                project_id: self.project_id.clone(),
                view_id: view_id.into(),
                dx,
                dy,
                expected_revision,
            },
        ))
    }

    pub fn resize_view(
        &self,
        view_id: &str,
        width: f64,
        height: f64,
        expected_revision: u64,
    ) -> Result<(), String> {
        self.editable_snapshot(expected_revision)?;
        self.submit(MdeUiCommand::ResizeView(
            bellatrix_mde_cordis::ModelingResizeViewRequested {
                project_id: self.project_id.clone(),
                view_id: view_id.into(),
                width,
                height,
                expected_revision,
            },
        ))
    }

    pub fn set_view_path(
        &self,
        diagram_id: &str,
        view_id: &str,
        geometry: bellatrix_mde_cordis::Geometry,
        expected_revision: u64,
    ) -> Result<(), String> {
        self.editable_snapshot(expected_revision)?;
        self.submit(MdeUiCommand::SetViewPath(
            bellatrix_mde_cordis::ModelingSetViewPathRequested {
                project_id: self.project_id.clone(),
                diagram_id: diagram_id.into(),
                view_id: view_id.into(),
                geometry,
                expected_revision,
            },
        ))
    }

    pub fn remove_view(
        &self,
        diagram_id: &str,
        view_id: &str,
        expected_revision: u64,
    ) -> Result<(), String> {
        self.editable_snapshot(expected_revision)?;
        self.submit(MdeUiCommand::RemoveView(
            bellatrix_mde_cordis::ModelingRemoveViewRequested {
                project_id: self.project_id.clone(),
                diagram_id: diagram_id.into(),
                view_id: view_id.into(),
                expected_revision,
            },
        ))
    }

    pub fn place_element_view(
        &self,
        diagram_id: &str,
        element_id: &str,
        x: f64,
        y: f64,
        expected_revision: u64,
    ) -> Result<(), String> {
        self.editable_snapshot(expected_revision)?;
        self.submit(MdeUiCommand::PlaceElementView(
            bellatrix_mde_cordis::ModelingPlaceElementViewRequested {
                project_id: self.project_id.clone(),
                diagram_id: diagram_id.into(),
                element_id: element_id.into(),
                x,
                y,
                expected_revision,
            },
        ))
    }

    pub fn rename_element(
        &self,
        element_id: &str,
        new_name: String,
        expected_revision: u64,
    ) -> Result<(), String> {
        self.editable_snapshot(expected_revision)?;
        self.submit(MdeUiCommand::Rename(
            bellatrix_mde_cordis::ModelingRenameRequested {
                project_id: self.project_id.clone(),
                element_id: element_id.into(),
                new_name,
                expected_revision,
            },
        ))
    }

    pub fn rename_diagram(
        &self,
        diagram_id: &str,
        name: String,
        expected_revision: u64,
    ) -> Result<(), String> {
        self.editable_snapshot(expected_revision)?;
        self.submit(MdeUiCommand::RenameDiagram(
            bellatrix_mde_cordis::ModelingRenameDiagramRequested {
                project_id: self.project_id.clone(),
                diagram_id: diagram_id.into(),
                name,
                expected_revision,
            },
        ))
    }

    pub fn delete_diagram(&self, diagram_id: &str, expected_revision: u64) -> Result<(), String> {
        self.editable_snapshot(expected_revision)?;
        self.submit(MdeUiCommand::DeleteDiagram(
            bellatrix_mde_cordis::ModelingDeleteDiagramRequested {
                project_id: self.project_id.clone(),
                diagram_id: diagram_id.into(),
                expected_revision,
            },
        ))
    }

    pub fn set_name_expression(
        &self,
        element_id: &str,
        expression: String,
        expected_revision: u64,
    ) -> Result<(), String> {
        self.editable_snapshot(expected_revision)?;
        self.submit(MdeUiCommand::SetNameExpression(
            bellatrix_mde_cordis::ModelingSetNameExpressionRequested {
                project_id: self.project_id.clone(),
                element_id: element_id.into(),
                expression,
                expected_revision,
            },
        ))
    }

    pub fn set_owner(
        &self,
        element_id: &str,
        owner_id: Option<String>,
        expected_revision: u64,
    ) -> Result<(), String> {
        self.editable_snapshot(expected_revision)?;
        self.submit(MdeUiCommand::SetOwner(
            bellatrix_mde_cordis::ModelingSetOwnerRequested {
                project_id: self.project_id.clone(),
                element_id: element_id.into(),
                owner_id,
                expected_revision,
            },
        ))
    }

    pub fn set_explorer_parent(
        &self,
        subject_id: &str,
        parent_id: Option<String>,
        parent_occurrence: Option<bellatrix_mde_cordis::ViewOccurrence>,
        expected_revision: u64,
    ) -> Result<(), String> {
        self.editable_snapshot(expected_revision)?;
        self.submit(MdeUiCommand::SetExplorerParent(
            bellatrix_mde_cordis::ModelingSetExplorerParentRequested {
                project_id: self.project_id.clone(),
                subject_id: subject_id.into(),
                parent_id,
                parent_occurrence,
                expected_revision,
            },
        ))
    }

    fn diagram_creation_request(
        &self,
        kind: bellatrix_mde_cordis::StructuralDiagramKind,
        selected: Option<&str>,
        expected_revision: u64,
    ) -> Result<bellatrix_mde_cordis::ModelingCreateDiagramRequested, String> {
        let snapshot = self.editable_snapshot(expected_revision)?;
        let uses_context = kind == bellatrix_mde_cordis::StructuralDiagramKind::CompositeStructure
            || snapshot
                .section
                .tools
                .diagram(kind)
                .is_some_and(|metadata| metadata.context_metaclass.is_some());
        Ok(bellatrix_mde_cordis::ModelingCreateDiagramRequested {
            project_id: self.project_id.clone(),
            expected_revision,
            kind,
            name: format!(
                "{} diagram {}",
                crate::pages::mde_page::diagram_kind_label(kind),
                snapshot.section.diagrams.len().saturating_add(1)
            ),
            model_elements: if uses_context {
                selected.map(str::to_owned).into_iter().collect()
            } else {
                Vec::new()
            },
        })
    }

    pub fn create_diagram(
        &self,
        kind: bellatrix_mde_cordis::StructuralDiagramKind,
        selected: Option<&str>,
        expected_revision: u64,
    ) -> Result<(), String> {
        let request = self.diagram_creation_request(kind, selected, expected_revision)?;
        self.submit(MdeUiCommand::CreateDiagram(request))
    }

    pub fn create_explorer_diagram(
        &self,
        kind: bellatrix_mde_cordis::StructuralDiagramKind,
        parent_id: Option<&str>,
        parent_occurrence: Option<bellatrix_mde_cordis::ViewOccurrence>,
        expected_revision: u64,
    ) -> Result<(), String> {
        let diagram = self.diagram_creation_request(kind, parent_id, expected_revision)?;
        self.submit(MdeUiCommand::CreateExplorerDiagram(
            bellatrix_mde_cordis::ModelingCreateExplorerDiagramRequested {
                diagram,
                parent_id: parent_id.map(str::to_owned),
                parent_occurrence,
            },
        ))
    }

    pub fn validate_diagram(&self, diagram_id: &str, expected_revision: u64) -> Result<(), String> {
        self.submit(MdeUiCommand::ValidateDiagram(
            bellatrix_mde_cordis::ModelingValidateDiagramRequested {
                project_id: self.project_id.clone(),
                diagram_id: diagram_id.into(),
                expected_revision,
            },
        ))
    }

    pub fn save_project(&self, path: std::path::PathBuf) -> Result<(), String> {
        let document = self.document()?;
        self.submit(MdeUiCommand::SaveProject(
            bellatrix_project::ProjectSaveRequested { document, path },
        ))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_visual_draft(
        &self,
        diagram_id: &str,
        notation_kind: &str,
        label: Option<String>,
        geometry: bellatrix_mde_cordis::Geometry,
        properties: std::collections::BTreeMap<String, serde_json::Value>,
        parent_view_id: Option<String>,
        source_view_id: Option<String>,
        target_view_id: Option<String>,
        expected_revision: u64,
    ) -> Result<(), String> {
        self.editable_snapshot(expected_revision)?;
        self.submit(MdeUiCommand::CreateVisualDraft(
            bellatrix_mde_cordis::ModelingCreateVisualDraftRequested {
                project_id: self.project_id.clone(),
                diagram_id: diagram_id.into(),
                notation_kind: notation_kind.into(),
                label,
                geometry,
                properties,
                parent_view_id,
                source_view_id,
                target_view_id,
                expected_revision,
            },
        ))
    }

    pub fn set_property_tracked(
        &self,
        element_id: &str,
        path: &str,
        value: serde_json::Value,
        expected_revision: u64,
        completion: watch::Sender<Option<Result<u64, String>>>,
    ) -> Result<(), String> {
        self.submit(MdeUiCommand::SetPropertyTracked {
            request: bellatrix_mde_cordis::ModelingSetPropertyRequested {
                project_id: self.project_id.clone(),
                element_id: element_id.into(),
                path: path.into(),
                value,
                expected_revision,
            },
            completion,
        })
    }

    pub fn set_view_label_tracked(
        &self,
        view_id: &str,
        label: Option<String>,
        expected_revision: u64,
        completion: watch::Sender<Option<Result<u64, String>>>,
    ) -> Result<(), String> {
        self.submit(MdeUiCommand::SetViewLabelTracked {
            request: bellatrix_mde_cordis::ModelingSetViewLabelRequested {
                project_id: self.project_id.clone(),
                view_id: view_id.into(),
                label,
                expected_revision,
            },
            completion,
        })
    }

    pub fn set_view_timing_tracked(
        &self,
        view_id: &str,
        timing: Option<bellatrix_mde_cordis::TimingPresentation>,
        expected_revision: u64,
        completion: watch::Sender<Option<Result<u64, String>>>,
    ) -> Result<(), String> {
        self.submit(MdeUiCommand::SetViewTimingTracked {
            request: bellatrix_mde_cordis::ModelingSetViewTimingRequested {
                project_id: self.project_id.clone(),
                view_id: view_id.into(),
                timing,
                expected_revision,
            },
            completion,
        })
    }

    fn admit_view_style_edit(
        &self,
        diagram_id: &str,
        view_id: &str,
        layout: bool,
    ) -> Result<(), String> {
        let snapshot = self.snapshot()?;
        let diagram = snapshot
            .section
            .diagrams
            .iter()
            .find(|diagram| diagram.id == diagram_id)
            .ok_or("The original diagram is no longer available")?;
        if !crate::view_model::selection::supports_view_style_editing(diagram, view_id) {
            return Err("This view supports inspection only".into());
        }
        if layout && !crate::view_model::selection::supports_view_layout_editing(diagram, view_id) {
            return Err("Routing and stacking changes require a supported editable view".into());
        }
        Ok(())
    }

    pub fn set_view_style_tracked(
        &self,
        diagram_id: &str,
        view_id: &str,
        style: bellatrix_mde_cordis::Style,
        expected_revision: u64,
        completion: watch::Sender<Option<Result<u64, String>>>,
    ) -> Result<(), String> {
        self.admit_view_style_edit(diagram_id, view_id, false)?;
        self.submit(MdeUiCommand::SetViewStyleTracked {
            request: bellatrix_mde_cordis::ModelingSetViewStyleRequested {
                project_id: self.project_id.clone(),
                diagram_id: diagram_id.into(),
                view_id: view_id.into(),
                style,
                expected_revision,
            },
            completion,
        })
    }

    pub fn set_view_routing_tracked(
        &self,
        diagram_id: &str,
        view_id: &str,
        routing: bellatrix_mde_cordis::RelationshipRouting,
        expected_revision: u64,
        completion: watch::Sender<Option<Result<u64, String>>>,
    ) -> Result<(), String> {
        self.admit_view_style_edit(diagram_id, view_id, true)?;
        self.submit(MdeUiCommand::SetViewRoutingTracked {
            request: bellatrix_mde_cordis::ModelingSetViewRoutingRequested {
                project_id: self.project_id.clone(),
                diagram_id: diagram_id.into(),
                view_id: view_id.into(),
                routing,
                expected_revision,
            },
            completion,
        })
    }

    pub fn reorder_view_tracked(
        &self,
        diagram_id: &str,
        view_id: &str,
        order: bellatrix_mde_cordis::ViewOrder,
        expected_revision: u64,
        completion: watch::Sender<Option<Result<u64, String>>>,
    ) -> Result<(), String> {
        self.admit_view_style_edit(diagram_id, view_id, true)?;
        self.submit(MdeUiCommand::ReorderViewTracked {
            request: bellatrix_mde_cordis::ModelingReorderViewRequested {
                project_id: self.project_id.clone(),
                diagram_id: diagram_id.into(),
                view_id: view_id.into(),
                order,
                expected_revision,
            },
            completion,
        })
    }

    pub fn submit(&self, command: MdeUiCommand) -> Result<(), String> {
        let revision = self.interaction_revision();
        let identity = match &command {
            MdeUiCommand::MoveView(request) => Some(request.view_id.as_str()),
            MdeUiCommand::ResizeView(request) => Some(request.view_id.as_str()),
            MdeUiCommand::SetViewPath(request)
            | MdeUiCommand::SetViewPathTracked { request, .. } => Some(request.view_id.as_str()),
            _ => None,
        };
        let action = match &command {
            MdeUiCommand::SetViewLabelTracked { .. } => "command.enqueue.set_view_label_tracked",
            MdeUiCommand::SetViewTimingTracked { .. } => "command.enqueue.set_view_timing_tracked",
            MdeUiCommand::SetViewStyleTracked { .. } => "command.enqueue.set_view_style_tracked",
            MdeUiCommand::SetViewRoutingTracked { .. } => {
                "command.enqueue.set_view_routing_tracked"
            }
            MdeUiCommand::SetViewPathTracked { .. } => "command.enqueue.set_view_path_tracked",
            MdeUiCommand::ReorderViewTracked { .. } => "command.enqueue.reorder_view_tracked",
            MdeUiCommand::SetPropertyTracked { .. } => "command.enqueue.set_property_tracked",
            MdeUiCommand::SetViewLabel(..) => "command.enqueue.set_view_label",
            MdeUiCommand::SetViewStyle(..) => "command.enqueue.set_view_style",
            MdeUiCommand::SetViewRouting(..) => "command.enqueue.set_view_routing",
            MdeUiCommand::SetViewPath(..) => "command.enqueue.set_view_path",
            MdeUiCommand::CreateDiagram(..) => "command.enqueue.create_diagram",
            MdeUiCommand::CreateExplorerDiagram(..) => "command.enqueue.create_explorer_diagram",
            MdeUiCommand::RenameDiagram(..) => "command.enqueue.rename_diagram",
            MdeUiCommand::DeleteDiagram(..) => "command.enqueue.delete_diagram",
            MdeUiCommand::ValidateDiagram(..) => "command.enqueue.validate_diagram",
            MdeUiCommand::CreateVisualDraft(..) => "command.enqueue.create_visual_draft",
            MdeUiCommand::CreateElementOrVisualDraft(..) => {
                "command.enqueue.create_element_or_visual_draft"
            }
            MdeUiCommand::CreateElement(..) => "command.enqueue.create_element",
            MdeUiCommand::CreateElementVisualDraft(..) => {
                "command.enqueue.create_element_visual_draft"
            }
            MdeUiCommand::CreateModelElement(..) => "command.enqueue.create_model_element",
            MdeUiCommand::CreateRelationOrVisualDraft(..) => {
                "command.enqueue.create_relation_or_visual_draft"
            }
            MdeUiCommand::CreateRelationAtViewsOrVisualDraft(..) => {
                "command.enqueue.create_relation_at_views_or_visual_draft"
            }
            MdeUiCommand::CreateRelation(..) => "command.enqueue.create_relation",
            MdeUiCommand::MoveView(..) => "command.enqueue.move_view",
            MdeUiCommand::ResizeView(..) => "command.enqueue.resize_view",
            MdeUiCommand::RemoveView(..) => "command.enqueue.remove_view",
            MdeUiCommand::PlaceElementView(..) => "command.enqueue.place_element_view",
            MdeUiCommand::Rename(..) => "command.enqueue.rename",
            MdeUiCommand::SetOwner(..) => "command.enqueue.set_owner",
            MdeUiCommand::SetExplorerParent(..) => "command.enqueue.set_explorer_parent",
            MdeUiCommand::DeleteElement(..) => "command.enqueue.delete_element",
            MdeUiCommand::SetProperty(..) => "command.enqueue.set_property",
            MdeUiCommand::AddMember(..) => "command.enqueue.add_member",
            MdeUiCommand::MoveParameter(..) => "command.enqueue.move_parameter",
            MdeUiCommand::QuickEdit(..) => "command.enqueue.quick_edit",
            MdeUiCommand::SetNameExpression(..) => "command.enqueue.set_name_expression",
            MdeUiCommand::SaveProject(..) => "command.enqueue.save_project",
        };

        if self.is_presentation_suspended() {
            self.journal.record_interaction(
                revision,
                action,
                identity,
                bellatrix_mde_cordis::TraceOutcome::Invalid,
            );
            return Err("This Workbench is not the active MDE document. Select its MDE tab before editing or saving its model.".into());
        }
        if self.is_read_only() && !matches!(&command, MdeUiCommand::ValidateDiagram(_)) {
            self.journal.record_interaction(
                revision,
                action,
                identity,
                bellatrix_mde_cordis::TraceOutcome::Invalid,
            );
            return Err("The document is temporarily unavailable for editing; finish its pending write or reopen the native document if its session closed".into());
        }
        let identity = identity.map(str::to_owned);
        let result = self.session.submit(command);
        self.journal.record_interaction(
            revision,
            action,
            identity.as_deref(),
            if result.is_ok() {
                bellatrix_mde_cordis::TraceOutcome::Success
            } else {
                bellatrix_mde_cordis::TraceOutcome::Error
            },
        );
        result
    }
}
