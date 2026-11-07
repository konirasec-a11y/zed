use crate::uml::types::{RelationEnd, RelationEndMultiplicity};
use crate::{LineMode, ModelingSection, Relation};
pub use crate::contracts::protocol::presentation::{
    RelationshipEndpointLabels, RelationshipMarker, RelationshipStyle,
};

pub fn relationship_style_for_notation(kind: &str) -> RelationshipStyle {
    use RelationshipMarker::*;
    let (source_marker, target_marker, dashed) = match kind {
        "uml.generalization" => (None, Triangle, false),
        "uml.extension" | "uml.message" => (None, FilledTriangle, false),
        "uml.control_flow" | "uml.object_flow" | "uml.transition" => (None, OpenArrow, false),
        "uml.include"
        | "uml.extend"
        | "uml.package_import"
        | "uml.package_merge"
        | "uml.deployment"
        | "uml.manifestation"
        | "uml.abstraction"
        | "uml.substitution"
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
        "view.association_class_attachment" | "uml.note_link" | "uml.constraint_link" => {
            (None, None, true)
        }
        _ => (None, None, false),
    };
    RelationshipStyle {
        source_marker,
        target_marker,
        line_mode: if kind == "uml.general_ordering" {
            LineMode::Dotted
        } else if dashed {
            LineMode::Dashed
        } else {
            LineMode::Solid
        },
        middle_marker: if kind == "uml.general_ordering" {
            FilledTriangle
        } else {
            None
        },
    }
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
    pub diagnostics: Vec<String>,
}
impl RelationshipPresentation {
    pub fn describe(&self, from: &str, to: &str) -> String {
        if self.style.source_marker == RelationshipMarker::None
            && self.style.target_marker == RelationshipMarker::None
            && self.style.middle_marker == RelationshipMarker::None
            && self.style.line_mode == LineMode::Solid
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
pub(crate) fn association_diamonds<'a>(
    section: &'a ModelingSection,
    relation: &Relation,
) -> Result<[&'a str; 2], String> {
    let references =
        crate::uml::references::ReferenceIndex::new(crate::uml::root::Model::from_section(section));
    let ends = crate::uml::associations::member_ends(&references, &relation.id)?;
    let [source, target] = ends.as_slice() else {
        return Err("Association diamond projection requires two Property ends".into());
    };
    // The diamond is opposite the typed Property that owns aggregation.
    Ok([
        crate::uml::associations::aggregation(*target)?,
        crate::uml::associations::aggregation(*source)?,
    ])
}

pub fn relationship_presentation(
    section: &ModelingSection,
    relation: &Relation,
) -> RelationshipPresentation {
    let mut diagnostics = Vec::new();
    let [source_aggregation, target_aggregation] = if matches!(
        relation.kind.as_str(),
        "uml.association" | "uml.directed_association"
    ) {
        match association_diamonds(section, relation) {
            Ok(aggregations) => aggregations,
            Err(message) => {
                diagnostics.push(message);
                ["none", "none"]
            }
        }
    } else {
        ["none", "none"]
    };
    let notation = match relation.kind.as_str() {
        "depends_on" => "uml.dependency",
        "uml.association" | "uml.directed_association" => {
            let aggregations = [source_aggregation, target_aggregation];
            if aggregations.contains(&"composite") {
                "uml.composition"
            } else if aggregations.contains(&"shared") {
                "uml.aggregation"
            } else {
                relation.kind.as_str()
            }
        }
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
            if matches!(target_aggregation, "shared" | "composite") =>
        {
            ("part", "whole")
        }
        "uml.aggregation" | "uml.composition" => ("whole", "part"),
        "uml.containment" => ("container", "contained"),
        _ => ("source", "target"),
    };
    let text = |key: &str| {
        relation
            .properties
            .get(key)
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned)
    };
    let mut style = resolve_relationship_style(
        if relation.kind == "uml.directed_association" {
            &relation.kind
        } else {
            notation
        },
        &relation.properties,
    );

    if matches!(
        relation.kind.as_str(),
        "uml.association" | "uml.directed_association"
    ) {
        let marker = |aggregation: &str| match aggregation {
            "shared" => RelationshipMarker::Diamond,
            "composite" => RelationshipMarker::FilledDiamond,
            _ => RelationshipMarker::None,
        };
        style.source_marker = marker(source_aggregation);
        let target_marker = marker(target_aggregation);
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
        diagnostics,
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
        "uml.substitution" => Some("«substitute»"),
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

/// Interpret notation metadata equally for canonical relationships and View-only drafts.
pub fn resolve_relationship_style(
    kind: &str,
    properties: &std::collections::BTreeMap<String, serde_json::Value>,
) -> RelationshipStyle {
    let mut style = relationship_style_for_notation(kind);
    if kind == "uml.message" {
        match properties
            .get("messageSort")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("synchCall")
        {
            "synchCall" => style.target_marker = RelationshipMarker::FilledTriangle,
            "reply" | "createMessage" => {
                style.target_marker = RelationshipMarker::OpenArrow;
                style.line_mode = LineMode::Dashed;
            }
            "asynchCall" | "asynchSignal" | "deleteMessage" => {
                style.target_marker = RelationshipMarker::OpenArrow
            }
            _ => {}
        }
        match properties
            .get("messageKind")
            .and_then(serde_json::Value::as_str)
        {
            Some("found") => style.source_marker = RelationshipMarker::FilledCircle,
            Some("lost") => {
                style.target_marker = if style.target_marker == RelationshipMarker::FilledTriangle {
                    RelationshipMarker::FilledTriangleAndCircle
                } else {
                    RelationshipMarker::OpenArrowAndCircle
                };
            }
            _ => {}
        }
    }
    style
}
