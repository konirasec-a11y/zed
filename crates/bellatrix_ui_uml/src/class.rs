//! Shared classifier labels, compartments and editing. The notation selects the silhouette.
use crate::{inline_edit::UmlInlineNameEditor, interface::UmlInterfaceBall};
use bellatrix_design_system::Palette;
use bellatrix_mde::{ClassifierPresentation, UmlHeader, UmlNotation, uml_header, uml_notation};
use gpui::{
    App, Entity, InteractiveElement, IntoElement, MouseButton, RenderOnce, SharedString, Window,
    div, prelude::*, px, rgb,
};
use std::rc::Rc;
use ui_input::InputField;

#[derive(Clone, Default)]
pub(crate) struct ViewAppearance(pub(crate) bellatrix_mde::Style);
impl ViewAppearance {
    fn color(&self, value: Option<&str>, fallback: u32) -> gpui::Rgba {
        value
            .and_then(|value| gpui::Rgba::try_from(value).ok())
            .unwrap_or_else(|| rgb(fallback))
            .opacity(self.0.opacity.unwrap_or(1.0) as f32)
    }
    pub(crate) fn fill(&self, fallback: u32) -> gpui::Rgba {
        self.color(self.0.fill.as_deref(), fallback)
    }
    pub(crate) fn stroke(&self, palette: Palette, selected: bool) -> gpui::Rgba {
        // Selection is editor chrome, not part of the persisted presentation style.
        if selected {
            rgb(palette.selection)
        } else {
            self.color(self.0.stroke.as_deref(), palette.diagram_outline(false))
        }
    }
    pub(crate) fn text(&self, fallback: u32) -> gpui::Rgba {
        self.color(self.0.text_color.as_deref(), fallback)
    }
    pub(crate) fn width(&self, default: f32, scale: f32, selected: bool) -> f32 {
        let width = self.0.stroke_width.map_or(default, |value| value as f32) * scale;
        if selected { width.max(2.0) } else { width }
    }
    pub(crate) fn font_size(&self, default: f32, scale: f32) -> f32 {
        self.0
            .font_size
            .map_or(default, |value| value as f32 * default / 12.0)
            * scale
    }
}

pub(crate) type ElementAction = Rc<dyn Fn(&str, &mut Window, &mut App)>;
pub(crate) type Action = Rc<dyn Fn(&mut Window, &mut App)>;
#[derive(Clone)]
pub(crate) struct MemberRow {
    pub(crate) id: Option<String>,
    pub(crate) title: SharedString,
}
#[derive(Clone)]
pub(crate) struct MemberInteractions {
    pub editing: Option<(String, Entity<InputField>)>,
    pub select: Option<ElementAction>,
    pub edit: Option<ElementAction>,
    pub confirm: Option<Action>,
    pub cancel: Option<Action>,
    pub scale: f32,
    pub appearance: ViewAppearance,
}

#[derive(IntoElement)]
pub struct UmlClassifier {
    pub(crate) class_id: Option<String>,
    pub(crate) title: SharedString,
    pub(crate) header: UmlHeader,
    pub(crate) presentation: ClassifierPresentation,
    pub(crate) interface_ball: bool,
    pub(crate) attributes: Vec<MemberRow>,
    pub(crate) operations: Vec<MemberRow>,
    pub(crate) receptions: Vec<MemberRow>,
    pub(crate) template_parameters: Vec<MemberRow>,
    pub(crate) selected: bool,
    pub(crate) editing: Option<(String, Entity<InputField>)>,
    pub(crate) select_member: Option<ElementAction>,
    pub(crate) edit_member: Option<ElementAction>,
    pub(crate) edit_title: Option<Action>,
    pub(crate) confirm_edit: Option<Action>,
    pub(crate) cancel_edit: Option<Action>,
    pub(crate) view_width: f64,
    pub(crate) view_height: f64,
    pub(crate) view_scale: f32,
    pub(crate) appearance: ViewAppearance,
}

