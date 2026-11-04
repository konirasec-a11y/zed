use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EndpointMarkerGlyph {
    CrowfootOne,
    CrowfootMany,
    CrowfootZeroOne,
    CrowfootZeroMany,
    OpenArrow,
    SolidArrow,
    BlankArrow,
    Cross,
    OpenDiamond,
    FilledDiamond,
    OpenTriangle,
    FilledTriangle,
    OpenCircle,
    FilledCircle,
    Dot,
    Containment,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EndpointMarkerComposition {
    /// Painted in order. An empty composition intentionally draws no marker;
    /// it is not an absent value. This is persistent presentation, not UML data.
    pub glyphs: Vec<EndpointMarkerPart>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EndpointMarkerPlacement {
    /// Anchor at the edge endpoint, allowing explicitly authored overlays.
    Endpoint,
    /// Anchor at the inward edge of the preceding part. On the first part this
    /// has the same origin as Endpoint; glyph sizes are renderer measurements.
    AfterPrevious,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EndpointMarkerPart {
    pub glyph: EndpointMarkerGlyph,
    pub placement: EndpointMarkerPlacement,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "source", rename_all = "snake_case", deny_unknown_fields)]
pub enum EndpointMarkerSource {
    /// Resolve from the bound endpoint and the view's notation convention.
    /// No copied semantic values or cached marker composition are persisted.
    Model,
    /// An explicit graphical override, not a change to the model endpoint.
    Authored {
        composition: EndpointMarkerComposition,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "target", rename_all = "snake_case", deny_unknown_fields)]
pub enum EndpointMarkerTarget {
    /// A separately identified relationship end, such as an association Property.
    ModelEnd { id: String },
    /// A role of this view's subject (or visual draft), such as client/supplier.
    /// Relationships without independently identified ends need no invented ID.
    /// For a multi-valued role, the view attachment selects the displayed
    /// occurrence; this binding neither replaces nor narrows the model role.
    RelationshipRole { role: String },
    /// A graphical endpoint in the same diagram, independent of semantic ends.
    View { id: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EndpointMarkerBinding {
    pub target: EndpointMarkerTarget,
    pub marker: EndpointMarkerSource,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RelationshipEndpointPresentation {
    pub source: EndpointMarkerBinding,
    pub target: EndpointMarkerBinding,
}
