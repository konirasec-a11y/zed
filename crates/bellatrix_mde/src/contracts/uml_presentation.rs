use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StereotypeDisplay {
    #[serde(rename = "none")]
    Hidden,
    Label,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UmlStyle {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stereotype_display: Option<StereotypeDisplay>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suppress_attributes: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suppress_operations: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suppress_receptions: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_visibility: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_type: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_property: Option<bool>,
}

impl UmlStyle {
    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }

    pub fn should_show_visibility(&self) -> bool {
        self.show_visibility != Some(false)
    }
    pub fn should_show_type(&self) -> bool {
        self.show_type != Some(false)
    }
    pub fn should_show_property(&self) -> bool {
        self.show_property != Some(false)
    }
    pub fn should_show_stereotype(&self) -> bool {
        self.stereotype_display != Some(StereotypeDisplay::Hidden)
    }
}
