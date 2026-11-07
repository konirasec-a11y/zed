use super::ModelingSection;
use serde::{Deserialize, Serialize};

fn current_schema() -> u32 {
    2
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Project {
    #[serde(default = "current_schema")]
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub model: ModelingSection,
}
