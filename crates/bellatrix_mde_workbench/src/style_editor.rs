use crate::property_editor::compact_row;
use crate::{MdeWorkbenchItem, panels::MdeRightPanel};
use bellatrix_mde::{RelationshipRouting, Style, View};
use bellatrix_mde_cordis::{
    MdeUiCommand, ModelingSetViewRoutingRequested, ModelingSetViewStyleRequested,
};
use gpui::{AnyElement, Context, Entity, IntoElement, Window, div, prelude::*, px};
use std::collections::HashMap;
use ui::{Button, ContextMenu, DropdownMenu, Label, prelude::*};
use ui_input::InputField;

const FIELDS: &[(&str, &str)] = &[
    ("fill", "Fill"),
    ("stroke", "Stroke"),
    ("text_color", "Text color"),
    ("font_family", "Font"),
    ("font_size", "Font size"),
    ("stroke_width", "Line width"),
    ("opacity", "Opacity"),
];

pub(crate) fn inputs_for_style(
    style: &Style,
    window: &mut Window,
    cx: &mut Context<MdeRightPanel>,
) -> HashMap<String, Entity<InputField>> {
    FIELDS
        .iter()
        .map(|(key, _)| {
            let value = style_value(style, key);
            let input = cx.new(|cx| {
                let input = InputField::new(window, cx, "Inherited").label_min_width(px(0.0));
                input.set_text(&value, window, cx);
                input
            });
            ((*key).to_owned(), input)
        })
        .collect()
}

fn style_value(style: &Style, key: &str) -> String {
    match key {
        "fill" => style.fill.clone(),
        "stroke" => style.stroke.clone(),
        "text_color" => style.text_color.clone(),
        "font_family" => style.font_family.clone(),
        "font_size" => style.font_size.map(|v| v.to_string()),
        "stroke_width" => style.stroke_width.map(|v| v.to_string()),
        "opacity" => style.opacity.map(|v| v.to_string()),
        _ => None,
    }
    .unwrap_or_default()
}

fn style_value_matches(key: &str, draft: &str, stored: &str) -> bool {
    if matches!(key, "font_size" | "stroke_width" | "opacity") {
        match (optional_number(draft), optional_number(stored)) {
            (Ok(draft), Ok(stored)) => draft == stored,
            _ => false,
        }
    } else {
        draft.trim() == stored.trim()
    }
}

pub(crate) fn style_inputs_dirty(
    inputs: &HashMap<String, Entity<InputField>>,
    baseline: &Style,
    cx: &gpui::App,
) -> bool {
    inputs.iter().any(|(key, input)| {
        !style_value_matches(key, &input.read(cx).text(cx), &style_value(baseline, key))
    })
}

pub(crate) fn sync_style_inputs(
    inputs: &HashMap<String, Entity<InputField>>,
    previous: &Style,
    current: &Style,
    window: &mut Window,
    cx: &mut Context<MdeRightPanel>,
) {
    for (key, input) in inputs {
        let text = input.read(cx).text(cx);
        if style_value_matches(key, &text, &style_value(previous, key))
            && !style_value_matches(key, &text, &style_value(current, key))
        {
            input.update(cx, |field, cx| {
                field.set_text(&style_value(current, key), window, cx)
            });
        }
    }
}

pub(crate) struct PendingThemeReset {
    expected_revision: u64,
    style: Style,
    drafts: HashMap<String, String>,
    completion: tokio::sync::watch::Receiver<Option<Result<u64, String>>>,
}

#[derive(Debug, PartialEq)]
pub(crate) enum ResetAcknowledgment {
    Pending,
    Applied,
    Rejected,
}

fn reset_acknowledgment(
    expected_revision: u64,
    desired: &Style,
    revision: u64,
    current: &Style,
    completion: Option<&Result<u64, String>>,
) -> ResetAcknowledgment {
    match completion {
        Some(Ok(applied_revision))
            if *applied_revision == expected_revision
                || *applied_revision == expected_revision.saturating_add(1) =>
        {
            if revision < *applied_revision {
                ResetAcknowledgment::Pending
            } else if revision == *applied_revision && current == desired {
                ResetAcknowledgment::Applied
            } else {
                ResetAcknowledgment::Rejected
            }
        }
        Some(_) => ResetAcknowledgment::Rejected,
        None => ResetAcknowledgment::Pending,
    }
}

