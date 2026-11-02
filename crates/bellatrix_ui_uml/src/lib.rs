//! Native Zed GPUI UML class view; presentation independent of MDE storage.
//! Member rows may carry semantic IDs and forward click selection to the host.
use bellatrix_design_system::Palette;
use gpui::{
    App, InteractiveElement, IntoElement, MouseButton, RenderOnce, SharedString,
    StatefulInteractiveElement, Window, div, prelude::*, px, rgb,
};
use std::rc::Rc;

type MemberSelected = Rc<dyn Fn(&str, &mut Window, &mut App)>;

#[derive(IntoElement)]
pub struct UmlClassCard {
    title: SharedString,
    attributes: Vec<(Option<String>, SharedString)>,
    operations: Vec<(Option<String>, SharedString)>,
    selected: bool,
    on_member_click: Option<MemberSelected>,
}

impl UmlClassCard {
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            attributes: Vec::new(),
            operations: Vec::new(),
            selected: false,
            on_member_click: None,
        }
    }

    pub fn attribute(mut self, value: impl Into<SharedString>) -> Self {
        self.attributes.push((None, value.into()));
        self
    }
    pub fn operation(mut self, value: impl Into<SharedString>) -> Self {
        self.operations.push((None, value.into()));
        self
    }

    /// A semantic child has its own Inspector and contextual Quick Edit.
    pub fn attribute_with_id(mut self, id: impl Into<String>, title: impl Into<SharedString>) -> Self {
        self.attributes.push((Some(id.into()), title.into()));
        self
    }
    pub fn operation_with_id(mut self, id: impl Into<String>, title: impl Into<SharedString>) -> Self {
        self.operations.push((Some(id.into()), title.into()));
        self
    }
    pub fn on_member_click(mut self, handler: impl Fn(&str, &mut Window, &mut App) + 'static) -> Self {
        self.on_member_click = Some(Rc::new(handler));
        self
    }

    pub fn selected(mut self, value: bool) -> Self {
        self.selected = value;
        self
    }
}

impl RenderOnce for UmlClassCard {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = Palette::current(cx);
        let line = rgb(if self.selected { p.selection } else { p.border });
        let select_attr = self.on_member_click.clone();
        let select_op = self.on_member_click.clone();
        div()
            .size_full().flex().flex_col()
            .border_1().border_color(line)
            .rounded_sm().bg(rgb(p.panel)).overflow_hidden()
            .child(
                div().h(px(34.0)).w_full().flex_none().px_3()
                    .flex().items_center().justify_center()
                    .bg(rgb(p.panel_alt))
                    .border_b_1().border_color(line)
                    .text_size(px(12.0))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(rgb(p.text))
                    .child(self.title),
            )
            .when(!self.attributes.is_empty(), |this| {
                this.child(
                    div().w_full().px_3().py_2()
                        .flex().flex_col().gap_1()
                        .children(self.attributes.into_iter().enumerate().map(move |(idx,(id,label))| {
                            let on_click = select_attr.clone();
                            div()
                                .id(format!("bellatrix-uml-attribute-{idx}"))
                                .w_full().cursor_pointer()
                                .text_size(px(12.0)).text_color(rgb(p.muted))
                                .on_mouse_down(MouseButton::Left, |_,_,cx| cx.stop_propagation())
                                .on_click(move |_,window,cx| {
                                    if let (Some(id),Some(f)) = (id.as_deref(),on_click.as_ref()) {
                                        f(id,window,cx);
                                    }
                                    cx.stop_propagation();
                                })
                                .child(label)
                        })),
                )
            })
            .when(!self.operations.is_empty(), |this| {
                this.child(
                    div().w_full().px_3().py_2()
                        .flex().flex_col().gap_1()
                        .border_t_1().border_color(line)
                        .children(self.operations.into_iter().enumerate().map(move |(idx,(id,label))| {
                            let on_click = select_op.clone();
                            div()
                                .id(format!("bellatrix-uml-operation-{idx}"))
                                .w_full().cursor_pointer()
                                .text_size(px(12.0)).text_color(rgb(p.muted))
                                .on_mouse_down(MouseButton::Left, |_,_,cx| cx.stop_propagation())
                                .on_click(move |_,window,cx| {
                                    if let (Some(id),Some(f)) = (id.as_deref(),on_click.as_ref()) {
                                        f(id,window,cx);
                                    }
                                    cx.stop_propagation();
                                })
                                .child(label)
                        })),
                )
            })
    }
}
