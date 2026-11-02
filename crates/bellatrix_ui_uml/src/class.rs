//! Shared classifier labels, compartments and editing. The notation selects the silhouette.
use crate::{inline_edit::UmlInlineNameEditor, interface::UmlInterfaceBall};
use bellatrix_design_system::Palette;
use bellatrix_mde::{UmlHeader, UmlNotation, uml_header, uml_notation};
use gpui::{
    App, Entity, InteractiveElement, IntoElement, MouseButton, RenderOnce, SharedString, Window,
    div, prelude::*, px, rgb,
};
use std::rc::Rc;
use ui_input::InputField;

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
}

#[derive(IntoElement)]
pub struct UmlClassifier {
    pub(crate) class_id: Option<String>,
    pub(crate) title: SharedString,
    pub(crate) header: UmlHeader,
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
}

impl UmlClassifier {
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            class_id: None,
            title: title.into(),
            header: uml_header("", None),
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
        }
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
    div()
        .w_full()
        .flex()
        .flex_col()
        .px(px(8.0 * scale))
        .py(px(4.0 * scale))
        .when(with_border, |element| {
            element
                .border_t(px(scale))
                .border_color(rgb(palette.border))
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
                .text_size(px(12.0 * scale))
                .text_color(rgb(palette.text))
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

impl RenderOnce for UmlClassifier {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        if self.interface_ball {
            return UmlInterfaceBall::new(self).into_any_element();
        }
        let palette = Palette::current(cx);
        let line = rgb(if self.selected {
            palette.selection
        } else {
            palette.border
        });
        let interactions = self.member_interactions();
        let scale = self.view_scale;
        let title_editor = self.editing.as_ref().and_then(|(subject, field)| {
            (self.class_id.as_deref() == Some(subject.as_str())).then(|| field.clone())
        });
        let title = match title_editor {
            Some(field) => inline_field(field, self.confirm_edit.clone(), self.cancel_edit.clone())
                .into_any_element(),
            None => div().child(self.title).into_any_element(),
        };
        let edit_title = self.edit_title.clone();
        let header_height = self.header.classifier_header_height();
        let keyword = self.header.keyword;
        let applied_stereotype = self.header.applied_stereotype;
        div()
            .size_full()
            .flex()
            .flex_col()
            .border_1()
            .border_color(line)
            .rounded_sm()
            .bg(rgb(palette.panel))
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
                    .bg(rgb(palette.panel_alt))
                    .border_b_1()
                    .border_color(line)
                    .text_size(px(12.0 * scale))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(rgb(palette.text))
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
                                        .text_size(px(10.0 * scale))
                                        .text_color(rgb(palette.muted))
                                        .child(value),
                                )
                            })
                            .when_some(applied_stereotype, |header, value| {
                                header.child(
                                    div()
                                        .w_full()
                                        .text_center()
                                        .text_size(px(10.0 * scale))
                                        .text_color(rgb(palette.muted))
                                        .child(value),
                                )
                            })
                            .child(title),
                    ),
            )
            .when(!self.template_parameters.is_empty(), |element| {
                element.child(compartment(
                    self.template_parameters,
                    "template-parameter",
                    interactions.clone(),
                    palette,
                    false,
                ))
            })
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
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