fn style_submission_allowed(conflict: bool, reset_pending: bool) -> bool {
    !conflict && !reset_pending
}

fn reset_can_clear_draft(captured: &str, current: &str) -> bool {
    captured == current
}

pub(crate) fn acknowledge_theme_reset(
    pending: &PendingThemeReset,
    inputs: &HashMap<String, Entity<InputField>>,
    revision: u64,
    style: &Style,
    window: &mut Window,
    cx: &mut Context<MdeRightPanel>,
) -> ResetAcknowledgment {
    let completion = pending.completion.borrow().clone().or_else(|| {
        pending
            .completion
            .has_changed()
            .is_err()
            .then(|| Err("Style command channel closed".into()))
    });
    match reset_acknowledgment(
        pending.expected_revision,
        &pending.style,
        revision,
        style,
        completion.as_ref(),
    ) {
        ResetAcknowledgment::Pending => ResetAcknowledgment::Pending,
        ResetAcknowledgment::Rejected => ResetAcknowledgment::Rejected,
        ResetAcknowledgment::Applied => {
            for (key, input) in inputs {
                if pending.drafts.get(key).is_some_and(|captured| {
                    reset_can_clear_draft(captured, &input.read(cx).text(cx))
                }) {
                    input.update(cx, |field, cx| {
                        field.set_text("", window, cx);
                        field.set_error(None::<&str>, cx);
                    });
                }
            }
            ResetAcknowledgment::Applied
        }
    }
}

fn inherited_style_values() -> HashMap<String, String> {
    FIELDS
        .iter()
        .map(|(key, _)| ((*key).to_owned(), String::new()))
        .collect()
}

fn edited_style(style: &Style, values: HashMap<String, String>) -> Result<Style, &'static str> {
    let mut style = style.clone();
    for (key, text) in values {
        let text = text.trim();
        let string = (!text.is_empty()).then(|| text.to_owned());
        match key.as_str() {
            "fill" => style.fill = string,
            "stroke" => style.stroke = string,
            "text_color" => style.text_color = string,
            "font_family" => style.font_family = string,
            "font_size" => style.font_size = optional_number(text)?,
            "stroke_width" => style.stroke_width = optional_number(text)?,
            "opacity" => style.opacity = optional_number(text)?,
            _ => {}
        }
    }
    Ok(style)
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

