//! UML classifier view: class/interface heading and owned member compartments.
//! Supports selection and inline editing but delegates all semantic actions,
//! validation and persistence to the host.
use crate::inline_edit::UmlInlineNameEditor;
use bellatrix_design_system::Palette;
use gpui::{
    App, Entity, InteractiveElement, IntoElement, MouseButton, RenderOnce,
    SharedString, Window, div, prelude::*, px, rgb,
};
use std::rc::Rc;
use ui_input::InputField;

type ElementAction = Rc<dyn Fn(&str, &mut Window, &mut App)>;
type Action = Rc<dyn Fn(&mut Window, &mut App)>;

#[derive(Clone)]
struct MemberRow {
    id: Option<String>,
    title: SharedString,
}

#[derive(IntoElement)]
pub struct UmlClassCard {
    class_id: Option<String>,
    title: SharedString,
    attributes: Vec<MemberRow>,
    operations: Vec<MemberRow>,
    selected: bool,
    editing: Option<(String, Entity<InputField>)>,
    select_member: Option<ElementAction>,
    edit_member: Option<ElementAction>,
    edit_title: Option<Action>,
    confirm_edit: Option<Action>,
    cancel_edit: Option<Action>,
}

impl UmlClassCard {
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            class_id: None,
            title: title.into(),
            attributes: Vec::new(),
            operations: Vec::new(),
            selected: false,
            editing: None,
            select_member: None,
            edit_member: None,
            edit_title: None,
            confirm_edit: None,
            cancel_edit: None,
        }
    }

    pub fn class_id(mut self, id: impl Into<String>) -> Self {
        self.class_id = Some(id.into());
        self
    }
    pub fn attribute(mut self, value: impl Into<SharedString>) -> Self {
        self.attributes.push(MemberRow { id: None, title: value.into() });
        self
    }
    pub fn operation(mut self, value: impl Into<SharedString>) -> Self {
        self.operations.push(MemberRow { id: None, title: value.into() });
        self
    }
    pub fn attribute_with_id(
        mut self, id: impl Into<String>, title: impl Into<SharedString>,
    ) -> Self {
        self.attributes.push(MemberRow { id: Some(id.into()), title: title.into() });
        self
    }
    pub fn operation_with_id(
        mut self, id: impl Into<String>, title: impl Into<SharedString>,
    ) -> Self {
        self.operations.push(MemberRow { id: Some(id.into()), title: title.into() });
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
    /// First click selects an owned UML member without moving its classifier.
    pub fn on_member_click(
        mut self, handler: impl Fn(&str, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.select_member = Some(Rc::new(handler));
        self
    }
    /// Double-click on an owned attribute/operation opens its inline editor.
    pub fn on_member_edit(
        mut self, handler: impl Fn(&str, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.edit_member = Some(Rc::new(handler));
        self
    }
    /// Double-click on classifier title to edit directly over the label.
    pub fn on_title_edit(
        mut self, handler: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.edit_title = Some(Rc::new(handler));
        self
    }
    /// The host owns the editor's focus and lifetime, the UI owns placement.
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
}

fn inline_field(
    field: Entity<InputField>,
    confirm: Option<Action>,
    cancel: Option<Action>,
) -> UmlInlineNameEditor {
    UmlInlineNameEditor::new(
        field,
        move |window, cx| {
            if let Some(handler) = &confirm { handler(window, cx); }
        },
        move |window, cx| {
            if let Some(handler) = &cancel { handler(window, cx); }
        },
    )
}

fn compartment(
    rows: Vec<MemberRow>,
    compartment_name: &'static str,
    editing: Option<(String, Entity<InputField>)>,
    select: Option<ElementAction>,
    edit: Option<ElementAction>,
    confirm: Option<Action>,
    cancel: Option<Action>,
    p: Palette,
    with_border: bool,
) -> impl IntoElement {
    let line = rgb(p.border);
    div()
        .w_full().flex().flex_col()
        .px_2().py_1().gap_0p5()
        .when(with_border, |el| el.border_t_1().border_color(line))
        .children(rows.into_iter().enumerate().map(move |(index, member)| {
            let selected = select.clone();
            let edit_member = edit.clone();
            let editor = editing.as_ref().and_then(|(subject, field)| {
                (member.id.as_deref() == Some(subject.as_str())).then(|| field.clone())
            });
            let inner = if let Some(field) = editor {
                inline_field(field, confirm.clone(), cancel.clone()).into_any_element()
            } else {
                div().w_full().child(member.title).into_any_element()
            };
            div()
                .id(format!("uml-{compartment_name}-{index}"))
                .w_full().min_h(px(22.0))
                .cursor_pointer()
                .text_size(px(12.0))
                .text_color(rgb(p.muted))
                .on_mouse_down(MouseButton::Left, move |event, window, cx| {
                    if let Some(id) = member.id.as_deref() {
                        if event.click_count >= 2 {
                            if let Some(action) = &edit_member { action(id, window, cx); }
                        } else if let Some(action) = &selected {
                            action(id, window, cx);
                        }
                    }
                    cx.stop_propagation();
                })
                .child(inner)
        }))
}

impl RenderOnce for UmlClassCard {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let palette = Palette::current(cx);
        let line = rgb(if self.selected { palette.selection } else { palette.border });
        let title_editor = self.editing.as_ref().and_then(|(subject, field)| {
            (self.class_id.as_deref() == Some(subject.as_str())).then(|| field.clone())
        });
        let title: gpui::AnyElement = match title_editor {
            Some(field) => inline_field(
                field, self.confirm_edit.clone(), self.cancel_edit.clone(),
            ).into_any_element(),
            None => div().child(self.title).into_any_element(),
        };
        let edit_title = self.edit_title.clone();
        div()
            .size_full()
            .flex().flex_col()
            .border_1().border_color(line)
            .rounded_sm()
            .bg(rgb(palette.panel))
            .overflow_hidden()
            .child(
                div()
                    .id("uml-class-title")
                    .h(px(34.0)).w_full().flex_none()
                    .px_1()
                    .flex().items_center().justify_center()
                    .bg(rgb(palette.panel_alt))
                    .border_b_1().border_color(line)
                    .text_size(px(12.0))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(rgb(palette.text))
                    .on_mouse_down(MouseButton::Left, move |event, window, cx| {
                        if event.click_count >= 2 {
                            if let Some(action) = &edit_title { action(window, cx); }
                            cx.stop_propagation();
                        }
                    })
                    .child(title),
            )
            .when(!self.attributes.is_empty(), |el| {
                el.child(compartment(
                    self.attributes, "attribute", self.editing.clone(),
                    self.select_member.clone(), self.edit_member.clone(),
                    self.confirm_edit.clone(), self.cancel_edit.clone(), palette, false,
                ))
            })
            .when(!self.operations.is_empty(), |el| {
                el.child(compartment(
                    self.operations, "operation", self.editing,
                    self.select_member, self.edit_member,
                    self.confirm_edit, self.cancel_edit, palette, true,
                ))
            })
    }
}
