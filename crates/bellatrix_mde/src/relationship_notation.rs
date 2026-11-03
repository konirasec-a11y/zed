use crate::Relation;
use crate::uml::types::{RelationEnd, RelationEndMultiplicity};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelationshipMarker {
    None,
    OpenArrow,
    Triangle,
    Diamond,
    FilledDiamond,
    FilledTriangle,
    FilledCircle,
    Containment,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RelationshipStyle {
    pub source_marker: RelationshipMarker,
    pub target_marker: RelationshipMarker,
    pub dashed: bool,
}
pub fn relationship_style_for_notation(kind: &str) -> RelationshipStyle {
    use RelationshipMarker::*;
    let (source_marker, target_marker, dashed) = match kind {
        "uml.generalization" => (None, Triangle, false),
        "uml.extension" => (None, FilledTriangle, false),
        "uml.control_flow" | "uml.object_flow" | "uml.transition" => (None, OpenArrow, false),
        "uml.include"
        | "uml.extend"
        | "uml.package_import"
        | "uml.package_merge"
        | "uml.deployment"
        | "uml.manifestation"
        | "uml.abstraction"
        | "uml.profile_application"
        | "uml.element_import" => (None, OpenArrow, true),
        "uml.interface_realization" | "uml.realization" => (None, Triangle, true),
        "uml.directed_association" => (None, OpenArrow, false),
        "uml.dependency" | "uml.usage" | "uml.template_binding" | "depends_on" => {
            (None, OpenArrow, true)
        }
        "uml.aggregation" => (Diamond, None, false),
        "uml.composition" => (FilledDiamond, None, false),
        "uml.containment" => (Containment, None, false),
        "uml.note_link" | "uml.constraint_link" => (None, None, true),
        _ => (None, None, false),
    };
    RelationshipStyle {
        source_marker,
        target_marker,
        dashed,
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RelationshipEndpointLabels {
    pub role: Option<String>,
    pub multiplicity: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelationshipPresentation {
    pub name: Option<String>,
    pub source_labels: RelationshipEndpointLabels,
    pub target_labels: RelationshipEndpointLabels,
    pub label: String,
    pub notation: String,
    pub style: RelationshipStyle,
    pub source_role: &'static str,
    pub target_role: &'static str,
}
impl RelationshipPresentation {
    pub fn describe(&self, from: &str, to: &str) -> String {
        if self.style.source_marker == RelationshipMarker::None
            && self.style.target_marker == RelationshipMarker::None
            && !self.style.dashed
        {
            format!("{from} —[{}]— {to}", self.label)
        } else {
            format!(
                "{from} ({}) —[{}]→ {to} ({})",
                self.source_role, self.label, self.target_role
            )
        }
    }
}
pub fn relationship_presentation(relation: &Relation) -> RelationshipPresentation {
    let notation = match relation.kind.as_str() {
        "depends_on" => "uml.dependency",
        "uml.association" => match relation
            .properties
            .get("end1Aggregation")
            .or_else(|| relation.properties.get("end2Aggregation"))
            .and_then(serde_json::Value::as_str)
        {
            Some("shared") => "uml.aggregation",
            Some("composite") => "uml.composition",
            _ => "uml.association",
        },
        other => other,
    };
    let label = notation
        .strip_prefix("uml.")
        .unwrap_or(notation)
        .replace('_', " ");
    let (source_role, target_role) = match notation {
        "uml.generalization" => ("subtype", "supertype"),
        "uml.interface_realization" => ("implementer", "interface"),
        "uml.realization" => ("implementation", "specification"),
        "uml.dependency" | "uml.usage" | "uml.template_binding" => ("client", "supplier"),
        "uml.aggregation" | "uml.composition"
            if relation
                .properties
                .get("end2Aggregation")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|kind| matches!(kind, "shared" | "composite")) =>
        {
            ("part", "whole")
        }
        "uml.aggregation" | "uml.composition" => ("whole", "part"),
        "uml.containment" => ("container", "contained"),
        "uml.association" => ("end1", "end2"),
        _ => ("source", "target"),
    };
    let text = |key: &str| {
        relation
            .properties
            .get(key)
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned)
    };
    let mut style = relationship_style_for_notation(notation);
    if relation.kind == "uml.message" {
        match relation
            .properties
            .get("messageSort")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("synchCall")
        {
            "synchCall" => style.target_marker = RelationshipMarker::FilledTriangle,
            "reply" | "createMessage" => {
                style.target_marker = RelationshipMarker::OpenArrow;
                style.dashed = true;
            }
            "asynchCall" | "asynchSignal" | "deleteMessage" => {
                style.target_marker = RelationshipMarker::OpenArrow
            }
            _ => {}
        }
    }

    if matches!(
        relation.kind.as_str(),
        "uml.association" | "uml.directed_association"
    ) {
        let marker = |key: &str| match relation
            .properties
            .get(key)
            .and_then(serde_json::Value::as_str)
        {
            Some("shared") => RelationshipMarker::Diamond,
            Some("composite") => RelationshipMarker::FilledDiamond,
            _ => RelationshipMarker::None,
        };
        style.source_marker = marker("end1Aggregation");
        let target_marker = marker("end2Aggregation");
        if relation.kind == "uml.association" || target_marker != RelationshipMarker::None {
            style.target_marker = target_marker;
        }
    }
    RelationshipPresentation {
        name: format_relationship_label(notation, &relation.properties),
        source_labels: RelationshipEndpointLabels {
            role: text("sourceRole"),
            multiplicity: RelationEndMultiplicity::new(relation, RelationEnd::Source)
                .authored_notation()
                .map(str::to_owned),
        },
        target_labels: RelationshipEndpointLabels {
            role: text("targetRole"),
            multiplicity: RelationEndMultiplicity::new(relation, RelationEnd::Target)
                .authored_notation()
                .map(str::to_owned),
        },
        label,
        notation: notation.to_owned(),
        style,
        source_role,
        target_role,
    }
}

/// Shared notation text for canonical edges and View-only visual drafts.
pub fn format_relationship_label(
    kind: &str,
    properties: &std::collections::BTreeMap<String, serde_json::Value>,
) -> Option<String> {
    let text = |key: &str| properties.get(key).and_then(serde_json::Value::as_str);
    let keyword = match kind {
        "uml.usage" => Some("«use»"),
        "uml.include" => Some("«include»"),
        "uml.extend" => Some("«extend»"),
        "uml.package_import" | "uml.element_import" => {
            Some(if text("visibility") == Some("private") {
                "«access»"
            } else {
                "«import»"
            })
        }
        "uml.package_merge" => Some("«merge»"),
        "uml.deployment" => Some("«deploy»"),
        "uml.manifestation" => Some("«manifest»"),
        "uml.profile_application" => Some(
            if properties
                .get("isStrict")
                .and_then(serde_json::Value::as_bool)
                == Some(true)
            {
                "«strict»"
            } else {
                "«apply»"
            },
        ),
        _ => None,
    };
    let name = text("name").filter(|name| !name.is_empty());
    match (keyword, name) {
        (Some(keyword), Some(name)) => Some(format!("{keyword} {name}")),
        (Some(keyword), None) => Some(keyword.into()),
        (None, name) => name.map(str::to_owned),
    }
}
