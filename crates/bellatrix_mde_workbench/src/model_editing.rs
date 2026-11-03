use super::*;

impl MdeWorkbenchItem {
    pub fn add_member_to_selected(&mut self, kind: ModelingMemberKind, cx: &mut Context<Self>) {
        if self.source.is_read_only() {
            return;
        }
        let Some(owner_id) = self.selected_element.clone() else {
            self.feedback = "Select a classifier first".into();
            cx.notify();
            return;
        };
        let Ok(snapshot) = self.source.snapshot() else {
            self.feedback = "Project snapshot unavailable".into();
            cx.notify();
            return;
        };
        self.submit(
            MdeUiCommand::AddMember(ModelingAddMemberRequested {
                project_id: self.project_id().clone(),
                owner_id,
                kind,
                expected_revision: snapshot.revision,
            }),
            cx,
        );
    }
    pub(crate) fn run_parameter_action(
        &mut self,
        action: ParameterAction,
        project_id: ProjectId,
        element_id: String,
        expected_revision: u64,
        cx: &mut Context<Self>,
    ) {
        if self.source.is_read_only() {
            return;
        }
        if self.project_id() != &project_id || self.selected_element() != Some(element_id.as_str())
        {
            self.feedback = "Parameter action not sent: the active selection changed".into();
            cx.notify();
            return;
        }
        let Ok(snapshot) = self.source.snapshot() else {
            self.feedback = "Project snapshot unavailable".into();
            cx.notify();
            return;
        };
        if snapshot.revision != expected_revision {
            self.feedback = "Parameter action not sent: the model changed; try again".into();
            cx.notify();
            return;
        }
        let Some(element) = snapshot
            .section
            .elements
            .iter()
            .find(|element| element.id == element_id)
        else {
            self.feedback = "Element was removed from the model".into();
            cx.notify();
            return;
        };
        if !quick_edit_view::parameter_actions(&element.kind)
            .iter()
            .any(|(_, available)| *available == action)
        {
            self.feedback = "This element does not support the parameter action".into();
            cx.notify();
            return;
        }
        let command = match action {
            ParameterAction::Add => MdeUiCommand::AddMember(ModelingAddMemberRequested {
                project_id,
                owner_id: element_id,
                kind: ModelingMemberKind::Parameter,
                expected_revision,
            }),
            ParameterAction::Delete => {
                MdeUiCommand::DeleteElement(ModelingDeleteElementRequested {
                    project_id,
                    element_id,
                    expected_revision,
                })
            }
            ParameterAction::Move { up } => {
                MdeUiCommand::MoveParameter(ModelingMoveParameterRequested {
                    project_id,
                    parameter_id: element_id,
                    up,
                    expected_revision,
                })
            }
        };
        self.submit(command, cx);
    }

    pub(crate) fn submit_admitted_inspector_property(
        &mut self,
        project_id: ProjectId,
        element_id: String,
        path: String,
        value: serde_json::Value,
        expected_revision: u64,
        cx: &mut Context<Self>,
    ) -> Result<tokio::sync::watch::Receiver<Option<Result<u64, String>>>, String> {
        if self.source.is_read_only() {
            return Err("This document is not currently editable; draft preserved".into());
        }
        // Admission captured the target while its input was active. A later
        // selection change must not redirect or discard that accepted edit.
        if self.project_id() != &project_id {
            return Err("Inspector project changed; draft preserved".into());
        }
        let (completion, receipt) = tokio::sync::watch::channel(None);
        self.source.submit(MdeUiCommand::SetPropertyTracked {
            request: ModelingSetPropertyRequested {
                project_id,
                element_id,
                path,
                value,
                expected_revision,
            },
            completion,
        })?;
        self.feedback = "Updating inspected property…".into();
        cx.notify();
        Ok(receipt)
    }

