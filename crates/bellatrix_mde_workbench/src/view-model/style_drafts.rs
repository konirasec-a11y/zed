use bellatrix_mde::{
    LineMode, RelationshipRouting, StereotypeDisplay, Style, TextAlignment, TextVerticalAlignment,
    commands::ViewOrder,
};
use std::collections::HashMap;

pub(crate) const FIELDS: &[(&str, &str)] = &[
    ("fill", "Fill"),
    ("stroke", "Stroke"),
    ("text_color", "Text color"),
    ("font_family", "Font"),
    ("font_size", "Font size"),
    ("stroke_width", "Line width"),
    ("opacity", "Opacity"),
    ("show_visibility", "Show visibility"),
    ("show_type", "Show type"),
    ("show_property", "Show property annotations"),
    ("stereotype_display", "Stereotype display"),
    ("suppress_attributes", "Hide attribute compartment"),
    ("suppress_operations", "Hide operation compartment"),
    ("suppress_receptions", "Hide reception compartment"),
    ("word_wrap", "Wrap text"),
    ("show_shadow", "Show shadow"),
    ("line_mode", "Line pattern"),
    ("text_align", "Horizontal text alignment"),
    ("text_vertical_align", "Vertical text alignment"),
];

pub(crate) fn style_value(style: &Style, key: &str) -> String {
    match key {
        "show_visibility" => style.show_visibility.map(|value| value.to_string()),
        "show_type" => style.show_type.map(|value| value.to_string()),
        "show_property" => style.show_property.map(|value| value.to_string()),
        "stereotype_display" => style.stereotype_display.map(|value| {
            match value {
                StereotypeDisplay::Hidden => "none",
                StereotypeDisplay::Label => "label",
            }
            .into()
        }),
        "suppress_attributes" => style.suppress_attributes.map(|value| value.to_string()),
        "suppress_operations" => style.suppress_operations.map(|value| value.to_string()),
        "suppress_receptions" => style.suppress_receptions.map(|value| value.to_string()),
        "word_wrap" => style.word_wrap.map(|value| value.to_string()),
        "show_shadow" => style.show_shadow.map(|value| value.to_string()),
        "line_mode" => style.line_mode.map(|value| {
            match value {
                LineMode::Solid => "solid",
                LineMode::Dotted => "dotted",
                LineMode::Dashed => "dashed",
            }
            .into()
        }),
        "text_align" => style.text_align.map(|value| {
            match value {
                TextAlignment::Left => "left",
                TextAlignment::Center => "center",
                TextAlignment::Right => "right",
            }
            .into()
        }),
        "text_vertical_align" => style.text_vertical_align.map(|value| {
            match value {
                TextVerticalAlignment::Top => "top",
                TextVerticalAlignment::Center => "center",
                TextVerticalAlignment::Bottom => "bottom",
            }
            .into()
        }),
        "fill" => style.fill.clone(),
        "stroke" => style.stroke.clone(),
        "text_color" => style.text_color.clone(),
        "font_family" => style.font_family.clone(),
        "font_size" => style.font_size.map(|value| value.to_string()),
        "stroke_width" => style.stroke_width.map(|value| value.to_string()),
        "opacity" => style.opacity.map(|value| value.to_string()),
        _ => None,
    }
    .unwrap_or_default()
}

fn optional_number(text: &str) -> Result<Option<f64>, &'static str> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(None);
    }
    text.parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())
        .map(Some)
        .ok_or("Enter a finite number, or leave blank to inherit")
}

fn matches_value(key: &str, draft: &str, stored: &str) -> bool {
    if matches!(key, "font_size" | "stroke_width" | "opacity") {
        match (optional_number(draft), optional_number(stored)) {
            (Ok(draft), Ok(stored)) => draft == stored,
            _ => false,
        }
    } else {
        draft.trim() == stored.trim()
    }
}

