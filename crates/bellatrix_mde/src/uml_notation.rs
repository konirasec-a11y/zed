use crate::UmlType;

/// Supported presentation strategies, not UML metaclass inheritance.
/// Unknown and deferred kinds must not acquire a class rectangle by fallback.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UmlNotation {
    ClassifierRectangle,
    PackageTab,
    FoldedNote,
    Frame,
    AssociationDiamond,
    Port,
    Part,
    Actor,
    UseCase,
    DeploymentNode,
    Collaboration,
    Instance,
    Action,
    State,
    InitialNode,
    FinalNode,
    FlowFinal,
    Decision,
    Fork,
    Lifeline,
    Execution,
    InteractionUse,
    CombinedFragment,
    InteractionOperand,
    ObjectNode,
    SendSignal,
    AcceptEvent,
    Partition,
    TimingState,
    History,
    Terminate,
    Unsupported,
}

impl UmlNotation {
    pub fn minimum_size(self) -> (f64, f64) {
        match self {
            Self::InitialNode
            | Self::FinalNode
            | Self::FlowFinal
            | Self::History
            | Self::Terminate => (20.0, 20.0),
            Self::Fork => (20.0, 8.0),
            Self::Execution => (12.0, 24.0),
            Self::Port => (12.0, 12.0),
            Self::Decision => (32.0, 32.0),
            _ => (80.0, 48.0),
        }
    }
}

pub fn resolve_pseudostate_notation(kind: &str) -> UmlNotation {
    match kind {
        "initial" | "junction" => UmlNotation::InitialNode,
        "choice" => UmlNotation::Decision,
        "shallowHistory" | "deepHistory" | "entryPoint" => UmlNotation::History,
        "exitPoint" => UmlNotation::FlowFinal,
        "terminate" => UmlNotation::Terminate,
        "fork" | "join" => UmlNotation::Fork,
        _ => UmlNotation::Unsupported,
    }
}

pub fn resolve_element_notation(element: &crate::Element) -> UmlNotation {
    if element.kind == "uml.pseudostate" {
        resolve_pseudostate_notation(
            element
                .properties
                .get("kind")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("initial"),
        )
    } else {
        uml_notation(&element.kind)
    }
}

pub fn uml_notation(kind: &str) -> UmlNotation {
    // These are presentation strategies, not executable behavior adapters.
    match kind {
        "uml.deployment_specification" => return UmlNotation::ClassifierRectangle,
        "uml.collaboration_use" => return UmlNotation::Collaboration,
        "uml.destruction_occurrence_specification" => return UmlNotation::Terminate,
        "uml.continuation" => return UmlNotation::State,
        "uml.actor" => return UmlNotation::Actor,
        "uml.use_case" => return UmlNotation::UseCase,
        "uml.instance_specification" => return UmlNotation::Instance,
        "uml.activity"
        | "uml.opaque_action"
        | "uml.call_behavior_action"
        | "uml.call_operation_action"
        | "uml.structured_activity_node" => return UmlNotation::Action,
        "uml.state" | "uml.state_machine" | "uml.state_invariant" => return UmlNotation::State,
        "uml.pseudostate" | "uml.initial_node" => return UmlNotation::InitialNode,
        "uml.activity_final_node" | "uml.final_state" => return UmlNotation::FinalNode,
        "uml.flow_final_node" => return UmlNotation::FlowFinal,
        "uml.decision_node" | "uml.merge_node" => return UmlNotation::Decision,
        "uml.fork_node" | "uml.join_node" => return UmlNotation::Fork,
        "uml.lifeline" => return UmlNotation::Lifeline,
        "uml.behavior_execution_specification" | "uml.action_execution_specification" => {
            return UmlNotation::Execution;
        }
        "uml.interaction" | "uml.interaction_use" => return UmlNotation::InteractionUse,
        "uml.combined_fragment" => return UmlNotation::CombinedFragment,
        "uml.interaction_operand" => return UmlNotation::InteractionOperand,
        "uml.input_pin" | "uml.output_pin" | "uml.value_pin" => return UmlNotation::Port,
        "uml.central_buffer_node" | "uml.data_store_node" | "uml.activity_parameter_node" => {
            return UmlNotation::ObjectNode;
        }
        "uml.send_signal_action" => return UmlNotation::SendSignal,
        "uml.accept_event_action" => return UmlNotation::AcceptEvent,
        "uml.activity_partition" | "uml.region" => return UmlNotation::Partition,
        _ => {}
    }
    match UmlType::from_kind(kind) {
        Some(
            UmlType::Class
            | UmlType::Interface
            | UmlType::Signal
            | UmlType::DataType
            | UmlType::PrimitiveType
            | UmlType::Enumeration
            | UmlType::AssociationClass
            | UmlType::Template
            | UmlType::Component
            | UmlType::Artifact
            | UmlType::Stereotype,
        ) => UmlNotation::ClassifierRectangle,
        Some(UmlType::Package | UmlType::Profile | UmlType::Model | UmlType::Subsystem) => {
            UmlNotation::PackageTab
        }
        Some(UmlType::Note | UmlType::Constraint) => UmlNotation::FoldedNote,
        Some(UmlType::Frame) => UmlNotation::Frame,
        Some(UmlType::NaryAssociation) => UmlNotation::AssociationDiamond,
        Some(UmlType::Port) => UmlNotation::Port,
        Some(UmlType::Part) => UmlNotation::Part,
        Some(UmlType::Node | UmlType::Device | UmlType::ExecutionEnvironment) => {
            UmlNotation::DeploymentNode
        }
        Some(UmlType::Collaboration) => UmlNotation::Collaboration,
        _ => UmlNotation::Unsupported,
    }
}

