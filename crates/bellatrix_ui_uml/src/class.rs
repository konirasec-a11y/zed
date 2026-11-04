//! Shared classifier labels, compartments and editing. The notation selects the silhouette.
use crate::inline_edit::UmlInlineNameEditor;
use bellatrix_design_system::Palette;
use bellatrix_mde_cordis::presentation_contract::{
    ClassifierPresentation, MemberPresentation, RenderStyle, UmlHeader,
};
use bellatrix_mde_cordis::{LineMode, TextAlignment, TextVerticalAlignment};
use gpui::{
    App, Entity, InteractiveElement, IntoElement, MouseButton, PathBuilder, RenderOnce,
    SharedString, Window, canvas, div, point, prelude::*, px, rgb,
};
use std::rc::Rc;
use ui_input::InputField;

#[derive(Clone, Default)]
pub struct ViewAppearance(pub(crate) RenderStyle);
impl ViewAppearance {
    pub fn new(style: RenderStyle) -> Self {
        Self(style)
    }
    fn color(&self, value: Option<&str>, fallback: u32) -> gpui::Rgba {
        value
            .and_then(|value| gpui::Rgba::try_from(value).ok())
            .unwrap_or_else(|| rgb(fallback))
            .opacity(self.0.opacity.unwrap_or(1.0) as f32)
    }
    pub fn fill(&self, fallback: u32) -> gpui::Rgba {
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
    pub(crate) fn resolve_stroke_pattern(
        &self,
        notation_mode: LineMode,
        scale: f32,
    ) -> Vec<gpui::Pixels> {
        self.0
            .stroke_pattern
            .as_deref()
            .unwrap_or(match notation_mode {
                LineMode::Solid => &[],
                LineMode::Dotted => &[1.0, 3.0],
                LineMode::Dashed => &[6.0, 5.0],
            })
            .iter()
            .map(|value| px(*value as f32 * scale))
            .collect()
    }
    pub fn apply_rectangle_border(
        &self,
        element: gpui::Div,
        palette: Palette,
        selected: bool,
        scale: f32,
        default_radius: f64,
    ) -> gpui::Div {
        self.apply_notation_rectangle_border(
            element,
            palette,
            selected,
            scale,
            default_radius,
            false,
        )
    }
    pub(crate) fn apply_notation_rectangle_border(
        &self,
        element: gpui::Div,
        palette: Palette,
        selected: bool,
        scale: f32,
        default_radius: f64,
        notation_dashed: bool,
    ) -> gpui::Div {
        let width = self.width(1.0, scale, selected);
        let radius = self.0.corner_radius.unwrap_or(default_radius) as f32 * scale;
        let color = self.stroke(palette, selected);
        let pattern = self.resolve_stroke_pattern(
            if notation_dashed {
                LineMode::Dashed
            } else {
                LineMode::Solid
            },
            scale,
        );
        let element = element.border(px(width)).rounded(px(radius));
        if pattern.is_empty() {
            return element.border_color(color);
        }
        element.border_color(gpui::rgba(0)).child(
            canvas(
                |_, _, _| (),
                move |bounds, (), window, _| {
                    let inset = width / 2.0;
                    let left = f32::from(bounds.origin.x) + inset;
                    let top = f32::from(bounds.origin.y) + inset;
                    let right = left + (f32::from(bounds.size.width) - width).max(0.0);
                    let bottom = top + (f32::from(bounds.size.height) - width).max(0.0);
                    let radius = radius
                        .max(0.0)
                        .min((right - left) / 2.0)
                        .min((bottom - top) / 2.0);
                    let mut points = Vec::new();
                    for (center_x, center_y, start_angle) in [
                        (right - radius, top + radius, -std::f32::consts::FRAC_PI_2),
                        (right - radius, bottom - radius, 0.0),
                        (left + radius, bottom - radius, std::f32::consts::FRAC_PI_2),
                        (left + radius, top + radius, std::f32::consts::PI),
                    ] {
                        for step in 0..=8 {
                            let angle =
                                start_angle + step as f32 * std::f32::consts::FRAC_PI_2 / 8.0;
                            points.push(point(
                                px(center_x + radius * angle.cos()),
                                px(center_y + radius * angle.sin()),
                            ));
                        }
                    }
                    let mut outline = PathBuilder::stroke(px(width)).dash_array(&pattern);
                    outline.add_polygon(&points, true);
                    match outline.build() {
                        Ok(path) => window.paint_path(path, color),
                        Err(error) => log::error!("UML styled outline: {error}"),
                    }
                },
            )
            .absolute()
            .top_0()
            .left_0()
            .size_full(),
        )
    }
    pub(crate) fn apply_text_wrap<T: gpui::Styled>(&self, element: T) -> T {
        match self.0.word_wrap {
            Some(true) => element.whitespace_normal(),
            Some(false) => element.whitespace_nowrap(),
            None => element,
        }
    }
    pub fn apply_surface<T: gpui::Styled>(&self, element: T, scale: f32) -> T {
        if self.0.show_shadow == Some(true)
            && let Some(shadow) = &self.0.shadow
        {
            return element.shadow(vec![
                gpui::BoxShadow::new(
                    px(shadow.offset_x as f32 * scale),
                    px(shadow.offset_y as f32 * scale),
                    self.color(Some(&shadow.color), 0).into(),
                )
                .blur_radius(px(shadow.blur_radius as f32 * scale)),
            ]);
        }
        match self.0.show_shadow {
            Some(true) => element.shadow(vec![
                gpui::BoxShadow::new(
                    px(2.0 * scale),
                    px(3.0 * scale),
                    gpui::hsla(0.0, 0.0, 0.0, 0.25 * self.0.opacity.unwrap_or(1.0) as f32),
                )
                .blur_radius(px(4.0 * scale)),
            ]),
            Some(false) => element.shadow(Vec::new()),
            None => element,
        }
    }
    pub(crate) fn align_text<T: gpui::Styled>(&self, element: T) -> T {
        let element = self.apply_text_wrap(element);
        match self.0.text_align {
            Some(TextAlignment::Left) => element.text_left(),
            Some(TextAlignment::Center) => element.text_center(),
            Some(TextAlignment::Right) => element.text_right(),
            None => element,
        }
    }
    pub(crate) fn align_row<T: gpui::Styled>(&self, element: T) -> T {
        let element = self.align_text(element);
        let element = match self.0.text_align {
            Some(TextAlignment::Left) => element.justify_start(),
            Some(TextAlignment::Center) => element.justify_center(),
            Some(TextAlignment::Right) => element.justify_end(),
            None => element,
        };
        match self.0.text_vertical_align {
            Some(TextVerticalAlignment::Top) => element.items_start(),
            Some(TextVerticalAlignment::Center) => element.items_center(),
            Some(TextVerticalAlignment::Bottom) => element.items_end(),
            None => element,
        }
    }
    pub fn align_column<T: gpui::Styled>(&self, element: T) -> T {
        let element = self.align_text(element);
        let element = match self.0.text_align {
            Some(TextAlignment::Left) => element.items_start(),
            Some(TextAlignment::Center) => element.items_center(),
            Some(TextAlignment::Right) => element.items_end(),
            None => element,
        };
        match self.0.text_vertical_align {
            Some(TextVerticalAlignment::Top) => element.justify_start(),
            Some(TextVerticalAlignment::Center) => element.justify_center(),
            Some(TextVerticalAlignment::Bottom) => element.justify_end(),
            None => element,
        }
    }
    pub(crate) fn font_size(&self, default: f32, scale: f32) -> f32 {
        self.0
            .font_size
            .map_or(default, |value| value as f32 * default / 12.0)
            * scale
    }
    pub fn apply_text_style<T: gpui::Styled>(
        &self,
        element: T,
        default_size: f32,
        default_color: u32,
        scale: f32,
    ) -> T {
        let mut element = element
            .text_size(px(self.font_size(default_size, scale)))
            .text_color(self.text(default_color));
        if let Some(family) = &self.0.font_family {
            element = element.font_family(family.clone());
        }
        if let Some(weight) = self.0.font_weight {
            element = element.font_weight(gpui::FontWeight(f32::from(weight)));
        }
        element = match self.0.font_italic {
            Some(true) => element.italic(),
            Some(false) => element.not_italic(),
            None => element,
        };
        element = match self.0.font_underline {
            Some(true) => element.underline(),
            Some(false) => element.text_decoration_none(),
            None => element,
        };
        self.align_text(element)
    }
}

pub(crate) type ElementAction = Rc<dyn Fn(&str, &mut Window, &mut App)>;
pub(crate) type Action = Rc<dyn Fn(&mut Window, &mut App)>;
#[derive(Clone)]
pub(crate) struct MemberRow {
    pub(crate) id: Option<String>,
    pub(crate) title: SharedString,
    pub(crate) presentation: MemberPresentation,
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
    pub(crate) attributes: Vec<MemberRow>,
    pub(crate) operations: Vec<MemberRow>,
    pub(crate) receptions: Vec<MemberRow>,
    show_internal_structure: bool,
    authored_contents: bool,
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
    pub fn authored_contents(mut self, authored: bool) -> Self {
        self.authored_contents = authored;
        self
    }
    pub fn show_internal_structure(mut self, show: bool) -> Self {
        self.show_internal_structure = show;
        self
    }
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            class_id: None,
            title: title.into(),
            header: UmlHeader::default(),
            presentation: ClassifierPresentation::default(),
            attributes: Vec::new(),
            operations: Vec::new(),
            receptions: Vec::new(),
            show_internal_structure: false,
            authored_contents: false,
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
    pub fn view_style(mut self, style: &RenderStyle) -> Self {
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
    pub fn header(mut self, header: UmlHeader) -> Self {
        self.header = header;
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
            presentation: MemberPresentation::default(),
        });
        self
    }
    pub fn operation(mut self, value: impl Into<SharedString>) -> Self {
        self.operations.push(MemberRow {
            id: None,
            title: value.into(),
            presentation: MemberPresentation::default(),
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
            presentation: MemberPresentation::default(),
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
            presentation: MemberPresentation::default(),
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
            presentation: MemberPresentation::default(),
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
            presentation: MemberPresentation::default(),
        });
        self
    }
    pub fn member_presentation(mut self, id: &str, presentation: MemberPresentation) -> Self {
        for row in self
            .attributes
            .iter_mut()
            .chain(&mut self.operations)
            .chain(&mut self.receptions)
        {
            if row.id.as_deref() == Some(id) {
                row.presentation = presentation;
            }
        }
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
                    .when(member.presentation.underlined, |element| {
                        element.underline()
                    })
                    .child(member.title)
                    .into_any_element()
            };
            div()
                .id(format!("uml-{family}-{index}"))
                .w_full()
                .min_w_0()
                .min_h(px(20.0 * scale))
                .flex()
                .flex_col()
                .map(|element| appearance.align_column(element))
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
    let layout = crate::figure_geometry::template_signature_layout(body_width, rows.len())?;
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
                    .flex()
                    .flex_col()
                    .map(|element| appearance.align_column(element))
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
        let mut header = self.header;
        if !appearance.0.show_stereotype {
            header.applied_stereotype = None;
        }
        let header_height = header.classifier_header_height;
        let keyword = header.keyword;
        let applied_stereotype = header.applied_stereotype;
        let body = div()
            .size_full()
            .flex()
            .flex_col()
            .relative()
            .map(|element| {
                appearance.apply_rectangle_border(element, palette, self.selected, scale, 2.0)
            })
            .when_some(appearance.0.font_family.clone(), |element, family| {
                element.font_family(family)
            })
            .bg(appearance.fill(palette.panel))
            .overflow_hidden()
            .when(!self.authored_contents, |element| {
                element.child(
                    div()
                        .id("uml-classifier-title")
                        .min_h(px(header_height as f32 * scale))
                        .w_full()
                        .flex_none()
                        .px_1()
                        .flex()
                        .items_center()
                        .justify_center()
                        .map(|element| appearance.align_row(element))
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
                                .map(|element| appearance.align_column(element))
                                .whitespace_normal()
                                .map(|element| appearance.apply_text_wrap(element))
                                .when_some(keyword, |header, value| {
                                    header.child(
                                        div()
                                            .w_full()
                                            .text_center()
                                            .map(|element| appearance.align_text(element))
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
                                            .map(|element| appearance.align_text(element))
                                            .text_size(px(appearance.font_size(10.0, scale)))
                                            .text_color(appearance.text(palette.muted))
                                            .child(value),
                                    )
                                })
                                .child(title),
                        ),
                )
            })
            .when(
                !self.authored_contents
                    && appearance.0.show_attributes
                    && !self.attributes.is_empty(),
                |element| {
                    element.child(compartment(
                        self.attributes,
                        "attribute",
                        interactions.clone(),
                        palette,
                        false,
                    ))
                },
            )
            .when(
                !self.authored_contents
                    && appearance.0.show_operations
                    && !self.operations.is_empty(),
                |element| {
                    element.child(compartment(
                        self.operations,
                        "operation",
                        interactions.clone(),
                        palette,
                        true,
                    ))
                },
            )
            .when(
                !self.authored_contents
                    && appearance.0.show_receptions
                    && !self.receptions.is_empty(),
                |element| {
                    element.child(compartment(
                        self.receptions,
                        "reception",
                        interactions,
                        palette,
                        true,
                    ))
                },
            )
            .when(
                self.show_internal_structure && !self.authored_contents,
                |element| {
                    element.child(
                        div()
                            .flex_1()
                            .min_h(px(60.0 * scale))
                            .border_t(px(appearance.width(1.0, scale, false)))
                            .border_color(line)
                            .pt(px(3.0 * scale))
                            .px(px(6.0 * scale))
                            .text_size(px(appearance.font_size(10.0, scale)))
                            .text_color(appearance.text(palette.muted))
                            .child("internal structure"),
                    )
                },
            )
            .into_any_element();
        div()
            .size_full()
            .relative()
            .map(|element| appearance.apply_surface(element, scale))
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
