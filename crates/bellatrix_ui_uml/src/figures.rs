//! Non-classifier notations share editing, but not the classifier silhouette.
use crate::{
    class::{MemberInteractions, MemberRow, UmlClassifier, ViewAppearance, template_signature},
    inline_edit::UmlInlineNameEditor,
};
use bellatrix_design_system::Palette;
use bellatrix_mde::{UmlNotation, folded_note_layout, uml_header, uml_notation};
use gpui::{
    App, Bounds, Entity, InteractiveElement, IntoElement, MouseButton, PathBuilder, Pixels,
    RenderOnce, SharedString, Window, canvas, div, point, prelude::*, px,
};
use std::rc::Rc;
use ui_input::InputField;
type Action = Rc<dyn Fn(&mut Window, &mut App)>;

#[derive(IntoElement)]
pub struct UmlFigure {
    kind: String,
    name: SharedString,
    applied_stereotype: Option<SharedString>,
    selected: bool,
    scale: f32,
    appearance: ViewAppearance,
    edit: Option<Action>,
    editor: Option<(Entity<InputField>, Action, Action)>,
    template_parameters: Option<(Vec<MemberRow>, MemberInteractions, f64)>,
}
impl UmlFigure {
    pub fn supports(kind: &str) -> bool {
        matches!(
            uml_notation(kind),
            UmlNotation::PackageTab
                | UmlNotation::FoldedNote
                | UmlNotation::Frame
                | UmlNotation::AssociationDiamond
                | UmlNotation::Port
                | UmlNotation::Part
        )
    }
    pub fn new(kind: impl Into<String>, name: impl Into<SharedString>) -> Self {
        Self {
            kind: kind.into(),
            name: name.into(),
            applied_stereotype: None,
            selected: false,
            scale: 1.0,
            appearance: ViewAppearance::default(),
            edit: None,
            editor: None,
            template_parameters: None,
        }
    }
    pub fn view_style(mut self, style: &bellatrix_mde::Style) -> Self {
        self.appearance = ViewAppearance(style.clone());
        self
    }
    pub fn with_template_parameters_from(mut self, projection: &UmlClassifier) -> Self {
        if uml_notation(&self.kind) == UmlNotation::PackageTab {
            self.template_parameters = Some((
                projection.template_parameters.clone(),
                projection.member_interactions(),
                projection.view_width,
            ));
        }
        self
    }
    pub fn stereotype(mut self, value: impl Into<SharedString>) -> Self {
        let value = value.into();
        self.applied_stereotype = uml_header(&self.kind, Some(value.as_ref()))
            .applied_stereotype
            .map(SharedString::from);
        self
    }
    pub fn selected(mut self, value: bool) -> Self {
        self.selected = value;
        self
    }
    pub fn view_scale(mut self, scale: f32) -> Self {
        if scale.is_finite() && scale > 0.0 {
            self.scale = scale;
        }
        self
    }
    pub fn on_title_edit(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.edit = Some(Rc::new(handler));
        self
    }
    pub fn inline_edit(
        mut self,
        field: Entity<InputField>,
        confirm: impl Fn(&mut Window, &mut App) + 'static,
        cancel: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.editor = Some((field, Rc::new(confirm), Rc::new(cancel)));
        self
    }
}
fn paint_folded_paper(
    bounds: Bounds<Pixels>,
    scale: f32,
    palette: Palette,
    selected: bool,
    appearance: &ViewAppearance,
    window: &mut Window,
) {
    let layout = folded_note_layout(
        f64::from(bounds.size.width) / f64::from(scale),
        f64::from(bounds.size.height) / f64::from(scale),
    );
    let outline = layout.outline.map(|position| {
        point(
            bounds.origin.x + px(position.x as f32 * scale),
            bounds.origin.y + px(position.y as f32 * scale),
        )
    });
    let fold = layout.fold.map(|position| {
        point(
            bounds.origin.x + px(position.x as f32 * scale),
            bounds.origin.y + px(position.y as f32 * scale),
        )
    });
    let stroke = appearance.stroke(palette, selected);
    let mut fill = PathBuilder::fill();
    fill.add_polygon(&outline, true);
    match fill.build() {
        Ok(path) => window.paint_path(path, appearance.fill(palette.panel)),
        Err(error) => log::error!("UML paper fill: {error}"),
    }
    let mut border = PathBuilder::stroke(px(appearance.width(1.0, scale, selected)));
    border.add_polygon(&outline, true);
    border.add_polygon(&fold, false);
    match border.build() {
        Ok(path) => window.paint_path(path, stroke),
        Err(error) => log::error!("UML paper outline: {error}"),
    }
}
impl RenderOnce for UmlFigure {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let palette = Palette::current(cx);
        let scale = self.scale;
        let selected = self.selected;
        let signature = self
            .template_parameters
            .and_then(|(rows, interactions, width)| {
                template_signature(rows, width, interactions, palette, selected)
            });
        let appearance = self.appearance;
        let font_family = appearance.0.font_family.clone();
        let border = appearance.stroke(palette, selected);
        let title: gpui::AnyElement = if let Some((field, confirm, cancel)) = self.editor {
            UmlInlineNameEditor::new(
                field,
                move |window, cx| confirm(window, cx),
                move |window, cx| cancel(window, cx),
            )
            .into_any_element()
        } else {
            div().child(self.name).into_any_element()
        };
        let edit = self.edit;
        let title_area = div()
            .id("uml-figure-title")
            .min_w_0()
            .text_size(px(appearance.font_size(12.0, scale)))
            .text_color(appearance.text(palette.text))
            .font_weight(gpui::FontWeight::SEMIBOLD)
            .on_mouse_down(MouseButton::Left, move |event, window, cx| {
                if event.click_count >= 2 {
                    if let Some(handler) = &edit {
                        handler(window, cx);
                    }
                    cx.stop_propagation();
                }
            })
            .child(title);
        let figure = match uml_notation(&self.kind) {
            UmlNotation::FoldedNote => div()
                .size_full()
                .relative()
                .child(
                    canvas(
                        move |_, _, _| (),
                        move |bounds, (), window, _| {
                            paint_folded_paper(
                                bounds,
                                scale,
                                palette,
                                selected,
                                &appearance,
                                window,
                            );
                        },
                    )
                    .absolute()
                    .size_full(),
                )
                .child(
                    div()
                        .absolute()
                        .left(px(10.0 * scale))
                        .top(px(10.0 * scale))
                        .right(px(24.0 * scale))
                        .bottom(px(8.0 * scale))
                        .overflow_hidden()
                        .child(title_area),
                )
                .into_any_element(),
            UmlNotation::PackageTab => {
                let keyword = uml_header(&self.kind, None).keyword;
                div()
                    .size_full()
                    .relative()
                    .child(
                        div()
                            .absolute()
                            .left_0()
                            .top(px(16.0 * scale))
                            .right_0()
                            .bottom_0()
                            .border(px(appearance.width(1.0, scale, selected)))
                            .border_color(border)
                            .bg(appearance.fill(palette.panel)),
                    )
                    .child(
                        div()
                            .absolute()
                            .left_0()
                            .top_0()
                            .w(px(87.0 * scale))
                            .h(px(17.0 * scale))
                            .border(px(appearance.width(1.0, scale, selected)))
                            .border_color(border)
                            .bg(appearance.fill(palette.panel_alt)),
                    )
                    .child(
                        div()
                            .absolute()
                            .left(px(11.0 * scale))
                            .top(px(25.0 * scale))
                            .right(px(8.0 * scale))
                            .flex()
                            .flex_col()
                            .overflow_hidden()
                            .when_some(keyword, |element, label| {
                                element.child(
                                    div()
                                        .whitespace_normal()
                                        .text_size(px(appearance.font_size(10.0, scale)))
                                        .text_color(appearance.text(palette.muted))
                                        .child(label),
                                )
                            })
                            .when_some(self.applied_stereotype, |element, label| {
                                element.child(
                                    div()
                                        .whitespace_normal()
                                        .text_size(px(appearance.font_size(10.0, scale)))
                                        .text_color(appearance.text(palette.muted))
                                        .child(label),
                                )
                            })
                            .child(title_area),
                    )
                    .into_any_element()
            }
            UmlNotation::Port => div()
                .size_full()
                .border(px(appearance.width(1.0, scale, selected)))
                .border_color(border)
                .bg(appearance.fill(palette.panel_alt))
                .child(
                    div()
                        .size_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(8.0 * scale))
                        .child(title_area),
                )
                .into_any_element(),
            UmlNotation::Part => div()
                .size_full()
                .flex()
                .flex_col()
                .border(px(appearance.width(1.0, scale, selected)))
                .border_color(border)
                .bg(appearance.fill(palette.panel))
                .child(
                    div()
                        .w_full()
                        .h(px(20.0 * scale))
                        .px_1()
                        .border_b(px(appearance.width(1.0, scale, false)))
                        .border_color(border)
                        .flex()
                        .items_center()
                        .child(title_area),
                )
                .into_any_element(),
            UmlNotation::Frame => div()
                .when(appearance.0.fill.is_some(), |element| {
                    element.bg(appearance.fill(palette.canvas))
                })
                .size_full()
                .relative()
                .border(px(appearance.width(1.0, scale, selected)))
                .border_color(border)
                .child(
                    div()
                        .w(px(142.0 * scale))
                        .h(px(30.0 * scale))
                        .border_r(px(appearance.width(1.0, scale, false)))
                        .border_b(px(appearance.width(1.0, scale, false)))
                        .border_color(border)
                        .px_2()
                        .flex()
                        .items_center()
                        .bg(appearance.fill(palette.panel_alt))
                        .child(title_area),
                )
                .into_any_element(),
            UmlNotation::AssociationDiamond => div()
                .size_full()
                .relative()
                .child(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .right_0()
                        .bottom(px(22.0 * scale))
                        .child(
                            canvas(
                                |_, _, _| (),
                                move |bounds, (), window, _| {
                                    let width = f32::from(bounds.size.width);
                                    let height = f32::from(bounds.size.height);
                                    let vertices =
                                        [(0.5, 0.04), (0.96, 0.5), (0.5, 0.96), (0.04, 0.5)].map(
                                            |(x, y)| {
                                                point(
                                                    bounds.origin.x + px(x * width),
                                                    bounds.origin.y + px(y * height),
                                                )
                                            },
                                        );
                                    if appearance.0.fill.is_some() {
                                        let mut fill = PathBuilder::fill();
                                        fill.add_polygon(&vertices, true);
                                        match fill.build() {
                                            Ok(path) => window
                                                .paint_path(path, appearance.fill(palette.canvas)),
                                            Err(error) => {
                                                log::error!("UML association diamond fill: {error}")
                                            }
                                        }
                                    }
                                    let mut outline = PathBuilder::stroke(px(
                                        appearance.width(1.0, scale, selected)
                                    ));
                                    outline.add_polygon(&vertices, true);
                                    match outline.build() {
                                        Ok(path) => window.paint_path(path, border),
                                        Err(error) => {
                                            log::error!("UML association diamond outline: {error}")
                                        }
                                    }
                                },
                            )
                            .size_full(),
                        ),
                )
                .child(
                    div()
                        .absolute()
                        .left_0()
                        .right_0()
                        .bottom_0()
                        .h(px(22.0 * scale))
                        .flex()
                        .justify_center()
                        .items_center()
                        .child(title_area),
                )
                .into_any_element(),
            _ => div()
                .size_full()
                .border(px(appearance.width(1.0, scale, selected)))
                .border_color(border)
                .flex()
                .items_center()
                .justify_center()
                .child(title_area)
                .into_any_element(),
        };
        div()
            .size_full()
            .relative()
            .when_some(font_family, |element, family| element.font_family(family))
            .child(figure)
            .when_some(signature, |element, signature| element.child(signature))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_and_model_reuse_the_same_signature_projection_and_body_width() {
        let projection = UmlClassifier::new("Owner")
            .view_size(220.0, 100.0, 2.0)
            .template_parameter_with_id("formal", "Item");
        for kind in ["uml.package", "uml.model"] {
            let figure = UmlFigure::new(kind, "Owner").with_template_parameters_from(&projection);
            let (rows, interactions, width) =
                figure.template_parameters.expect("signature projection");
            assert_eq!(width, 220.0);
            assert_eq!(interactions.scale, 2.0);
            let row = rows.first().expect("formal parameter");
            assert_eq!(row.id.as_deref(), Some("formal"));
            assert_eq!(row.title.as_ref(), "Item");
        }
        assert!(
            UmlFigure::new("uml.note", "Note")
                .with_template_parameters_from(&projection)
                .template_parameters
                .is_none()
        );
    }

    #[test]
    fn profile_reuses_package_figure_but_deferred_notations_do_not() {
        assert!(UmlFigure::supports("uml.profile"));
        for kind in [
            "uml.node",
            "uml.instance_specification",
            "uml.collaboration",
        ] {
            assert!(!UmlFigure::supports(kind));
        }
    }
}