pub(crate) fn field_error(key: &str, text: &str) -> Option<&'static str> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    match key {
        "stereotype_display" => {
            (!matches!(text, "none" | "label")).then_some("Choose stereotype label or hidden")
        }
        "show_visibility"
        | "show_type"
        | "show_property"
        | "word_wrap"
        | "show_shadow"
        | "suppress_attributes"
        | "suppress_operations"
        | "suppress_receptions" => {
            (!matches!(text, "true" | "false")).then_some("Choose enabled or disabled")
        }
        "line_mode" => {
            (!matches!(text, "solid" | "dotted" | "dashed")).then_some("Choose a line pattern")
        }
        "text_align" => (!matches!(text, "left" | "center" | "right"))
            .then_some("Choose horizontal text alignment"),
        "text_vertical_align" => (!matches!(text, "top" | "center" | "bottom"))
            .then_some("Choose vertical text alignment"),
        "fill" | "stroke" | "text_color" => {
            let valid = text.strip_prefix('#').is_some_and(|digits| {
                matches!(digits.len(), 6 | 8)
                    && digits.bytes().all(|digit| digit.is_ascii_hexdigit())
            });
            (!valid).then_some("Use #RRGGBB or #RRGGBBAA, or leave blank to inherit")
        }
        "font_size" | "stroke_width" | "opacity" => match optional_number(text) {
            Err(error) => Some(error),
            Ok(Some(value)) if key == "opacity" && !(0.0..=1.0).contains(&value) => {
                Some("Enter a number between 0 and 1")
            }
            Ok(Some(value)) if key == "font_size" && !(1.0..=400.0).contains(&value) => {
                Some("Enter a font size between 1 and 400")
            }
            Ok(Some(value)) if key == "stroke_width" && value > 100.0 => {
                Some("Enter a line width up to 100")
            }
            Ok(Some(value)) if key != "opacity" && value <= 0.0 => Some("Enter a positive number"),
            _ => None,
        },
        "font_family" => text
            .chars()
            .any(char::is_control)
            .then_some("Font names cannot contain control characters"),
        _ => None,
    }
}

#[derive(Clone, Debug)]
pub(crate) struct StyleSubmission {
    pub expected_revision: u64,
    pub style: Style,
    pub routing: Option<RelationshipRouting>,
    pub order: Option<ViewOrder>,
    reset: bool,
    captured: HashMap<String, String>,
    admitted: HashMap<String, String>,
}

pub(crate) struct StyleEditState {
    pub values: HashMap<String, String>,
    pub baseline: Style,
    pub revision: u64,
    pub conflict: bool,
    pub error: Option<String>,
    pub pending: Option<StyleSubmission>,
    queued: bool,
    admitted: HashMap<String, String>,
    routing: Option<RelationshipRouting>,
    order: Option<ViewOrder>,
    reset: bool,
}

impl StyleEditState {
    pub fn new(style: &Style, revision: u64) -> Self {
        Self {
            values: FIELDS
                .iter()
                .map(|(key, _)| ((*key).into(), style_value(style, key)))
                .collect(),
            baseline: style.clone(),
            revision,
            conflict: false,
            error: None,
            pending: None,
            queued: false,
            admitted: HashMap::new(),
            routing: None,
            order: None,
            reset: false,
        }
    }

    pub fn edit(&mut self, key: &str, text: String) -> bool {
        if self.values.get(key) == Some(&text) && self.admitted.get(key) == Some(&text) {
            return false;
        }
        self.values.insert(key.into(), text.clone());
        self.admitted.insert(key.into(), text);
        self.queued = true;
        self.error = None;
        true
    }

    pub fn preserve_input(&mut self, key: &str, text: String) {
        self.values.insert(key.into(), text);
    }

    pub fn reorder(&mut self, order: ViewOrder) {
        self.order = Some(order);
        self.queued = true;
        self.error = None;
    }

    pub fn set_routing(&mut self, routing: RelationshipRouting) {
        self.routing = Some(routing);
        self.queued = true;
        self.error = None;
    }

    pub fn displayed_routing(&self) -> Option<RelationshipRouting> {
        self.routing.or(self.baseline.routing)
    }

    pub fn restore_theme(&mut self) {
        self.reset = true;
        self.queued = true;
        self.error = None;
    }

    pub fn needs_flush(&self) -> bool {
        self.queued && self.pending.is_none() && !self.conflict && self.error.is_none()
    }

    pub fn dirty(&self) -> bool {
        self.values
            .iter()
            .any(|(key, value)| !matches_value(key, value, &style_value(&self.baseline, key)))
    }

    pub fn reconcile(&mut self, current: &Style, revision: u64) {
        if self.pending.is_some() || revision < self.revision {
            return;
        }
        for (key, value) in &mut self.values {
            let baseline = style_value(&self.baseline, key);
            let canonical = style_value(current, key);
            if matches_value(key, value, &baseline) {
                if !matches_value(key, value, &canonical) {
                    *value = canonical;
                }
            } else if !matches_value(key, value, &canonical) && canonical != baseline {
                self.conflict = true;
            }
        }
        if self.routing.is_some_and(|routing| {
            current.routing != self.baseline.routing && current.routing != Some(routing)
        }) {
            self.conflict = true;
        }
        if !self.conflict {
            self.baseline = current.clone();
            self.revision = revision;
        }
    }

