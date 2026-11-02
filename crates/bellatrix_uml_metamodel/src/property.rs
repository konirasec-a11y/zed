// Native UML/Ecore declaration from javapak/ruml-wasm 5c90d7399acbce4c37847b272759ed85d677ba92; MIT.
#![allow(unused_imports)]
use serde::{Serialize, Deserialize};
use crate::visibility_kind::VisibilityKind;
use crate::data_type::DataType;
use crate::aggregation_kind::AggregationKind;
use crate::association::Association;
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Property {
    /// Unique identifier for this instance

    pub id: String,

    pub e_annotations: Vec<String>,

    pub owned_comment: Vec<String>,

    pub name: Option<String>,

    pub visibility: Option<VisibilityKind>,

    pub client_dependency: Vec<String>,

    pub name_expression: Option<String>,

    pub is_leaf: String,

    pub is_static: String,

    pub type_: Option<String>,

    pub is_ordered: String,

    pub is_unique: String,

    pub upper_value: Option<String>,

    pub lower_value: Option<String>,

    pub is_read_only: String,

    pub owning_template_parameter: Option<String>,

    pub template_parameter: Option<String>,

    pub deployment: Vec<String>,

    pub template_binding: Vec<String>,

    pub owned_template_signature: Option<String>,

    pub datatype: Option<String>,

    pub is_derived: String,

    pub is_derived_union: String,

    pub aggregation: AggregationKind,

    pub redefined_property: Vec<String>,

    pub owning_association: Option<String>,

    pub default_value: Option<String>,

    pub subsetted_property: Vec<String>,

    pub association: Option<String>,

    pub qualifier: Vec<String>,

    pub association_end: Option<String>,
}
