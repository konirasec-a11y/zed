//! UML InterfaceView lollipop notation from the StarUML reference.
//! A bare Interface is a circle above its name; owned members extend below
//! as compartments. Semantic identity, gestures and updates remain in MDE.
use crate::{
    class::{Action, ElementAction, MemberRow, UmlClassCard},
    inline_edit::UmlInlineNameEditor,
};
use bellatrix_design_system::Palette;
use gpui::{
    App, Entity, InteractiveElement, IntoElement, MouseButton, RenderOnce,
    Window, div, prelude::*, px, rgb,
};
use ui_input::InputField;

#[derive(IntoElement)]
pub struct UmlInterfaceBall {
    classifier: UmlClassCard,
}
impl UmlInterfaceBall {
    pub(crate) fn new(classifier: UmlClassCard) -> Self { Self { classifier } }
}
fn editable_name(
    field: Entity<InputField>, confirm:Option<Action>, cancel:Option<Action>,
) -> UmlInlineNameEditor {
    UmlInlineNameEditor::new(
        field,
        move |window,cx| { if let Some(handler)=&confirm { handler(window,cx); } },
        move |window,cx| { if let Some(handler)=&cancel { handler(window,cx); } },
    )
}
fn member_rows(
    members:Vec<MemberRow>, family:&'static str,
    editing:Option<(String,Entity<InputField>)>,
    select:Option<ElementAction>, edit:Option<ElementAction>,
    confirm:Option<Action>,cancel:Option<Action>,p:Palette,
) -> gpui::AnyElement {
    div().w_full().flex().flex_col().border_t_1().border_color(rgb(p.border))
        .py_1().px_2()
        .children(members.into_iter().enumerate().map(move |(index,member)| {
            let selecting=select.clone();
            let editing_action=edit.clone();
            let inline=editing.as_ref().and_then(|(id,field)|
                (member.id.as_deref()==Some(id.as_str())).then(||field.clone()));
            let child:gpui::AnyElement=if let Some(field)=inline {
                editable_name(field,confirm.clone(),cancel.clone()).into_any_element()
            } else {
                div().child(member.title).into_any_element()
            };
            div().id(format!("uml-interface-{family}-{index}"))
                .min_h(px(20.0)).w_full().cursor_pointer()
                .text_size(px(11.0)).text_color(rgb(p.text))
                .on_mouse_down(MouseButton::Left,move |event,window,cx| {
                    if let Some(id)=member.id.as_deref() {
                        if event.click_count>=2 {
                            if let Some(action)=&editing_action {action(id,window,cx);}
                        } else if let Some(action)=&selecting {action(id,window,cx);}
                    }
                    cx.stop_propagation();
                })
                .child(child)
        }))
        .into_any_element()
}

impl RenderOnce for UmlInterfaceBall {
    fn render(self,_:&mut Window,cx:&mut App)->impl IntoElement {
        let card=self.classifier;
        let p=Palette::current(cx);
        let circle_stroke=rgb(if card.selected {p.selection}else{p.text});
        let title_editor=card.editing.as_ref().and_then(|(subject,field)|
            (card.class_id.as_deref()==Some(subject.as_str())).then(||field.clone()));
        let title:gpui::AnyElement=match title_editor {
            Some(field)=>editable_name(field,card.confirm_edit.clone(),card.cancel_edit.clone())
                .into_any_element(),
            None=>div().text_size(px(12.0)).text_color(rgb(p.text))
                .child(card.title).into_any_element(),
        };
        let edit_title=card.edit_title.clone();
        let stereotype=card.stereotype.clone();
        let selected=card.selected;
        let body=div().size_full().flex().flex_col().items_center()
            .when(selected,|e|e.border_1().border_color(rgb(p.selection)))
            .child(
                div().pt_2().pb_1().w_full().flex().flex_col().items_center()
                    .child(div()
                        .id("uml-interface-ball")
                        .w(px(28.0)).h(px(28.0)).rounded_full()
                        .border_1().border_color(circle_stroke).bg(rgb(p.canvas)))
                    .when_some(stereotype,|header,stereotype|header.child(
                        div().text_size(px(10.0)).text_color(rgb(p.muted))
                            .child(stereotype)
                    ))
                    .child(
                        div().id("uml-interface-name")
                            .w_full().min_h(px(22.0))
                            .px_1().flex().items_center().justify_center()
                            .cursor_pointer()
                            .on_mouse_down(MouseButton::Left,move |event,window,cx| {
                                if event.click_count>=2 {
                                    if let Some(action)=&edit_title { action(window,cx); }
                                    cx.stop_propagation();
                                }
                            })
                            .child(title)
                    ),
            );
        body.when(!card.attributes.is_empty(),|e|e.child(member_rows(
                card.attributes,"attribute",card.editing.clone(),card.select_member.clone(),
                card.edit_member.clone(),card.confirm_edit.clone(),card.cancel_edit.clone(),p,
            )))
            .when(!card.operations.is_empty(),|e|e.child(member_rows(
                card.operations,"operation",card.editing.clone(),card.select_member.clone(),
                card.edit_member.clone(),card.confirm_edit.clone(),card.cancel_edit.clone(),p,
            )))
            .when(!card.receptions.is_empty(),|e|e.child(member_rows(
                card.receptions,"reception",card.editing,
                card.select_member,card.edit_member,
                card.confirm_edit,card.cancel_edit,p,
            )))
    }
}