    pub fn next_submission(&mut self) -> Option<StyleSubmission> {
        if self.pending.is_some() || self.conflict || self.error.is_some() || !self.queued {
            return None;
        }
        self.queued = false;
        let mut style = if self.reset && self.order.is_none() && self.routing.is_none() {
            Style {
                routing: self.baseline.routing,
                ..Style::default()
            }
        } else {
            self.baseline.clone()
        };
        let routing = self
            .routing
            .filter(|routing| Some(*routing) != style.routing);
        let order = self.order;
        if order.is_some() {
            // Ordering shares the same receipt gate, but does not submit unrelated draft text.
        } else if let Some(routing) = routing {
            style.routing = Some(routing);
        } else {
            for (key, _) in FIELDS {
                let text = if self.reset {
                    ""
                } else if let Some(value) = self.admitted.get(*key) {
                    value.trim()
                } else {
                    continue;
                };
                if field_error(key, text).is_some() {
                    continue;
                }
                let value = (!text.is_empty()).then(|| text.to_owned());
                match *key {
                    "show_visibility" => style.show_visibility = text.parse().ok(),
                    "show_type" => style.show_type = text.parse().ok(),
                    "show_property" => style.show_property = text.parse().ok(),
                    "stereotype_display" => {
                        style.stereotype_display = match text {
                            "none" => Some(StereotypeDisplay::Hidden),
                            "label" => Some(StereotypeDisplay::Label),
                            _ => None,
                        }
                    }
                    "suppress_attributes" => style.suppress_attributes = text.parse().ok(),
                    "suppress_operations" => style.suppress_operations = text.parse().ok(),
                    "suppress_receptions" => style.suppress_receptions = text.parse().ok(),
                    "word_wrap" => style.word_wrap = text.parse().ok(),
                    "show_shadow" => style.show_shadow = text.parse().ok(),
                    "line_mode" => {
                        if text.is_empty() {
                            style.dash_pattern.clear();
                        }
                        style.line_mode = match text {
                            "solid" => Some(LineMode::Solid),
                            "dotted" => Some(LineMode::Dotted),
                            "dashed" => Some(LineMode::Dashed),
                            _ => None,
                        };
                    }
                    "text_align" => {
                        style.text_align = match text {
                            "left" => Some(TextAlignment::Left),
                            "center" => Some(TextAlignment::Center),
                            "right" => Some(TextAlignment::Right),
                            _ => None,
                        }
                    }
                    "text_vertical_align" => {
                        style.text_vertical_align = match text {
                            "top" => Some(TextVerticalAlignment::Top),
                            "center" => Some(TextVerticalAlignment::Center),
                            "bottom" => Some(TextVerticalAlignment::Bottom),
                            _ => None,
                        }
                    }
                    "fill" => style.fill = value,
                    "stroke" => style.stroke = value,
                    "text_color" => style.text_color = value,
                    "font_family" => style.font_family = value,
                    "font_size" => style.font_size = optional_number(text).ok().flatten(),
                    "stroke_width" => style.stroke_width = optional_number(text).ok().flatten(),
                    "opacity" => style.opacity = optional_number(text).ok().flatten(),
                    _ => {}
                }
            }
        }
        // Even a no-op reset needs a receipt before discarding invalid local text.
        if style == self.baseline && !self.reset && order.is_none() {
            self.admitted.clear();
            self.routing = None;
            return None;
        }
        let submission = StyleSubmission {
            expected_revision: self.revision,
            style,
            routing: if order.is_none() { routing } else { None },
            order,
            reset: self.reset && routing.is_none() && order.is_none(),
            captured: self.values.clone(),
            admitted: self.admitted.clone(),
        };
        self.pending = Some(submission.clone());
        Some(submission)
    }