/// Display-ready labels; application text is not proof of a UML ProfileApplication.
/// A semantic keyword must survive independently of the user's application label.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UmlHeader {
    pub keyword: Option<&'static str>,
    pub applied_stereotype: Option<String>,
}

impl UmlHeader {
    /// Logical lines including the title. Renderers still need wrapping and
    /// minimum height rather than a fixed height for long user-provided labels.
    pub fn line_count(&self) -> usize {
        1 + usize::from(self.keyword.is_some()) + usize::from(self.applied_stereotype.is_some())
    }

    pub fn classifier_header_height(&self) -> f64 {
        34.0 + 15.0 * (self.line_count() - 1) as f64
    }

    /// Shared unscaled minimum for diagram display, resize and edge anchors.
    pub fn minimum_height(&self, notation: UmlNotation) -> f64 {
        let labels = (self.line_count() - 1) as f64;
        match notation {
            UmlNotation::ClassifierRectangle => self.classifier_header_height().max(48.0),
            UmlNotation::PackageTab => (45.0 + 15.0 * labels).max(48.0),
            _ => notation.minimum_size().1,
        }
    }
}

pub fn uml_header(kind: &str, applied: Option<&str>) -> UmlHeader {
    let keyword = match UmlType::from_kind(kind) {
        // §10.4.4 permits expanded Interfaces with dependency arrows.
        Some(UmlType::Interface) => Some("«interface»"),
        // OMG UML 2.5.1 §§11.6.4, 19.3.4 and 12.3.4 permit these lightweight
        // presentations without replacing their semantic kinds with Class.
        Some(UmlType::Component) => Some("«component»"),
        Some(UmlType::Artifact) => Some("«artifact»"),
        Some(UmlType::Device) => Some("«device»"),
        Some(UmlType::ExecutionEnvironment) => Some("«executionEnvironment»"),
        Some(UmlType::Stereotype) => Some("«stereotype»"),
        Some(UmlType::Profile) => Some("«profile»"),
        // Preserve the existing editor's labels; these aliases are not a claim
        // that every legacy presentation implements a complete UML notation.
        Some(UmlType::Enumeration) => Some("«enumeration»"),
        Some(UmlType::DataType) => Some("«dataType»"),
        Some(UmlType::PrimitiveType) => Some("«primitive»"),
        Some(UmlType::Signal) => Some("«signal»"),
        // §11.5.4 uses an ordinary classifier box attached to the association.
        Some(UmlType::AssociationClass) => None,
        Some(UmlType::Template) => Some("«template»"),
        Some(UmlType::Model) => Some("«model»"),
        Some(UmlType::Subsystem) => Some("«subsystem»"),
        _ => match kind {
            "uml.deployment_specification" => Some("«deployment spec»"),
            _ => None,
        },
    };
    UmlHeader {
        keyword,
        applied_stereotype: applied.and_then(format_applied_stereotype),
    }
}

