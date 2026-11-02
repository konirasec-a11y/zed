// Native UML/Ecore declaration from javapak/ruml-wasm 5c90d7399acbce4c37847b272759ed85d677ba92; MIT.
#![allow(unused_imports)]
use serde::{Serialize, Deserialize};
use crate::visibility_kind::VisibilityKind;
use crate::deployment_specification::DeploymentSpecification;
use crate::deployment_target::DeploymentTarget;
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Deployment {
    /// Unique identifier for this instance

    pub id: String,

    pub e_annotations: Vec<String>,

    pub owned_comment: Vec<String>,

    pub name: Option<String>,

    pub visibility: Option<VisibilityKind>,

    pub client_dependency: Vec<String>,

    pub name_expression: Option<String>,

    pub owning_template_parameter: Option<String>,

    pub template_parameter: Option<String>,

    pub supplier: Vec<String>,

    pub client: Vec<String>,

    pub deployed_artifact: Vec<String>,

    pub configuration: Vec<String>,

    pub location: String,
}
