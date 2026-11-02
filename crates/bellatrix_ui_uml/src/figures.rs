//! Non-classifier notations share editing, but not the classifier silhouette.
use crate::inline_edit::UmlInlineNameEditor;
use bellatrix_design_system::Palette;
use bellatrix_mde_modeling::{UmlType, folded_note_layout};
use gpui::{App, Bounds, Entity, InteractiveElement, IntoElement, MouseButton, PathBuilder,
    Pixels, RenderOnce, SharedString, Window, canvas, div, point, prelude::*, px, rgb, svg};
use std::rc::Rc;
use ui_input::InputField;
type Action = Rc<dyn Fn(&mut Window, &mut App)>;

#[derive(IntoElement)]
pub struct UmlFigure {
    kind: String,
    name: SharedString,
    selected: bool,
    scale: f32,
    edit: Option<Action>,
    editor: Option<(Entity<InputField>, Action, Action)>,
}
impl UmlFigure {
    pub fn supports(kind: &str) -> bool {
        matches!(UmlType::from_kind(kind), Some(UmlType::Package | UmlType::Model |
            UmlType::Subsystem | UmlType::Note | UmlType::Constraint | UmlType::Frame |
            UmlType::NaryAssociation | UmlType::Port | UmlType::Part))
    }
    pub fn new(kind: impl Into<String>, name: impl Into<SharedString>) -> Self {
        Self { kind: kind.into(), name: name.into(), selected: false, scale: 1.0, edit: None, editor: None }
    }
    pub fn selected(mut self, value: bool) -> Self { self.selected = value; self }
    pub fn view_scale(mut self, scale: f32) -> Self {
        if scale.is_finite() && scale > 0.0 { self.scale = scale; }
        self
    }
    pub fn on_title_edit(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.edit = Some(Rc::new(handler)); self
    }
    pub fn inline_edit(mut self, field: Entity<InputField>,
        confirm: impl Fn(&mut Window, &mut App) + 'static,
        cancel: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.editor = Some((field, Rc::new(confirm), Rc::new(cancel))); self
    }
}

fn paint_folded_paper(bounds: Bounds<Pixels>, scale: f32, palette: Palette, selected: bool, window: &mut Window) {
    let layout = folded_note_layout(f64::from(bounds.size.width) / f64::from(scale),
        f64::from(bounds.size.height) / f64::from(scale));
    let outline = layout.outline.map(|position| point(
        bounds.origin.x + px(position.x as f32 * scale),
        bounds.origin.y + px(position.y as f32 * scale)));
    let fold = layout.fold.map(|position| point(
        bounds.origin.x + px(position.x as f32 * scale),
        bounds.origin.y + px(position.y as f32 * scale)));
    let stroke = rgb(if selected { palette.selection } else { palette.text });
    let mut fill = PathBuilder::fill();
    fill.add_polygon(&outline, true);
    match fill.build() {
        Ok(path) => window.paint_path(path, rgb(palette.panel)),
        Err(error) => log::error!("UML paper fill: {error}"),
    }
    let mut border = PathBuilder::stroke(px(scale));
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
        let border = rgb(if selected { palette.selection } else { palette.border });
        let title: gpui::AnyElement = if let Some((field, confirm, cancel)) = self.editor {
            UmlInlineNameEditor::new(field, move |window, cx| confirm(window, cx),
                move |window, cx| cancel(window, cx)).into_any_element()
        } else { div().child(self.name).into_any_element() };
        let edit = self.edit;
        let title_area = div().id("uml-figure-title").min_w_0()
            .text_size(px(12.0 * scale)).text_color(rgb(palette.text))
            .font_weight(gpui::FontWeight::SEMIBOLD)
            .on_mouse_down(MouseButton::Left, move |event, window, cx| {
                if event.click_count >= 2 {
                    if let Some(handler) = &edit { handler(window, cx); }
                    cx.stop_propagation();
                }
            }).child(title);
        match UmlType::from_kind(&self.kind) {
            Some(UmlType::Note | UmlType::Constraint) => {
                // The paper is the shape itself, not a square SVG icon behind a rectangular label.
                div().size_full().relative()
                    .child(canvas(move |_, _, _| (), move |bounds, (), window, _| {
                        paint_folded_paper(bounds, scale, palette, selected, window);
                    }).absolute().size_full())
                    .child(div().absolute().left(px(10.0 * scale)).top(px(10.0 * scale))
                        .right(px(24.0 * scale)).bottom(px(8.0 * scale))
                        .overflow_hidden().child(title_area))
                    .into_any_element()
            }
            Some(kind @ (UmlType::Package | UmlType::Model | UmlType::Subsystem)) => {
                let stereotype = match kind {
                    UmlType::Model => Some("«model»"), UmlType::Subsystem => Some("«subsystem»"), _ => None,
                };
                div().size_full().relative()
                    .child(div().absolute().left_0().top(px(16.0 * scale)).right_0().bottom_0()
                        .border_1().border_color(border).bg(rgb(palette.panel)))
                    .child(div().absolute().left_0().top_0().w(px(87.0 * scale)).h(px(17.0 * scale))
                        .border_1().border_color(border).bg(rgb(palette.panel_alt)))
                    .child(div().absolute().left(px(11.0 * scale)).top(px(25.0 * scale)).right(px(8.0 * scale))
                        .flex().flex_col().overflow_hidden()
                        .when_some(stereotype, |element, label| element.child(
                            div().text_size(px(10.0 * scale)).text_color(rgb(palette.muted)).child(label)))
                        .child(title_area)).into_any_element()
            }
            Some(UmlType::Port) => div().size_full()
                .border_1().border_color(border).bg(rgb(palette.panel_alt))
                .child(div().size_full().flex().items_center().justify_center()
                    .text_size(px(8.0 * scale)).child(title_area)).into_any_element(),
            Some(UmlType::Part) => div().size_full().flex().flex_col()
                .border_1().border_color(border).bg(rgb(palette.panel))
                .child(div().w_full().h(px(20.0 * scale)).px_1()
                    .border_b_1().border_color(border).flex().items_center()
                    .child(title_area)).into_any_element(),
            Some(UmlType::Frame) => div().size_full().relative().border_1().border_color(border)
                .child(div().w(px(142.0 * scale)).h(px(30.0 * scale))
                    .border_r_1().border_b_1().border_color(border).px_2().flex().items_center()
                    .bg(rgb(palette.panel_alt)).child(title_area)).into_any_element(),
            Some(UmlType::NaryAssociation) => {
                const DIAMOND: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><polygon points="50,4 96,50 50,96 4,50" fill="none" stroke="currentColor" stroke-width="3"/></svg>"#;
                div().size_full().relative()
                    .child(div().absolute().top_0().left_0().right_0().bottom(px(22.0 * scale))
                        .child(svg().data(DIAMOND).size_full().text_color(
                            rgb(if selected { palette.selection } else { palette.text }))))
                    .child(div().absolute().left_0().right_0().bottom_0().h(px(22.0 * scale))
                        .flex().justify_center().items_center().child(title_area)).into_any_element()
            }
            _ => div().size_full().border_1().border_color(border)
                .flex().items_center().justify_center().child(title_area).into_any_element(),
        }
    }
}