fn format_applied_stereotype(value: &str) -> Option<String> {
    let mut label = value.trim();
    loop {
        let unwrapped = label
            .strip_prefix('«')
            .and_then(|inner| inner.strip_suffix('»'))
            .or_else(|| {
                label
                    .strip_prefix("<<")
                    .and_then(|inner| inner.strip_suffix(">>"))
            });
        match unwrapped {
            Some(inner) => label = inner.trim(),
            None => break,
        }
    }
    (!label.is_empty()).then(|| format!("«{label}»"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mandatory_keywords_survive_applied_labels() {
        for (kind, keyword) in [
            ("uml.interface", "«interface»"),
            ("rust.trait", "«interface»"),
            ("uml.component", "«component»"),
            ("uml.artifact", "«artifact»"),
            ("uml.profile", "«profile»"),
            ("uml.stereotype", "«stereotype»"),
        ] {
            let header = uml_header(kind, Some("Domain"));
            assert_eq!(header.keyword, Some(keyword));
            assert_eq!(header.applied_stereotype.as_deref(), Some("«Domain»"));
            assert_eq!(header.line_count(), 3);
            assert_eq!(uml_header(kind, None).keyword, header.keyword);
        }
    }

    #[test]
    fn application_formatting_does_not_duplicate_guillemets() {
        for value in [
            "Domain",
            "  Domain  ",
            "«Domain»",
            "<<Domain>>",
            " « <<Domain>> » ",
        ] {
            assert_eq!(
                uml_header("uml.class", Some(value))
                    .applied_stereotype
                    .as_deref(),
                Some("«Domain»")
            );
        }
        for value in ["", "  ", "« »", "<<>>"] {
            assert_eq!(
                uml_header("uml.class", Some(value)).applied_stereotype,
                None
            );
        }
        assert_eq!(
            uml_header("uml.class", Some("First, Second"))
                .applied_stereotype
                .as_deref(),
            Some("«First, Second»")
        );
        assert_eq!(uml_header("uml.class", None).line_count(), 1);
    }

    #[test]
    fn notation_support_is_explicit_not_classifier_inheritance() {
        for kind in ["uml.unknown", "fs.file", "uml.operation"] {
            assert_eq!(uml_notation(kind), UmlNotation::Unsupported, "{kind}");
        }
        for kind in ["uml.component", "uml.artifact", "uml.stereotype"] {
            assert_eq!(uml_notation(kind), UmlNotation::ClassifierRectangle);
        }
        assert_eq!(uml_notation("uml.profile"), UmlNotation::PackageTab);
    }

    #[test]
    fn existing_notations_and_compatibility_aliases_are_preserved() {
        for kind in [
            "uml.class",
            "rust.struct",
            "uml.signal",
            "uml.data_type",
            "uml.primitive_type",
            "uml.enumeration",
            "uml.association_class",
            "uml.template",
        ] {
            assert_eq!(
                uml_notation(kind),
                UmlNotation::ClassifierRectangle,
                "{kind}"
            );
        }
        for kind in ["uml.interface", "rust.trait"] {
            assert_eq!(uml_notation(kind), UmlNotation::ClassifierRectangle);
        }
        for kind in ["uml.package", "uml.model", "uml.subsystem"] {
            assert_eq!(uml_notation(kind), UmlNotation::PackageTab);
        }
        for kind in ["uml.note", "uml.comment", "uml.constraint"] {
            assert_eq!(uml_notation(kind), UmlNotation::FoldedNote);
        }
        for (kind, notation) in [
            ("uml.frame", UmlNotation::Frame),
            ("uml.nary_association", UmlNotation::AssociationDiamond),
            ("uml.port", UmlNotation::Port),
            ("uml.part", UmlNotation::Part),
        ] {
            assert_eq!(uml_notation(kind), notation);
        }
    }

    #[test]
    fn projection_does_not_rewrite_persisted_identity() -> serde_json::Result<()> {
        let element: crate::Element = serde_json::from_value(serde_json::json!({
            "id": "component", "kind": "uml.component", "name": "Service",
            "properties": {"stereotype": "Domain"}
        }))?;
        let before = serde_json::to_value(&element)?;
        assert_eq!(
            uml_notation(&element.kind),
            UmlNotation::ClassifierRectangle
        );
        assert_eq!(
            uml_header(
                &element.kind,
                element
                    .properties
                    .get("stereotype")
                    .and_then(serde_json::Value::as_str)
            )
            .keyword,
            Some("«component»")
        );
        assert_eq!(serde_json::to_value(&element)?, before);
        Ok(())
    }

    fn small_header_project(
        kind: &str,
        applied: Option<&str>,
    ) -> serde_json::Result<crate::Project> {
        serde_json::from_value(serde_json::json!({
            "id": "headers", "name": "Header layout",
            "elements": [
                {"id": "source", "kind": kind, "name": "Source", "properties": {"stereotype": applied}},
                {"id": "target", "kind": "uml.class", "name": "Target"}
            ],
            "diagrams": [{"id": "diagram", "name": "Headers", "uml_di_type": "UMLClassDiagram", "views": [
                {"id": "source-view", "subject": "source", "geometry": {"kind": "rect", "x": 0.0, "y": 0.0, "width": 100.0, "height": 48.0}},
                {"id": "target-view", "subject": "target", "geometry": {"kind": "rect", "x": 0.0, "y": 200.0, "width": 100.0, "height": 48.0}}
            ]}]
        }))
    }

    #[test]
    fn projected_anchors_follow_both_headers_without_mutating_small_views()
    -> Result<(), Box<dyn std::error::Error>> {
        for (kind, expected) in [
            ("uml.component", 64.0),
            ("uml.profile", 75.0),
            ("uml.interface", 90.0),
            ("rust.trait", 90.0),
        ] {
            let project = small_header_project(kind, Some("Domain"))?;
            let before = serde_json::to_value(&project)?;
            let section = crate::ModelingSection::from_model(&project);
            let path = crate::route_relationship(&section, "diagram", "source", "target")?;
            assert!(
                matches!(path, crate::Geometry::Path { start, .. } if (start.y - expected).abs() < 0.000001)
            );
            assert_eq!(
                serde_json::to_value(section.to_model(&project.id, &project.name))?,
                before
            );
        }
        Ok(())
    }

    #[test]
    fn resize_persists_the_same_header_minimum_and_preserves_plain_class_size()
    -> Result<(), Box<dyn std::error::Error>> {
        for (kind, applied, expected) in [
            ("uml.component", Some("Domain"), 64.0),
            ("uml.profile", Some("Domain"), 75.0),
            ("uml.class", None, 48.0),
        ] {
            let project = small_header_project(kind, applied)?;
            let updated = crate::commands::apply(
                &project,
                &crate::commands::Command::ResizeView {
                    view_id: "source-view".into(),
                    width: 100.0,
                    height: 48.0,
                },
                1,
            )?;
            let source_view = updated
                .diagrams
                .iter()
                .flat_map(|diagram| &diagram.views)
                .find(|view| view.id == "source-view")
                .ok_or("missing source view")?;
            assert!(
                matches!(source_view.geometry, crate::Geometry::Rect { height, .. } if height == expected)
            );
            assert_eq!(
                uml_header(kind, applied).minimum_height(uml_notation(kind)),
                expected
            );
        }
        Ok(())
    }
}