    pub fn set_selected_property(
        &mut self,
        path: String,
        value: serde_json::Value,
        cx: &mut Context<Self>,
    ) {
        if self.source.is_read_only() {
            return;
        }
        let Some(element_id) = self.selected_element.clone() else {
            self.feedback = "Select a model element first".into();
            cx.notify();
            return;
        };
        let Ok(snapshot) = self.source.snapshot() else {
            self.feedback = "Project snapshot unavailable".into();
            cx.notify();
            return;
        };
        self.submit(
            MdeUiCommand::SetProperty(ModelingSetPropertyRequested {
                project_id: self.project_id().clone(),
                element_id,
                path,
                value,
                expected_revision: snapshot.revision,
            }),
            cx,
        );
    }
    pub fn delete_selected_from_model(&mut self, cx: &mut Context<Self>) {
        if self.source.is_read_only() {
            return;
        }
        let Some(element_id) = self.selected_element.clone() else {
            self.feedback = "Select an element to delete".into();
            cx.notify();
            return;
        };
        let Ok(snapshot) = self.source.snapshot() else {
            self.feedback = "Project snapshot unavailable".into();
            cx.notify();
            return;
        };
        self.submit(
            MdeUiCommand::DeleteElement(ModelingDeleteElementRequested {
                project_id: self.project_id().clone(),
                element_id,
                expected_revision: snapshot.revision,
            }),
            cx,
        );
        self.selected_element = None;
        self.selected_view = None;
        self.inline_editor = None;
        self.inline_draft = None;
        self.editing_subject = None;
        self.gesture = None;
        cx.notify();
    }
    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        self.gesture = None;
        if let Ok(snapshot) = self.source.snapshot() {
            if self.selected_view.is_some()
                && !snapshot.section.diagrams.iter().any(|diagram| {
                    Some(diagram.id.as_str()) == self.active_diagram.as_deref()
                        && diagram
                            .views
                            .iter()
                            .any(|view| Some(view.id.as_str()) == self.selected_view.as_deref())
                })
            {
                self.selected_view = None;
                self.inline_editor = None;
                self.inline_draft = None;
                self.editing_subject = None;
            }
        }
        cx.notify();
    }
    pub(super) fn submit(&mut self, command: MdeUiCommand, cx: &mut Context<Self>) {
        self.feedback = match self.source.submit(command) {
            Ok(()) => "Command sent to Cordis".into(),
            Err(error) => format!("Command not sent: {error}"),
        };
        cx.notify();
    }
    pub(super) fn click_modeling_tool(
        &mut self,
        tool: ModelingTool,
        x: f64,
        y: f64,
        cx: &mut Context<Self>,
    ) {
        if let ModelingTool::Relation(kind) = tool {
            self.connect_free_message_endpoint(kind, bellatrix_mde::Point { x, y }, cx);
        } else {
            self.place_modeling_tool(tool, x, y, cx);
        }
    }
    pub(super) fn place_modeling_tool(
        &mut self,
        tool: ModelingTool,
        x: f64,
        y: f64,
        cx: &mut Context<Self>,
    ) {
        match tool {
            ModelingTool::Element(kind) => self.create_element(kind, x, y, cx),
            ModelingTool::Relation(_) => self.arm_drag_tool(tool, cx),
            ModelingTool::ProvidedInterface | ModelingTool::RequiredInterface => {
                self.create_interface_view(tool, x, y, cx);
            }
        }
    }
    fn create_interface_view(
        &mut self,
        tool: ModelingTool,
        x: f64,
        y: f64,
        cx: &mut Context<Self>,
    ) {
        if self.source.is_read_only() {
            return;
        }
        let Some(presentation) = tool.resolve_interface_presentation() else {
            return;
        };
        let snapshot = match self.source.snapshot() {
            Ok(snapshot) => snapshot,
            Err(error) => {
                self.feedback = error;
                cx.notify();
                return;
            }
        };
        let Some(diagram) = snapshot
            .section
            .diagrams
            .iter()
            .find(|diagram| Some(diagram.id.as_str()) == self.active_diagram.as_deref())
        else {
            self.feedback = "Select a diagram to place the Interface presentation".into();
            cx.notify();
            return;
        };
        self.submit(
            MdeUiCommand::CreateVisualDraft(
                bellatrix_mde_cordis::ModelingCreateVisualDraftRequested {
                    project_id: self.project_id().clone(),
                    diagram_id: diagram.id.clone(),
                    notation_kind: "uml.interface".into(),
                    label: Some("Interface".into()),
                    geometry: Geometry::Rect {
                        x,
                        y,
                        width: 112.0,
                        height: 68.0,
                    },
                    properties: std::collections::BTreeMap::from([(
                        "interface_presentation".into(),
                        serde_json::json!(presentation),
                    )]),
                    parent_view_id: None,
                    source_view_id: None,
                    target_view_id: None,
                    expected_revision: snapshot.revision,
                },
            ),
            cx,
        );
        self.selected_tool = None;
        cx.notify();
    }

    pub fn create_element(
        &mut self,
        kind: ModelingElementKind,
        x: f64,
        y: f64,
        cx: &mut Context<Self>,
    ) {
        if self.source.is_read_only() {
            return;
        }
        let Ok(snapshot) = self.source.snapshot() else {
            self.feedback = "Project snapshot unavailable".into();
            cx.notify();
            return;
        };
        let diagram_id = self
            .active_diagram
            .as_deref()
            .and_then(|id| {
                snapshot
                    .section
                    .diagrams
                    .iter()
                    .find(|diagram| diagram.id == id)
            })
            .or_else(|| snapshot.section.diagrams.first())
            .map(|diagram| diagram.id.clone());
        let Some(diagram_id) = diagram_id else {
            self.feedback = "Project has no diagram".into();
            cx.notify();
            return;
        };
        let owner_id = creation_owner(kind, self.selected_element.as_deref());
        if self.visual_draft_mode {
            let (width, height) = snapshot
                .section
                .diagrams
                .iter()
                .find(|diagram| diagram.id == diagram_id)
                .map_or_else(
                    || kind.initial_size(),
                    |diagram| kind.initial_size_in_diagram(diagram),
                );
            let mut properties = std::collections::BTreeMap::new();
            if kind == ModelingElementKind::Part {
                properties.insert("aggregation".into(), serde_json::json!("composite"));
            }
            if let Some(pseudostate_kind) = kind.resolve_pseudostate_kind() {
                properties.insert("kind".into(), serde_json::json!(pseudostate_kind));
            }
            self.submit(
                MdeUiCommand::CreateVisualDraft(
                    bellatrix_mde_cordis::ModelingCreateVisualDraftRequested {
                        project_id: self.project_id().clone(),
                        diagram_id,
                        notation_kind: kind.model_kind().into(),
                        label: Some(kind.default_name().into()),
                        geometry: Geometry::Rect {
                            x,
                            y,
                            width,
                            height,
                        },
                        properties,
                        parent_view_id: None,
                        source_view_id: None,
                        target_view_id: None,
                        expected_revision: snapshot.revision,
                    },
                ),
                cx,
            );
            self.selected_tool = None;
            return;
        }
        self.submit(
            MdeUiCommand::CreateElementOrVisualDraft(ModelingCreateElementRequested {
                project_id: self.project_id().clone(),
                diagram_id,
                owner_id,
                kind,
                name: None,
                x: x.max(0.0),
                y: y.max(0.0),
                expected_revision: snapshot.revision,
            }),
            cx,
        );
        self.selected_tool = None;
    }
    pub fn show_element_in_active_diagram(
        &mut self,
        element_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.source.is_read_only() {
            return;
        }
        let Ok(snapshot) = self.source.snapshot() else {
            self.feedback = "Project snapshot unavailable".into();
            cx.notify();
            return;
        };
        let Some(diagram) = self.active_diagram.as_deref().and_then(|id| {
            snapshot
                .section
                .diagrams
                .iter()
                .find(|diagram| diagram.id == id)
        }) else {
            self.feedback = "Select an active diagram before showing a model element".into();
            cx.notify();
            return;
        };
        match element_view_placement(diagram, &element_id) {
            ElementViewPlacement::Existing(view_id) => {
                self.select_view(element_id, view_id, cx);
                self.feedback = "Selected existing view in active diagram".into();
                cx.notify();
            }
            ElementViewPlacement::NewAt(x, y) => {
                self.select_element(element_id.clone(), cx);
                self.submit(
                    MdeUiCommand::PlaceElementView(
                        bellatrix_mde_cordis::ModelingPlaceElementViewRequested {
                            project_id: self.project_id().clone(),
                            diagram_id: diagram.id.clone(),
                            element_id,
                            x,
                            y,
                            expected_revision: snapshot.revision,
                        },
                    ),
                    cx,
                );
            }
        }
        self.focus_handle.focus(window, cx);
    }
}
