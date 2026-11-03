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
    let captured_inputs = inputs.clone();
    let target_item = item.clone();
    let diagram = diagram_id.to_owned();
    let view_id = view.id.clone();
    rows.push(
        div()
            .px_3()
            .child(
                Button::new("mde-style-apply", "Apply style")
                    .disabled(conflict)
                    .on_click(move |_, _, cx| {
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
                                item.apply_view_style(
                                    &diagram,
                                    &view_id,
                                    expected_revision,
                                    values,
                                    cx,
                                )
                            });
                        }
                    }),
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
        let Some(view) = snapshot
            .section
            .diagrams
            .iter()
            .find(|d| d.id == diagram_id)
            .and_then(|d| d.views.iter().find(|v| v.id == view_id))
        else {
            return;
        };
        let mut style = view.style.clone();
        for (key, text) in values {
            let text = text.trim();
            let string = (!text.is_empty()).then(|| text.to_owned());
            match key.as_str() {
                "fill" => style.fill = string,
                "stroke" => style.stroke = string,
                "text_color" => style.text_color = string,
                "font_family" => style.font_family = string,
                "font_size" | "stroke_width" | "opacity" => {
                    let value = match optional_number(text) {
                        Ok(value) => value,
                        Err(error) => {
                            self.feedback = error.into();
                            cx.notify();
                            return;
                        }
                    };
                    match key.as_str() {
                        "font_size" => style.font_size = value,
                        "stroke_width" => style.stroke_width = value,
                        _ => style.opacity = value,
                    }
                }
                _ => {}
            }
        }
        self.submit(
            MdeUiCommand::SetViewStyle(ModelingSetViewStyleRequested {
                project_id: self.project_id().clone(),
                expected_revision,
                diagram_id: diagram_id.into(),
                view_id: view_id.into(),
                style,
            }),
            cx,
        );
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
    use super::{optional_number, style_value_matches};
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
