// Adapted from javapak/ruml-wasm (MIT); native type.
#![allow(unused_imports)]
use serde::{Serialize, Deserialize};
use crate::visibility_kind::VisibilityKind;
use crate::parameter_set::ParameterSet;
use crate::parameter_direction_kind::ParameterDirectionKind;
use crate::parameter_effect_kind::ParameterEffectKind;
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Parameter {
    /// Unique identifier for this instance

    pub id: String,

    pub e_annotations: Vec<String>,

    pub owned_comment: Vec<String>,

    pub name: Option<String>,

    pub visibility: Option<VisibilityKind>,

    pub client_dependency: Vec<String>,

    pub name_expression: Option<String>,

    pub type_: Option<String>,

    pub owning_template_parameter: Option<String>,

    pub template_parameter: Option<String>,

    pub is_ordered: String,

    pub is_unique: String,

    pub upper_value: Option<String>,

    pub lower_value: Option<String>,

    pub parameter_set: Vec<String>,

    pub direction: ParameterDirectionKind,

    pub default_value: Option<String>,

    pub is_exception: String,

    pub is_stream: String,

    pub effect: Option<ParameterEffectKind>,
}