impl UmlClassifier {
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            class_id: None,
            title: title.into(),
            header: uml_header("", None),
            presentation: ClassifierPresentation::default(),
            interface_ball: false,
            attributes: Vec::new(),
            operations: Vec::new(),
            receptions: Vec::new(),
            template_parameters: Vec::new(),
            selected: false,
            editing: None,
            select_member: None,
            edit_member: None,
            edit_title: None,
            confirm_edit: None,
            cancel_edit: None,
            view_width: 185.0,
            view_height: 112.0,
            view_scale: 1.0,
            appearance: ViewAppearance::default(),
        }
    }
    pub fn presentation(mut self, presentation: ClassifierPresentation) -> Self {
        self.presentation = presentation;
        self
    }
    pub fn view_style(mut self, style: &bellatrix_mde::Style) -> Self {
        self.appearance = ViewAppearance(style.clone());
        self
    }
    pub fn view_size(mut self, width: f64, height: f64, scale: f32) -> Self {
        if width.is_finite() && width > 0.0 {
            self.view_width = width;
        }
        if height.is_finite() && height > 0.0 {
            self.view_height = height;
        }
        if scale.is_finite() && scale > 0.0 {
            self.view_scale = scale;
        }
        self
    }
    pub fn model_kind(mut self, kind: &str) -> Self {
        self.interface_ball = uml_notation(kind) == UmlNotation::InterfaceBall;
        self.header.keyword = uml_header(kind, None).keyword;
        self
    }
    pub fn stereotype(mut self, value: impl Into<SharedString>) -> Self {
        let value = value.into();
        self.header.applied_stereotype = uml_header("", Some(value.as_ref())).applied_stereotype;
        self
    }
    pub fn class_id(mut self, id: impl Into<String>) -> Self {
        self.class_id = Some(id.into());
        self
    }
    pub fn attribute(mut self, value: impl Into<SharedString>) -> Self {
        self.attributes.push(MemberRow {
            id: None,
            title: value.into(),
        });
        self
    }
    pub fn operation(mut self, value: impl Into<SharedString>) -> Self {
        self.operations.push(MemberRow {
            id: None,
            title: value.into(),
        });
        self
    }
    pub fn attribute_with_id(
        mut self,
        id: impl Into<String>,
        title: impl Into<SharedString>,
    ) -> Self {
        self.attributes.push(MemberRow {
            id: Some(id.into()),
            title: title.into(),
        });
        self
    }
    pub fn operation_with_id(
        mut self,
        id: impl Into<String>,
        title: impl Into<SharedString>,
    ) -> Self {
        self.operations.push(MemberRow {
            id: Some(id.into()),
            title: title.into(),
        });
        self
    }
    pub fn template_parameter_with_id(
        mut self,
        id: impl Into<String>,
        title: impl Into<SharedString>,
    ) -> Self {
        self.template_parameters.push(MemberRow {
            id: Some(id.into()),
            title: title.into(),
        });
        self
    }
    pub fn reception_with_id(
        mut self,
        id: impl Into<String>,
        title: impl Into<SharedString>,
    ) -> Self {
        self.receptions.push(MemberRow {
            id: Some(id.into()),
            title: title.into(),
        });
        self
    }
    pub fn selected(mut self, value: bool) -> Self {
        self.selected = value;
        self
    }
    pub fn on_member_click(
        mut self,
        handler: impl Fn(&str, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.select_member = Some(Rc::new(handler));
        self
    }
    pub fn on_member_edit(
        mut self,
        handler: impl Fn(&str, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.edit_member = Some(Rc::new(handler));
        self
    }
    pub fn on_title_edit(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.edit_title = Some(Rc::new(handler));
        self
    }
    pub fn inline_edit(
        mut self,
        subject: impl Into<String>,
        field: Entity<InputField>,
        confirm: impl Fn(&mut Window, &mut App) + 'static,
        cancel: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.editing = Some((subject.into(), field));
        self.confirm_edit = Some(Rc::new(confirm));
        self.cancel_edit = Some(Rc::new(cancel));
        self
    }
    pub(crate) fn member_interactions(&self) -> MemberInteractions {
        MemberInteractions {
            editing: self.editing.clone(),
            select: self.select_member.clone(),
            edit: self.edit_member.clone(),
            confirm: self.confirm_edit.clone(),
            cancel: self.cancel_edit.clone(),
            scale: self.view_scale,
            appearance: self.appearance.clone(),
        }
    }
}

pub(crate) fn inline_field(
    field: Entity<InputField>,
    confirm: Option<Action>,
    cancel: Option<Action>,
) -> UmlInlineNameEditor {
    UmlInlineNameEditor::new(
        field,
        move |window, cx| {
            if let Some(handler) = &confirm {
                handler(window, cx);
            }
        },
        move |window, cx| {
            if let Some(handler) = &cancel {
                handler(window, cx);
            }
        },
    )
}

pub(crate) fn compartment(
    rows: Vec<MemberRow>,
    family: &'static str,
    interactions: MemberInteractions,
    palette: Palette,
    with_border: bool,
) -> gpui::AnyElement {
    let scale = interactions.scale;
    let appearance = interactions.appearance.clone();
    div()
        .w_full()
        .flex()
        .flex_col()
        .px(px(8.0 * scale))
        .py(px(4.0 * scale))
        .when(with_border, |element| {
            element
                .border_t(px(appearance.width(1.0, scale, false)))
                .border_color(appearance.color(appearance.0.stroke.as_deref(), palette.border))
        })
        .children(rows.into_iter().enumerate().map(move |(index, member)| {
            let select = interactions.select.clone();
            let edit = interactions.edit.clone();
            let field = interactions.editing.as_ref().and_then(|(subject, field)| {
                (member.id.as_deref() == Some(subject.as_str())).then(|| field.clone())
            });
            let inner = if let Some(field) = field {
                inline_field(
                    field,
                    interactions.confirm.clone(),
                    interactions.cancel.clone(),
                )
                .into_any_element()
            } else {
                div()
                    .w_full()
                    .min_w_0()
                    .child(member.title)
                    .into_any_element()
            };
            div()
                .id(format!("uml-{family}-{index}"))
                .w_full()
                .min_w_0()
                .min_h(px(20.0 * scale))
                .cursor_pointer()
                .text_size(px(appearance.font_size(12.0, scale)))
                .text_color(appearance.text(palette.text))
                .on_mouse_down(MouseButton::Left, move |event, window, cx| {
                    if let Some(id) = member.id.as_deref() {
                        if event.click_count >= 2 {
                            if let Some(handler) = &edit {
                                handler(id, window, cx);
                            }
                        } else if let Some(handler) = &select {
                            handler(id, window, cx);
                        }
                    }
                    cx.stop_propagation();
                })
                .child(inner)
        }))
        .into_any_element()
}

pub(crate) fn template_signature(
    rows: Vec<MemberRow>,
    body_width: f64,
    interactions: MemberInteractions,
    palette: Palette,
    selected: bool,
) -> Option<gpui::AnyElement> {
    let layout = bellatrix_mde::template_signature_layout(body_width, rows.len())?;
    let scale = interactions.scale;
    let appearance = interactions.appearance.clone();
    Some(
        div()
            .id("uml-template-signature")
            .absolute()
            .left(px(layout.left as f32 * scale))
            .top(px(layout.top as f32 * scale))
            .w(px(layout.width as f32 * scale))
            .h(px(layout.height as f32 * scale))
            .border(px(appearance.width(1.0, scale, selected)))
            .border_dashed()
            .border_color(appearance.stroke(palette, selected))
            .bg(appearance.fill(palette.panel))
            .px(px(6.0 * scale))
            .py(px(6.0 * scale))
            .flex()
            .flex_col()
            .overflow_hidden()
            .when_some(appearance.0.font_family.clone(), |element, family| {
                element.font_family(family)
            })
            .children(rows.into_iter().enumerate().map(move |(index, parameter)| {
                let select = interactions.select.clone();
                let edit = interactions.edit.clone();
                let field = interactions.editing.as_ref().and_then(|(subject, field)| {
                    (parameter.id.as_deref() == Some(subject.as_str())).then(|| field.clone())
                });
                let label = if let Some(field) = field {
                    inline_field(
                        field,
                        interactions.confirm.clone(),
                        interactions.cancel.clone(),
                    )
                    .into_any_element()
                } else {
                    div().child(parameter.title).into_any_element()
                };
                div()
                    .id(format!("uml-template-parameter-{index}"))
                    .h(px(20.0 * scale))
                    .flex_none()
                    .text_size(px(appearance.font_size(12.0, scale)))
                    .text_color(appearance.text(palette.text))
                    .whitespace_nowrap()
                    .cursor_pointer()
                    .on_mouse_down(MouseButton::Left, move |event, window, cx| {
                        if let Some(id) = parameter.id.as_deref() {
                            let handler = if event.click_count >= 2 {
                                edit.as_ref()
                            } else {
                                select.as_ref()
                            };
                            if let Some(handler) = handler {
                                handler(id, window, cx);
                            }
                        }
                        cx.stop_propagation();
                    })
                    .child(label)
            }))
            .into_any_element(),
    )
}

impl RenderOnce for UmlClassifier {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        if self.interface_ball {
            return UmlInterfaceBall::new(self).into_any_element();
        }
        let palette = Palette::current(cx);
        let appearance = self.appearance.clone();
        let line = appearance.stroke(palette, self.selected);
        let interactions = self.member_interactions();
        let scale = self.view_scale;
        let signature = template_signature(
            self.template_parameters,
            self.view_width,
            interactions.clone(),
            palette,
            self.selected,
        );
        let title_editor = self.editing.as_ref().and_then(|(subject, field)| {
            (self.class_id.as_deref() == Some(subject.as_str())).then(|| field.clone())
        });
        let title = match title_editor {
            Some(field) => inline_field(field, self.confirm_edit.clone(), self.cancel_edit.clone())
                .into_any_element(),
            None => div()
                .when(self.presentation.name_is_italic, |title| title.italic())
                .child(self.title)
                .into_any_element(),
        };
        let edit_title = self.edit_title.clone();
        let header_height = self.header.classifier_header_height();
        let keyword = self.header.keyword;
        let applied_stereotype = self.header.applied_stereotype;
        let body = div()
            .size_full()
            .flex()
            .flex_col()
            .border(px(appearance.width(1.0, scale, self.selected)))
            .border_color(line)
            .rounded(px(appearance.0.corner_radius.unwrap_or(2.0) as f32 * scale))
            .when_some(appearance.0.font_family.clone(), |element, family| {
                element.font_family(family)
            })
            .bg(appearance.fill(palette.panel))
            .overflow_hidden()
            .child(
                div()
                    .id("uml-classifier-title")
                    .min_h(px(header_height as f32 * scale))
                    .w_full()
                    .flex_none()
                    .px_1()
                    .flex()
                    .items_center()
                    .justify_center()
                    .bg(appearance.fill(palette.panel_alt))
                    .border_b(px(appearance.width(1.0, scale, false)))
                    .border_color(line)
                    .text_size(px(appearance.font_size(12.0, scale)))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(appearance.text(palette.text))
                    .on_mouse_down(MouseButton::Left, move |event, window, cx| {
                        if event.click_count >= 2 {
                            if let Some(handler) = &edit_title {
                                handler(window, cx);
                            }
                            cx.stop_propagation();
                        }
                    })
                    .child(
                        div()
                            .w_full()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .items_center()
                            .justify_center()
                            .whitespace_normal()
                            .when_some(keyword, |header, value| {
                                header.child(
                                    div()
                                        .w_full()
                                        .text_center()
                                        .text_size(px(appearance.font_size(10.0, scale)))
                                        .text_color(appearance.text(palette.muted))
                                        .child(value),
                                )
                            })
                            .when_some(applied_stereotype, |header, value| {
                                header.child(
                                    div()
                                        .w_full()
                                        .text_center()
                                        .text_size(px(appearance.font_size(10.0, scale)))
                                        .text_color(appearance.text(palette.muted))
                                        .child(value),
                                )
                            })
                            .child(title),
                    ),
            )
            .when(!self.attributes.is_empty(), |element| {
                element.child(compartment(
                    self.attributes,
                    "attribute",
                    interactions.clone(),
                    palette,
                    false,
                ))
            })
            .when(!self.operations.is_empty(), |element| {
                element.child(compartment(
                    self.operations,
                    "operation",
                    interactions.clone(),
                    palette,
                    true,
                ))
            })
            .when(!self.receptions.is_empty(), |element| {
                element.child(compartment(
                    self.receptions,
                    "reception",
                    interactions,
                    palette,
                    true,
                ))
            })
            .into_any_element();
        div()
            .size_full()
            .relative()
            .child(body)
            .when(self.presentation.double_sided, |element| {
                element.child(
                    div()
                        .absolute()
                        .top_0()
                        .bottom_0()
                        .left(px(5.0 * scale))
                        .right(px(5.0 * scale))
                        .border_l(px(appearance.width(1.0, scale, self.selected)))
                        .border_r(px(appearance.width(1.0, scale, self.selected)))
                        .border_color(line),
                )
            })
            .when_some(signature, |element, signature| element.child(signature))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semantic_presentation_is_independent_of_style_and_notation_builder_order() {
        let presentation = ClassifierPresentation {
            name_is_italic: true,
            double_sided: true,
        };
        let style = bellatrix_mde::Style {
            stroke_width: Some(3.0),
            font_size: Some(18.0),
            ..Default::default()
        };
        let classifier = UmlClassifier::new("Renamed")
            .presentation(presentation)
            .model_kind("uml.component")
            .view_style(&style)
            .stereotype("Service")
            .view_size(200.0, 130.0, 2.0);
        assert_eq!(classifier.presentation, presentation);
        assert_eq!(classifier.title.as_ref(), "Renamed");
        assert_eq!(classifier.appearance.0.stroke_width, Some(3.0));
        assert_eq!(classifier.appearance.0.font_size, Some(18.0));
        assert_eq!(classifier.view_width, 200.0);
        assert_eq!(classifier.view_height, 130.0);
    }

    #[test]
    fn template_rows_keep_semantic_ids_and_authoritative_labels_without_invented_syntax() {
        let classifier = UmlClassifier::new("Container")
            .template_parameter_with_id("formal-parameter", "Item: Classifier = DefaultItem");
        let parameter = classifier
            .template_parameters
            .first()
            .expect("formal parameter");
        assert_eq!(parameter.id.as_deref(), Some("formal-parameter"));
        assert_eq!(parameter.title.as_ref(), "Item: Classifier = DefaultItem");
        assert!(classifier.attributes.is_empty());
        assert!(classifier.operations.is_empty());
    }

    #[test]
    fn view_style_defaults_follow_theme_roles_and_scale() {
        let appearance = ViewAppearance::default();
        for mode in [
            bellatrix_design_system::ThemeMode::Dark,
            bellatrix_design_system::ThemeMode::Light,
        ] {
            let palette = Palette::for_mode(mode);
            assert_eq!(appearance.fill(palette.panel), rgb(palette.panel));
            assert_eq!(appearance.text(palette.text), rgb(palette.text));
            assert_eq!(
                appearance.stroke(palette, false),
                rgb(palette.diagram_outline(false))
            );
            assert_eq!(appearance.stroke(palette, true), rgb(palette.selection));
        }
        assert_eq!(appearance.font_size(12.0, 2.0), 24.0);
        assert_eq!(appearance.width(1.0, 2.0, false), 2.0);
    }

    #[test]
    fn style_overrides_are_cloned_per_view_and_selection_survives_transparency() {
        let mut style = bellatrix_mde::Style {
            fill: Some("#123456".into()),
            stroke: Some("#abcdef".into()),
            text_color: Some("#fedcba".into()),
            stroke_width: Some(0.0),
            font_size: Some(18.0),
            opacity: Some(0.0),
            ..Default::default()
        };
        let styled = UmlClassifier::new("Styled").view_style(&style);
        style.fill = None;
        let default = UmlClassifier::new("Default");
        assert_eq!(styled.appearance.0.fill.as_deref(), Some("#123456"));
        assert!(default.appearance.0.fill.is_none());
        let appearance = styled.appearance;
        let palette = Palette::for_mode(bellatrix_design_system::ThemeMode::Dark);
        assert_eq!(appearance.fill(palette.panel).a, 0.0);
        assert_eq!(appearance.text(palette.text).a, 0.0);
        assert_eq!(appearance.stroke(palette, false).a, 0.0);
        assert_eq!(appearance.stroke(palette, true), rgb(palette.selection));
        assert_eq!(appearance.width(1.0, 0.5, true), 2.0);
        assert_eq!(appearance.font_size(12.0, 2.0), 36.0);
        assert_eq!(appearance.font_size(10.0, 2.0), 30.0);
    }

    #[test]
    fn native_color_alpha_is_composed_with_view_opacity() {
        let appearance = ViewAppearance(bellatrix_mde::Style {
            fill: Some("#12345680".into()),
            opacity: Some(0.5),
            ..Default::default()
        });
        let color = appearance.fill(0xffffff);
        assert!((color.a - 128.0 / 255.0 * 0.5).abs() < 0.0001);
        assert!((color.r - 18.0 / 255.0).abs() < 0.0001);
    }

    #[test]
    fn model_keyword_and_application_label_are_independent_in_either_builder_order() {
        for kind in ["uml.component", "uml.artifact", "uml.stereotype"] {
            let expected = uml_header(kind, Some("Service"));
            let first = UmlClassifier::new("Example")
                .model_kind(kind)
                .stereotype("«Service»");
            let second = UmlClassifier::new("Example")
                .stereotype("<<Service>>")
                .model_kind(kind);
            assert_eq!(first.header, expected);
            assert_eq!(second.header, expected);
        }
    }
}