pub(crate) fn render_styles(
    target: Option<(&str, &View, bool)>,
    item: Entity<MdeWorkbenchItem>,
    inputs: &HashMap<String, Entity<InputField>>,
    expected_revision: u64,
    conflict: bool,
    reset_pending: bool,
    open: bool,
    window: &mut Window,
    cx: &mut Context<MdeRightPanel>,
) -> Vec<AnyElement> {
    let mut rows = vec![
        div()
            .px_3()
            .pt_2()
            .child(
                Button::new("mde-styles-toggle", "Styles")
                    .start_icon(
                        Icon::new(if open {
                            IconName::ChevronDown
                        } else {
                            IconName::ChevronRight
                        })
                        .size(IconSize::Small),
                    )
                    .on_click(cx.listener(|panel, _, window, cx| {
                        if panel.inspector_styles_open {
                            cx.focus_self(window);
                        }
                        panel.inspector_styles_open = !panel.inspector_styles_open;
                        cx.notify();
                    })),
            )
            .into_any_element(),
    ];
    if !open {
        return rows;
    }
    let Some((diagram_id, view, is_edge)) = target else {
        rows.push(
            div()
                .px_3()
                .py_1()
                .child(
                    Label::new("Select a diagram view to edit its appearance.")
                        .size(LabelSize::Small)
                        .color(Color::Muted),
                )
                .into_any_element(),
        );
        return rows;
    };
    for (key, label) in FIELDS {
        if is_edge && *key == "fill" {
            continue;
        }
        if let Some(input) = inputs.get(*key) {
            let color = input.read(cx).text(cx);
            let swatch = color
                .strip_prefix('#')
                .filter(|hex| hex.len() == 6)
                .and_then(|hex| u32::from_str_radix(hex, 16).ok());
            let control = div()
                .w_full()
                .flex()
                .items_center()
                .gap_1()
                .when(matches!(*key, "fill" | "stroke" | "text_color"), |row| {
                    row.child(
                        div()
                            .size(px(16.0))
                            .flex_none()
                            .border_1()
                            .border_color(cx.theme().colors().border)
                            .when_some(swatch, |swatch, color| swatch.bg(gpui::rgb(color))),
                    )
                })
                .child(div().flex_1().min_w_0().child(input.clone()));
            rows.push(compact_row(*label, control));
        }
    }
    if conflict {
        let key = (diagram_id.to_owned(), view.id.clone());
        rows.push(
            div()
                .px_3()
                .child(
                    Label::new("Project changed. Draft preserved; reload before applying.")
                        .size(LabelSize::XSmall)
                        .color(Color::Warning),
                )
                .child(
                    Button::new("mde-style-reload", "Discard draft and reload").on_click(
                        cx.listener(move |panel, _, _, cx| {
                            panel.pending_theme_resets.remove(&key);
                            panel.style_inputs.remove(&key);
                            panel.style_baselines.remove(&key);
                            panel.style_revisions.remove(&key);
                            panel.style_listeners.remove(&key);
                            cx.notify();
                        }),
                    ),
                )
                .into_any_element(),
        );
    }
    let apply_key = (diagram_id.to_owned(), view.id.clone());
    let captured_inputs = inputs.clone();
    let target_item = item.clone();
    let diagram = diagram_id.to_owned();
    let view_id = view.id.clone();
    rows.push(
        div()
            .px_3()
            .child(
                Button::new("mde-style-apply", "Apply style")
                    .disabled(!style_submission_allowed(conflict, reset_pending))
                    .on_click(cx.listener(move |panel, _, _, cx| {
                        if !style_submission_allowed(
                            conflict,
                            panel.pending_theme_resets.contains_key(&apply_key),
                        ) {
                            return;
                        }
                        let mut values = HashMap::new();
                        let mut valid = true;
                        for (key, input) in &captured_inputs {
                            let text = input.read(cx).text(cx);
                            let error =
                                if matches!(key.as_str(), "font_size" | "stroke_width" | "opacity")
                                {
                                    optional_number(&text).err()
                                } else {
                                    None
                                };
                            valid &= error.is_none();
                            input.update(cx, |field, cx| field.set_error(error, cx));
                            values.insert(key.clone(), text);
                        }
                        if valid {
                            target_item.update(cx, |item, cx| {
                                let _submitted = item.apply_view_style(
                                    &diagram,
                                    &view_id,
                                    expected_revision,
                                    values,
                                    None,
                                    cx,
                                );
                            });
                        }
                    })),
            )
            .into_any_element(),
    );
    let theme_inputs = inputs.clone();
    let theme_item = item.clone();
    let theme_diagram = diagram_id.to_owned();
    let theme_view = view.id.clone();
    rows.push(
        div()
            .px_3()
            .child(
                Button::new(
                    "mde-style-restore-theme",
                    if style_inputs_dirty(inputs, &view.style, cx) {
                        "Restore IDE theme and discard draft"
                    } else {
                        "Restore IDE theme"
                    },
                )
                .disabled(!style_submission_allowed(conflict, reset_pending))
                .on_click(cx.listener(move |panel, _, _, cx| {
                    if !style_submission_allowed(
                        conflict,
                        panel
                            .pending_theme_resets
                            .contains_key(&(theme_diagram.clone(), theme_view.clone())),
                    ) {
                        return;
                    }
                    let (completion_sender, completion) = tokio::sync::watch::channel(None);
                    let drafts = theme_inputs
                        .iter()
                        .map(|(key, input)| (key.clone(), input.read(cx).text(cx).to_string()))
                        .collect();
                    let submitted = theme_item.update(cx, |item, cx| {
                        item.apply_view_style(
                            &theme_diagram,
                            &theme_view,
                            expected_revision,
                            inherited_style_values(),
                            Some(completion_sender),
                            cx,
                        )
                    });
                    if let Some(style) = submitted {
                        panel.pending_theme_resets.insert(
                            (theme_diagram.clone(), theme_view.clone()),
                            PendingThemeReset {
                                expected_revision,
                                style,
                                drafts,
                                completion,
                            },
                        );
                    }
                    cx.notify();
                })),
            )
            .into_any_element(),
    );
    rows.push(
        div()
            .px_3()
            .child(
                Label::new("Blank values inherit the theme. Colors: #RRGGBB.")
                    .size(LabelSize::XSmall)
                    .color(Color::Muted),
            )
            .into_any_element(),
    );
    if is_edge {
        let diagram = diagram_id.to_owned();
        let view_id = view.id.clone();
        let menu = ContextMenu::build(window, cx, move |menu, _, _| {
            [
                ("Straight", RelationshipRouting::Straight),
                ("Automatic", RelationshipRouting::Automatic),
                ("Curve", RelationshipRouting::Curve),
            ]
            .into_iter()
            .fold(menu, |menu, (label, routing)| {
                let item = item.clone();
                let diagram = diagram.clone();
                let view_id = view_id.clone();
                menu.entry(label, None, move |_, cx| {
                    item.update(cx, |item, cx| {
                        item.apply_view_routing(&diagram, &view_id, routing, cx)
                    });
                })
            })
        });
        let label = match view.style.routing {
            Some(RelationshipRouting::Straight) => "Straight",
            Some(RelationshipRouting::Automatic) => "Automatic",
            Some(RelationshipRouting::Curve) => "Curve",
            None => "Stored geometry",
        };
        rows.push(
            compact_row(
                "Routing",
                DropdownMenu::new("mde-style-routing", label, menu).full_width(true),
            )
            .into_any_element(),
        );
        rows.push(
            div()
                .px_3()
                .child(
                    Label::new("UML controls arrowheads and line patterns.")
                        .size(LabelSize::XSmall)
                        .color(Color::Muted),
                )
                .into_any_element(),
        );
    }
    rows
}