    pub fn acknowledge(&mut self, receipt: Result<u64, String>) {
        let Some(submitted) = self.pending.take() else {
            return;
        };
        match receipt {
            Ok(revision)
                if revision == submitted.expected_revision
                    || revision == submitted.expected_revision.saturating_add(1) =>
            {
                self.baseline = submitted.style;
                if submitted.routing.is_none() && submitted.order.is_none() {
                    self.admitted
                        .retain(|key, value| submitted.admitted.get(key) != Some(value));
                }
                self.revision = revision;
                self.error = None;
                if submitted.reset {
                    for (key, value) in &mut self.values {
                        if submitted.captured.get(key) == Some(value) {
                            value.clear();
                        }
                    }
                    self.reset = false;
                }
                if submitted.routing == self.routing {
                    self.routing = None;
                }
                if submitted.order == self.order {
                    self.order = None;
                }
                self.queued |= !self.admitted.is_empty()
                    || self.routing.is_some()
                    || self.order.is_some()
                    || self.reset;
            }
            Ok(_) => {
                self.error = Some("Unexpected style command revision".into());
                self.queued = false;
                self.reset = false;
            }
            Err(error) => {
                let newer_edit = self.order != submitted.order
                    || self.admitted != submitted.admitted
                    || self
                        .routing
                        .is_some_and(|routing| Some(routing) != submitted.routing);
                self.error = (!newer_edit).then_some(error);
                self.queued = newer_edit;
                self.reset = false;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bellatrix_mde::{
        Project,
        commands::{Command, apply},
    };

    #[test]
    fn realtime_style_editing_preserves_typing_through_receipts_conflicts_and_reset()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut model: Project =
            serde_json::from_str(include_str!("../../../bellatrix_mde/fixtures/todo.json"))?;
        let style = |model: &Project| -> Result<Style, Box<dyn std::error::Error>> {
            model
                .diagrams
                .iter()
                .find(|diagram| diagram.id == "diagram-tasks")
                .and_then(|diagram| diagram.views.iter().find(|view| view.id == "view-task"))
                .map(|view| view.style.clone())
                .ok_or_else(|| "view missing".into())
        };
        let submit = |model: &Project, submission: &StyleSubmission| {
            apply(
                model,
                &Command::SetViewStyle {
                    diagram_id: "diagram-tasks".into(),
                    view_id: "view-task".into(),
                    style: submission.style.clone(),
                },
                submission.expected_revision + 1,
            )
        };
        let original_elements = model.elements.clone();
        let mut edits = StyleEditState::new(&style(&model)?, 0);
        edits.edit("fill", "#".into());
        assert!(edits.next_submission().is_none());
        assert!(field_error("fill", &edits.values["fill"]).is_some());
        edits.edit("opacity", "0.4".into());
        let first = edits
            .next_submission()
            .ok_or("valid opacity was not queued")?;
        assert_eq!(first.style.fill, None);
        model = submit(&model, &first)?;
        edits.edit("fill", "#123456".into());
        edits.edit("font_size", "16.0".into());
        assert!(
            edits.next_submission().is_none(),
            "one command stays in flight"
        );
        edits.acknowledge(Ok(1));
        edits.reconcile(&style(&model)?, 1);
        let second = edits.next_submission().ok_or("coalesced edit missing")?;
        assert_eq!(second.expected_revision, 1);
        assert_eq!(second.style.fill.as_deref(), Some("#123456"));
        assert_eq!(second.style.font_size, Some(16.0));
        model = submit(&model, &second)?;
        edits.edit("fill", "#abcdef".into());
        edits.acknowledge(Ok(2));
        edits.reconcile(&style(&model)?, 2);
        assert_eq!(edits.values["fill"], "#abcdef");
        assert_eq!(
            edits.values["font_size"], "16.0",
            "normalization does not reset the input/cursor"
        );
        let third = edits.next_submission().ok_or("later typing missing")?;
        model = submit(&model, &third)?;
        edits.acknowledge(Ok(3));
        edits.reconcile(&style(&model)?, 3);
        assert!(
            edits.next_submission().is_none(),
            "canonical events do not resend edits"
        );

        edits.edit("fill", "#334455".into());
        let mut remote = style(&model)?;
        remote.fill = Some("#445566".into());
        model = apply(
            &model,
            &Command::SetViewStyle {
                diagram_id: "diagram-tasks".into(),
                view_id: "view-task".into(),
                style: remote,
            },
            4,
        )?;
        edits.reconcile(&style(&model)?, 4);
        assert!(edits.conflict);
        assert!(edits.next_submission().is_none());
        assert_eq!(edits.values["fill"], "#334455");
        edits = StyleEditState::new(&style(&model)?, 4);
        edits.edit("fill", "#bad".into());
        edits.restore_theme();
        let reset = edits.next_submission().ok_or("reset missing")?;
        model = submit(&model, &reset)?;
        edits.edit("fill", "#556677".into());
        edits.acknowledge(Ok(5));
        edits.reconcile(&style(&model)?, 5);
        assert_eq!(edits.values["font_size"], "");
        assert_eq!(edits.values["fill"], "#556677");
        let after_reset = edits
            .next_submission()
            .ok_or("typing after reset missing")?;
        assert_eq!(after_reset.style.fill.as_deref(), Some("#556677"));
        edits.acknowledge(Err("stale revision".into()));
        assert!(
            edits.next_submission().is_none(),
            "rejection never enters a resend loop"
        );
        assert_eq!(edits.values["fill"], "#556677");
        edits.edit("fill", "#667788".into());
        edits.next_submission().ok_or("retry missing")?;
        edits.edit("fill", "#778899".into());
        edits.preserve_input("stroke", "#112233".into());
        edits.acknowledge(Err("earlier edit rejected".into()));
        let corrected = edits
            .next_submission()
            .ok_or("newer valid input was dropped")?;
        assert_eq!(corrected.style.fill.as_deref(), Some("#778899"));
        assert_eq!(
            corrected.style.stroke, None,
            "unadmitted text is not submitted with another field"
        );
        assert_eq!(model.elements, original_elements);
        Ok(())
    }
}
