//! Circular notation of a classifier; shared members and editing live in class.rs.
use crate::class::{UmlClassifier, compartment, inline_field};
use bellatrix_design_system::Palette;
use bellatrix_mde::{ClassifierContents, interface_layout};
use gpui::{
    App, InteractiveElement, IntoElement, MouseButton, RenderOnce, Window, div, prelude::*, px, rgb,
};

#[derive(IntoElement)]
pub struct UmlInterfaceBall {
    classifier: UmlClassifier,
}
impl UmlInterfaceBall {
    pub(crate) fn new(classifier: UmlClassifier) -> Self {
        Self { classifier }
    }
}
impl RenderOnce for UmlInterfaceBall {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let classifier = self.classifier;
        let palette = Palette::current(cx);
        let scale = classifier.view_scale;
        let interactions = classifier.member_interactions();
        let contents = ClassifierContents {
            attributes: classifier.attributes.len(),
            operations: classifier.operations.len(),
            receptions: classifier.receptions.len(),
            template_parameters: classifier.template_parameters.len(),
            stereotype: classifier.header.applied_stereotype.is_some(),
        };
        let layout = interface_layout(classifier.view_width, classifier.view_height, contents);
        let units = |value: f64| px(value as f32 * scale);
        let title_editor = classifier.editing.as_ref().and_then(|(subject, field)| {
            (classifier.class_id.as_deref() == Some(subject.as_str())).then(|| field.clone())
        });
        let title = match title_editor {
            Some(field) => inline_field(
                field,
                classifier.confirm_edit.clone(),
                classifier.cancel_edit.clone(),
            )
            .into_any_element(),
            None => div()
                .text_size(px(12.0 * scale))
                .text_color(rgb(palette.text))
                .child(classifier.title)
                .into_any_element(),
        };
        let edit_title = classifier.edit_title;
        let stroke = rgb(if classifier.selected {
            palette.selection
        } else {
            palette.text
        });
        div()
            .size_full()
            .relative()
            .when(classifier.selected, |root| {
                root.child(
                    div()
                        .absolute()
                        .size_full()
                        .border_1()
                        .border_color(rgb(palette.selection)),
                )
            })
            .child(
                div()
                    .id("uml-interface-ball")
                    .absolute()
                    .left(units(layout.center.x - layout.radius))
                    .top(units(layout.center.y - layout.radius))
                    .w(units(layout.radius * 2.0))
                    .h(units(layout.radius * 2.0))
                    .rounded_full()
                    .border_1()
                    .border_color(stroke)
                    .bg(rgb(palette.canvas)),
            )
            .when_some(classifier.header.applied_stereotype, |root, stereotype| {
                root.child(
                    div()
                        .absolute()
                        .left_0()
                        .right_0()
                        .top(units(layout.label_top - 14.0))
                        .h(px(14.0 * scale))
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(10.0 * scale))
                        .text_color(rgb(palette.muted))
                        .child(stereotype),
                )
            })
            .child(
                div()
                    .id("uml-interface-name")
                    .absolute()
                    .left_0()
                    .right_0()
                    .top(units(layout.label_top))
                    .min_h(px(22.0 * scale))
                    .min_w_0()
                    .px_1()
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .on_mouse_down(MouseButton::Left, move |event, window, cx| {
                        if event.click_count >= 2 {
                            if let Some(handler) = &edit_title {
                                handler(window, cx);
                            }
                            cx.stop_propagation();
                        }
                    })
                    .child(title),
            )
            .child(
                div()
                    .absolute()
                    .left_0()
                    .right_0()
                    .top(units(layout.members_top))
                    .flex()
                    .flex_col()
                    .when(!classifier.attributes.is_empty(), |root| {
                        root.child(compartment(
                            classifier.attributes,
                            "attribute",
                            interactions.clone(),
                            palette,
                            true,
                        ))
                    })
                    .when(!classifier.operations.is_empty(), |root| {
                        root.child(compartment(
                            classifier.operations,
                            "operation",
                            interactions.clone(),
                            palette,
                            true,
                        ))
                    })
                    .when(!classifier.receptions.is_empty(), |root| {
                        root.child(compartment(
                            classifier.receptions,
                            "reception",
                            interactions,
                            palette,
                            true,
                        ))
                    }),
            )
    }
}