impl MdeWorkbenchItem {
    fn apply_view_style(
        &mut self,
        diagram_id: &str,
        view_id: &str,
        expected_revision: u64,
        values: HashMap<String, String>,
        completion: Option<tokio::sync::watch::Sender<Option<Result<u64, String>>>>,
        cx: &mut Context<Self>,
    ) -> Option<Style> {
        if self.active_diagram.as_deref() != Some(diagram_id)
            || self.selected_view.as_deref() != Some(view_id)
        {
            return None;
        }
        let Ok(snapshot) = self.source.snapshot() else {
            self.feedback = "Project snapshot unavailable".into();
            cx.notify();
            return None;
        };
        let Some(view) = snapshot
            .section
            .diagrams
            .iter()
            .find(|d| d.id == diagram_id)
            .and_then(|d| d.views.iter().find(|v| v.id == view_id))
        else {
            return None;
        };
        let style = match edited_style(&view.style, values) {
            Ok(style) => style,
            Err(error) => {
                self.feedback = error.into();
                cx.notify();
                return None;
            }
        };
        let request = ModelingSetViewStyleRequested {
            project_id: self.project_id().clone(),
            expected_revision,
            diagram_id: diagram_id.into(),
            view_id: view_id.into(),
            style: style.clone(),
        };
        let command = match completion {
            Some(completion) => MdeUiCommand::SetViewStyleTracked {
                request,
                completion,
            },
            None => MdeUiCommand::SetViewStyle(request),
        };
        let result = self.source.submit(command);
        match result {
            Ok(()) => {
                self.feedback = "Command sent to Cordis".into();
                cx.notify();
                Some(style)
            }
            Err(error) => {
                self.feedback = format!("Command not sent: {error}");
                cx.notify();
                None
            }
        }
    }
    fn apply_view_routing(
        &mut self,
        diagram_id: &str,
        view_id: &str,
        routing: RelationshipRouting,
        cx: &mut Context<Self>,
    ) {
        if self.active_diagram.as_deref() != Some(diagram_id)
            || self.selected_view.as_deref() != Some(view_id)
        {
            return;
        }
        let Ok(snapshot) = self.source.snapshot() else {
            self.feedback = "Project snapshot unavailable".into();
            cx.notify();
            return;
        };
        self.submit(
            MdeUiCommand::SetViewRouting(ModelingSetViewRoutingRequested {
                project_id: self.project_id().clone(),
                expected_revision: snapshot.revision,
                diagram_id: diagram_id.into(),
                view_id: view_id.into(),
                routing,
            }),
            cx,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ResetAcknowledgment, edited_style, inherited_style_values, optional_number,
        reset_acknowledgment, reset_can_clear_draft, style_submission_allowed, style_value_matches,
    };
    use bellatrix_mde::{RelationshipRouting, Style};

    #[test]
    fn theme_reset_waits_for_authoritative_success() {
        let desired = Style::default();
        let authored = Style {
            fill: Some("#123456".into()),
            ..Style::default()
        };
        assert_eq!(
            reset_acknowledgment(4, &desired, 4, &desired, Some(&Ok(4))),
            ResetAcknowledgment::Applied
        );
        assert_eq!(
            reset_acknowledgment(4, &desired, 4, &authored, None),
            ResetAcknowledgment::Pending
        );
        assert_eq!(
            reset_acknowledgment(4, &desired, 5, &desired, None),
            ResetAcknowledgment::Pending
        );
        assert_eq!(
            reset_acknowledgment(4, &desired, 4, &authored, Some(&Ok(5))),
            ResetAcknowledgment::Pending
        );
        assert_eq!(
            reset_acknowledgment(4, &desired, 5, &desired, Some(&Ok(5))),
            ResetAcknowledgment::Applied
        );
    }

    #[test]
    fn theme_reset_rejection_staleness_and_late_typing_preserve_drafts() {
        let desired = Style::default();
        let authored = Style {
            fill: Some("#123456".into()),
            ..Style::default()
        };
        for (revision, current, completion) in [
            (4, &authored, Err("stale revision".into())),
            (4, &authored, Err("channel closed".into())),
            (4, &authored, Err("responder unavailable".into())),
            (5, &authored, Ok(5)),
            (6, &desired, Ok(5)),
            (5, &desired, Ok(6)),
        ] {
            assert_eq!(
                reset_acknowledgment(4, &desired, revision, current, Some(&completion)),
                ResetAcknowledgment::Rejected
            );
        }
        assert!(reset_can_clear_draft("#123456", "#123456"));
        assert!(!reset_can_clear_draft("#123456", "#abcdef"));
        assert!(!reset_can_clear_draft("#123456", ""));
    }

    #[test]
    fn pending_reset_blocks_repeat_reset_and_apply_until_receipt() {
        assert!(style_submission_allowed(false, false));
        assert!(!style_submission_allowed(false, true));
        assert!(!style_submission_allowed(true, false));
        assert!(!style_submission_allowed(true, true));
    }

    #[test]
    fn restore_theme_clears_appearance_without_changing_routing_or_shape() {
        let authored = Style {
            fill: Some("#303844".into()),
            stroke: Some("#82a3d5".into()),
            text_color: Some("#eeeeee".into()),
            font_family: Some("Custom font".into()),
            font_size: Some(17.0),
            stroke_width: Some(3.0),
            opacity: Some(0.5),
            routing: Some(RelationshipRouting::Curve),
            corner_radius: Some(4.0),
            dash_pattern: vec![3.0, 5.0],
        };
        let restored = edited_style(&authored, inherited_style_values()).expect("valid reset");
        assert_eq!(
            restored,
            Style {
                routing: authored.routing,
                corner_radius: authored.corner_radius,
                dash_pattern: authored.dash_pattern.clone(),
                ..Style::default()
            }
        );
        assert_eq!(
            edited_style(&restored, inherited_style_values()).expect("repeat reset"),
            restored
        );
        assert_eq!(authored.fill.as_deref(), Some("#303844"));
    }

    #[test]
    fn color_edits_are_real_overrides_and_blank_values_restore_inheritance() {
        let overrides = [("fill".into(), " #123456 ".into())].into();
        let authored = edited_style(&Style::default(), overrides).expect("valid color");
        assert_eq!(authored.fill.as_deref(), Some("#123456"));
        let restored = edited_style(&authored, inherited_style_values()).expect("valid reset");
        assert_eq!(restored.fill, None);
    }
    #[test]
    fn applied_values_acknowledge_normalized_persistence() {
        assert!(style_value_matches("font_size", "16.0", "16"));
        assert!(style_value_matches("stroke", " #ABCDEF ", "#ABCDEF"));
        assert!(!style_value_matches("font_size", "NaN", "NaN"));
        assert!(!style_value_matches("font_size", "17", "16"));
    }
    #[test]
    fn normalized_clean_input_detects_external_changes() {
        assert!(style_value_matches("font_size", "16.0", "16"));
        assert!(!style_value_matches("font_size", "16.0", "18"));
        assert!(!style_value_matches("font_size", "17", "16"));
    }

    #[test]
    fn inherited_numeric_style_and_non_finite_values() {
        assert_eq!(optional_number("  "), Ok(None));
        assert_eq!(optional_number(" 1.5 "), Ok(Some(1.5)));
        for text in ["NaN", "inf", "-inf", "invalid"] {
            assert!(optional_number(text).is_err());
        }
    }
}
