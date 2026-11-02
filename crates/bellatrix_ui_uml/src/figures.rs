//! Specialized UML shapes. These are Views, not semantic Elements or Cordis commands.
//! The class/interface/enum classifier notation remains in `class.rs`.
use crate::inline_edit::UmlInlineNameEditor;
use bellatrix_design_system::Palette;
use gpui::{
    App, Entity, InteractiveElement, IntoElement, MouseButton, RenderOnce,
    SharedString, Window, div, prelude::*, px, rgb, svg,
};
use std::rc::Rc;
use ui_input::InputField;

type Action = Rc<dyn Fn(&mut Window, &mut App)>;

#[derive(IntoElement)]
pub struct UmlFigure {
    kind: String,
    name: SharedString,
    selected: bool,
    edit: Option<Action>,
    editor: Option<(Entity<InputField>, Action, Action)>,
}
impl UmlFigure {
    pub fn new(kind: impl Into<String>, name: impl Into<SharedString>) -> Self {
        Self { kind:kind.into(),name:name.into(),selected:false,edit:None,editor:None }
    }
    pub fn selected(mut self,selected:bool)->Self { self.selected=selected;self }
    pub fn on_title_edit(mut self,handler:impl Fn(&mut Window,&mut App)+'static)->Self {
        self.edit=Some(Rc::new(handler));self
    }
    pub fn inline_edit(
        mut self,field:Entity<InputField>,
        confirm:impl Fn(&mut Window,&mut App)+'static,
        cancel:impl Fn(&mut Window,&mut App)+'static,
    )->Self {
        self.editor=Some((field,Rc::new(confirm),Rc::new(cancel)));
        self
    }
}
impl RenderOnce for UmlFigure {
    fn render(self,_:&mut Window,cx:&mut App)->impl IntoElement {
        let p=Palette::current(cx);
        let border=rgb(if self.selected {p.selection} else {p.border});
        let title:gpui::AnyElement=if let Some((field,confirm,cancel))=self.editor {
            UmlInlineNameEditor::new(
                field,move |w,cx| confirm(w,cx),move |w,cx| cancel(w,cx)
            ).into_any_element()
        } else {
            div().child(self.name).into_any_element()
        };
        let edit=self.edit.clone();
        let title_area = div()
            .id("uml-figure-title")
            .text_size(px(12.0)).text_color(rgb(p.text))
            .font_weight(gpui::FontWeight::SEMIBOLD)
            .on_mouse_down(MouseButton::Left,move |event,window,cx| {
                if event.click_count>=2 {
                    if let Some(callback)=&edit { callback(window,cx); }
                    cx.stop_propagation();
                }
            })
            .child(title);
        let base=div().size_full().relative()
            .border_1().border_color(border).bg(rgb(p.panel)).overflow_hidden();
        match self.kind.as_str() {
            "uml.package" | "uml.model" | "uml.subsystem" => {
                let stereotype=match self.kind.as_str() {
                    "uml.model" => Some("«model»"),"uml.subsystem"=>Some("«subsystem»"),_=>None,
                };
                base.flex().flex_col()
                    // Tab silhouette plus a large empty contents compartment.
                    .child(div().w(px(87.0)).h(px(16.0)).flex_none()
                        .border_r_1().border_b_1().border_color(border)
                        .bg(rgb(p.panel_alt)))
                    .child(div().w_full().px_2().py_1().flex().flex_col()
                        .when_some(stereotype,|this,s|this.child(
                            div().text_size(px(10.0)).text_color(rgb(p.muted)).child(s)))
                        .child(title_area))
                    .into_any_element()
            }
            "uml.frame" => {
                base.flex().flex_col().bg(rgb(p.canvas))
                    .child(div().w(px(142.0)).h(px(30.0)).flex_none()
                        .border_r_1().border_b_1().border_color(border)
                        .px_2().flex().items_center().bg(rgb(p.panel_alt))
                        .child(title_area))
                    .into_any_element()
            }
            "uml.comment" => {
                const FOLD:&[u8]=br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M4 2H16L22 8V22H4Z M16 2V8H22" fill="none" stroke="currentColor" stroke-width="1.5"/></svg>"#;
                base.flex().flex_col().p_2().gap_2()
                    .child(div().w(px(22.0)).h(px(22.0))
                        .child(svg().data(FOLD).w_full().h_full().text_color(rgb(p.muted))))
                    .child(title_area)
                    .into_any_element()
            }
            "uml.nary_association" => {
                const DIAMOND:&[u8]=br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><polygon points="50,4 96,50 50,96 4,50" fill="none" stroke="currentColor" stroke-width="3"/></svg>"#;
                base.bg(rgb(p.canvas)).flex().flex_col().items_center().justify_center()
                    .child(svg().data(DIAMOND).w(px(44.0)).h(px(44.0))
                        .text_color(rgb(p.text)))
                    .child(div().text_xs().text_color(rgb(p.muted)).child(title_area))
                    .into_any_element()
            }
            _ => base.flex().items_center().justify_center()
                .child(title_area).into_any_element(),
        }
    }
}
