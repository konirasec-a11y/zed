//! Non-classifier notations share editing, but not the classifier silhouette.
use crate::figure_geometry::{paint_folded_paper, paint_notation, port_label_layout};
use crate::{
    class::{MemberInteractions, MemberRow, UmlClassifier, ViewAppearance, template_signature},
    inline_edit::UmlInlineNameEditor,
    interface_presentation::InterfaceFigure,
    timing::TimingLifeline,
};
use bellatrix_design_system::Palette;
use bellatrix_mde_cordis::presentation_contract::{
    FigurePresentation, RenderStyle, UmlHeader, UmlNotation,
};
use bellatrix_mde_cordis::projection::InterfaceIconProjection;
use bellatrix_mde_cordis::{LineMode, TimingPresentation};
use gpui::{
    App, Entity, InteractiveElement, IntoElement, MouseButton, PathBuilder, RenderOnce,
    SharedString, Window, canvas, div, point, prelude::*, px,
};
use std::rc::Rc;
use ui_input::InputField;
type Action = Rc<dyn Fn(&mut Window, &mut App)>;

#[derive(IntoElement)]
pub struct UmlFigure {
    notation: UmlNotation,
    header: UmlHeader,
    title_underlined: bool,
    history_label: String,
    interface_icon: Option<InterfaceIconProjection>,
    context_frame_kind: Option<String>,
    timing_presentation: Option<TimingPresentation>,
    name: SharedString,
    slot_labels: Vec<String>,
    interaction_operator: Option<String>,
    has_nested_views: bool,
    authored_contents: bool,
    divider: bool,
    composite_role: bool,
    selected: bool,
    scale: f32,
    view_width: f64,
    view_height: f64,
    appearance: ViewAppearance,
    edit: Option<Action>,
    editor: Option<(Entity<InputField>, Action, Action)>,
    template_parameters: Option<(Vec<MemberRow>, MemberInteractions, f64)>,
}
impl UmlFigure {
    pub fn supports(presentation: &FigurePresentation) -> bool {
        presentation.interface.is_some()
            || presentation.timing.is_some()
            || matches!(
                presentation.notation,
                UmlNotation::Rectangle
                    | UmlNotation::Label
                    | UmlNotation::Compartment
                    | UmlNotation::Unsupported
                    | UmlNotation::PackageTab
                    | UmlNotation::FoldedNote
                    | UmlNotation::Frame
                    | UmlNotation::AssociationDiamond
                    | UmlNotation::Port
                    | UmlNotation::Part
                    | UmlNotation::Actor
                    | UmlNotation::UseCase
                    | UmlNotation::DeploymentNode
                    | UmlNotation::Collaboration
                    | UmlNotation::Instance
                    | UmlNotation::Action
                    | UmlNotation::State
                    | UmlNotation::InitialNode
                    | UmlNotation::FinalNode
                    | UmlNotation::FlowFinal
                    | UmlNotation::Decision
                    | UmlNotation::Fork
                    | UmlNotation::Lifeline
                    | UmlNotation::Execution
                    | UmlNotation::InteractionUse
                    | UmlNotation::CombinedFragment
                    | UmlNotation::InteractionOperand
                    | UmlNotation::ObjectNode
                    | UmlNotation::SendSignal
                    | UmlNotation::AcceptEvent
                    | UmlNotation::Partition
                    | UmlNotation::TimingState
                    | UmlNotation::History
                    | UmlNotation::Terminate
            )
    }
    pub fn new(presentation: FigurePresentation, name: impl Into<SharedString>) -> Self {
        Self {
            notation: presentation.notation,
            header: presentation.header,
            title_underlined: presentation.title_underlined,
            history_label: presentation.history_label,
            interface_icon: None,
            context_frame_kind: presentation.frame_kind,
            timing_presentation: presentation.timing,
            name: name.into(),
            slot_labels: presentation.slot_labels,
            interaction_operator: presentation.interaction_operator,
            has_nested_views: false,
            authored_contents: false,
            divider: false,
            composite_role: presentation.composite_role,
            selected: false,
            scale: 1.0,
            view_width: 16.0,
            view_height: 180.0,
            appearance: ViewAppearance::default(),
            edit: None,
            editor: None,
            template_parameters: None,
        }
    }
    pub fn interface_icon(mut self, projection: Option<InterfaceIconProjection>) -> Self {
        self.interface_icon = projection;
        self
    }
    pub fn authored_contents(mut self, authored: bool) -> Self {
        self.authored_contents = authored;
        self
    }
    pub fn divider(mut self, divider: bool) -> Self {
        self.divider = divider;
        self
    }
    pub fn nested_views(mut self, nested: bool) -> Self {
        self.has_nested_views = nested;
        self
    }
    pub fn view_style(mut self, style: &RenderStyle) -> Self {
        self.appearance = ViewAppearance(style.clone());
        self
    }
    pub fn with_template_parameters_from(mut self, projection: &UmlClassifier) -> Self {
        if self.notation == UmlNotation::PackageTab {
            self.template_parameters = Some((
                projection.template_parameters.clone(),
                projection.member_interactions(),
                projection.view_width,
            ));
        }
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
    pub fn view_size(mut self, width: f64, height: f64, scale: f32) -> Self {
        if width.is_finite() && width > 0.0 {
            self.view_width = width;
        }
        if height.is_finite() && height > 0.0 {
            self.view_height = height;
        }
        self.view_scale(scale)
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
impl RenderOnce for UmlFigure {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let notation = self.notation;
        let palette = Palette::current(cx);
        let scale = self.scale;
        let port_label = port_label_layout(self.view_width);
        let selected = self.selected;
        let frame_kind = self.context_frame_kind.as_deref();
        let is_diagram_frame = notation == UmlNotation::Frame || frame_kind.is_some();
        // Descriptor display is single-line; the canonical authored text stays unchanged.
        let descriptor_operator = self
            .interaction_operator
            .as_ref()
            .map(|text| text.replace(['\r', '\n'], " "));
        let display_name: SharedString = if is_diagram_frame {
            self.name.replace(['\r', '\n'], " ").into()
        } else {
            self.name.clone()
        };
        // Shape the actual descriptor with the active font instead of a fixed-width tab.
        let descriptor: SharedString = if let Some(kind) = frame_kind {
            format!("{kind} {display_name}").into()
        } else if notation == UmlNotation::Frame {
            display_name.clone()
        } else if notation == UmlNotation::CombinedFragment {
            descriptor_operator
                .as_deref()
                .unwrap_or("[operator]")
                .to_owned()
                .into()
        } else {
            "ref".into()
        };
        let mut descriptor_font = window.text_style().font();
        if let Some(family) = self.appearance.0.font_family.clone() {
            descriptor_font.family = family.into();
        }
        descriptor_font.weight = gpui::FontWeight::SEMIBOLD;
        let descriptor_size = self.appearance.font_size(12.0, scale);
        let frame_name_offset = if let Some(kind) = frame_kind {
            let prefix: SharedString = format!("{kind} ").into();
            8.0 * scale
                + f32::from(
                    window
                        .text_system()
                        .shape_line(
                            prefix.clone(),
                            px(descriptor_size),
                            &[gpui::TextRun {
                                len: prefix.len(),
                                font: descriptor_font.clone(),
                                ..Default::default()
                            }],
                            None,
                        )
                        .width,
                )
        } else {
            8.0 * scale
        };
        let descriptor_width = (f32::from(
            window
                .text_system()
                .shape_line(
                    descriptor.clone(),
                    px(descriptor_size),
                    &[gpui::TextRun {
                        len: descriptor.len(),
                        font: descriptor_font,
                        ..Default::default()
                    }],
                    None,
                )
                .width,
        ) + 28.0 * scale)
            .min(self.view_width as f32 * scale - 4.0 * scale)
            .max(32.0 * scale);
        let descriptor_height = descriptor_size * 1.5 + 10.0 * scale;
        let signature = self
            .template_parameters
            .and_then(|(rows, interactions, width)| {
                template_signature(rows, width, interactions, palette, selected)
            });
        let appearance = self.appearance;
        let alignment = appearance.clone();
        let applied_stereotype = self
            .header
            .applied_stereotype
            .filter(|_| alignment.0.show_stereotype && !self.authored_contents)
            .map(SharedString::from);
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
            div().child(display_name).into_any_element()
        };
        let edit = self.edit;
        let title_area = div()
            .id("uml-figure-title")
            .min_w_0()
            .map(|element| alignment.align_text(element))
            .text_size(px(appearance.font_size(12.0, scale)))
            .text_color(appearance.text(palette.text))
            .font_weight(if notation == UmlNotation::Label {
                gpui::FontWeight::NORMAL
            } else {
                gpui::FontWeight::SEMIBOLD
            })
            .when(notation == UmlNotation::Label, |element| {
                appearance.apply_text_style(element, 12.0, palette.text, scale)
            })
            .on_mouse_down(MouseButton::Left, move |event, window, cx| {
                if event.click_count >= 2 {
                    if let Some(handler) = &edit {
                        handler(window, cx);
                    }
                    cx.stop_propagation();
                }
            })
            .when(self.title_underlined, |element| element.underline())
            .when(
                !self.authored_contents || notation == UmlNotation::Unsupported,
                |element| element.child(title),
            );
        if let Some(projection) = self.interface_icon {
            return InterfaceFigure {
                projection,
                title: (!self.authored_contents).then(|| title_area.into_any_element()),
                stereotype: applied_stereotype,
                appearance,
                palette,
                selected,
                scale,
            }
            .render();
        }
        if self.timing_presentation.is_some() {
            return TimingLifeline {
                title: div()
                    .flex()
                    .flex_col()
                    .map(|element| alignment.align_column(element))
                    .when_some(applied_stereotype, |element, label| {
                        element.child(
                            div()
                                .text_size(px(appearance.font_size(10.0, scale)))
                                .text_color(appearance.text(palette.muted))
                                .child(label),
                        )
                    })
                    .child(title_area)
                    .into_any_element(),
                presentation: self.timing_presentation,
                appearance,
                palette,
                selected,
                scale,
                width: self.view_width,
                height: self.view_height,
            }
            .render();
        }
        let figure = match notation {
            UmlNotation::Unsupported => div()
                .size_full()
                .border_1()
                .border_color(gpui::rgb(palette.accent))
                .rounded(px(6.0 * scale))
                .bg(gpui::rgb(palette.panel_alt))
                .p(px(6.0 * scale))
                .flex()
                .flex_col()
                .gap(px(4.0 * scale))
                .child(
                    div()
                        .text_size(px(10.0 * scale))
                        .text_color(gpui::rgb(palette.accent))
                        .child("Unsupported notation"),
                )
                .child(title_area)
                .into_any_element(),
            UmlNotation::Rectangle => div()
                .size_full()
                .relative()
                .bg(appearance.fill(palette.panel))
                .map(|element| {
                    appearance.apply_rectangle_border(element, palette, selected, scale, 0.0)
                })
                .flex()
                .flex_col()
                .map(|element| alignment.align_column(element))
                .child(title_area)
                .into_any_element(),
            UmlNotation::Label => div()
                .size_full()
                .flex()
                .flex_col()
                .map(|element| alignment.align_column(element))
                .child(title_area)
                .into_any_element(),
            UmlNotation::Compartment => div()
                .size_full()
                .relative()
                .when(self.divider, |element| {
                    element.child(
                        canvas(
                            |_, _, _| (),
                            move |bounds, (), window, _| {
                                let width = appearance.width(1.0, scale, selected);
                                let pattern =
                                    appearance.resolve_stroke_pattern(LineMode::Solid, scale);
                                let mut outline = PathBuilder::stroke(px(width));
                                if !pattern.is_empty() {
                                    outline = outline.dash_array(&pattern);
                                }
                                let top = bounds.origin.y + px(width / 2.0);
                                outline.add_polygon(
                                    &[
                                        point(bounds.origin.x, top),
                                        point(bounds.origin.x + bounds.size.width, top),
                                    ],
                                    false,
                                );
                                match outline.build() {
                                    Ok(path) => window
                                        .paint_path(path, appearance.stroke(palette, selected)),
                                    Err(error) => log::error!("UML compartment divider: {error}"),
                                }
                            },
                        )
                        .absolute()
                        .size_full(),
                    )
                })
                .into_any_element(),
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
                        .flex()
                        .flex_col()
                        .map(|element| alignment.align_column(element))
                        .child(title_area),
                )
                .into_any_element(),
            UmlNotation::PackageTab => {
                let keyword = self.header.keyword;
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
                            .map(|element| {
                                appearance
                                    .apply_rectangle_border(element, palette, selected, scale, 0.0)
                            })
                            .bg(appearance.fill(palette.panel)),
                    )
                    .child(
                        div()
                            .absolute()
                            .left_0()
                            .top_0()
                            .w(px(if self.has_nested_views {
                                (self.view_width as f32 * scale * 0.70).max(87.0 * scale)
                            } else {
                                87.0 * scale
                            }))
                            .h(px(if self.has_nested_views {
                                26.0 * scale
                            } else {
                                17.0 * scale
                            }))
                            .map(|element| {
                                appearance
                                    .apply_rectangle_border(element, palette, selected, scale, 0.0)
                            })
                            .bg(appearance.fill(palette.panel_alt)),
                    )
                    .child(
                        div()
                            .absolute()
                            .left(px(11.0 * scale))
                            .when(!self.has_nested_views, |element| {
                                element
                                    .top(px(25.0 * scale))
                                    .bottom(px(8.0 * scale))
                                    .right(px(8.0 * scale))
                            })
                            .when(self.has_nested_views, |element| {
                                element
                                    .top(px(2.0 * scale))
                                    .max_w(px(self.view_width as f32 * scale * 0.65))
                            })
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
                            .when_some(applied_stereotype, |element, label| {
                                element.child(
                                    div()
                                        .whitespace_normal()
                                        .text_size(px(appearance.font_size(10.0, scale)))
                                        .text_color(appearance.text(palette.muted))
                                        .child(label),
                                )
                            })
                            .flex()
                            .flex_col()
                            .map(|element| alignment.align_column(element))
                            .child(title_area),
                    )
                    .into_any_element()
            }
            UmlNotation::Port => div()
                .size_full()
                .relative()
                .map(|element| {
                    appearance.apply_rectangle_border(element, palette, selected, scale, 0.0)
                })
                .bg(appearance.fill(palette.panel_alt))
                .child(
                    div()
                        .absolute()
                        .left(px(port_label.left as f32 * scale))
                        .top(px(port_label.top as f32 * scale))
                        .whitespace_nowrap()
                        .flex()
                        .flex_col()
                        .map(|element| alignment.align_column(element))
                        .child(title_area),
                )
                .into_any_element(),
            UmlNotation::Part => div()
                .size_full()
                .flex()
                .flex_col()
                .map(|element| {
                    appearance.apply_notation_rectangle_border(
                        element,
                        palette,
                        selected,
                        scale,
                        0.0,
                        !self.composite_role,
                    )
                })
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
                        .map(|element| alignment.align_row(element))
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
                                    let dashes =
                                        appearance.resolve_stroke_pattern(LineMode::Solid, scale);
                                    let mut outline = PathBuilder::stroke(px(
                                        appearance.width(1.0, scale, selected)
                                    ));
                                    if !dashes.is_empty() {
                                        outline = outline.dash_array(&dashes);
                                    }
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
                        .flex()
                        .flex_col()
                        .map(|element| alignment.align_column(element))
                        .child(title_area),
                )
                .into_any_element(),
            UmlNotation::Action | UmlNotation::State => div()
                .size_full()
                .relative()
                .map(|element| {
                    appearance.apply_rectangle_border(element, palette, selected, scale, 14.0)
                })
                .bg(appearance.fill(palette.panel))
                .flex()
                .items_center()
                .justify_center()
                .px(px(10.0 * scale))
                .flex()
                .flex_col()
                .map(|element| alignment.align_column(element))
                .child(title_area)
                .into_any_element(),
            UmlNotation::Frame
            | UmlNotation::Actor
            | UmlNotation::UseCase
            | UmlNotation::DeploymentNode
            | UmlNotation::Collaboration
            | UmlNotation::Instance
            | UmlNotation::InitialNode
            | UmlNotation::FinalNode
            | UmlNotation::FlowFinal
            | UmlNotation::Decision
            | UmlNotation::Fork
            | UmlNotation::Lifeline
            | UmlNotation::Execution
            | UmlNotation::InteractionUse
            | UmlNotation::CombinedFragment
            | UmlNotation::InteractionOperand
            | UmlNotation::ObjectNode
            | UmlNotation::SendSignal
            | UmlNotation::AcceptEvent
            | UmlNotation::Partition
            | UmlNotation::TimingState
            | UmlNotation::History
            | UmlNotation::Terminate => {
                let external_label = matches!(
                    notation,
                    UmlNotation::Actor
                        | UmlNotation::InitialNode
                        | UmlNotation::FinalNode
                        | UmlNotation::FlowFinal
                        | UmlNotation::Decision
                        | UmlNotation::Fork
                        | UmlNotation::Execution
                        | UmlNotation::History
                        | UmlNotation::Terminate
                );
                let heading = matches!(
                    notation,
                    UmlNotation::Lifeline
                        | UmlNotation::Partition
                        | UmlNotation::CombinedFragment
                        | UmlNotation::Instance
                        | UmlNotation::ObjectNode
                        | UmlNotation::DeploymentNode
                        | UmlNotation::TimingState
                );
                let history_label = self.history_label;
                let keyword = self.header.keyword;
                let label_color = appearance.text(palette.text);
                let label_size = appearance.font_size(10.0, scale);
                let slot_font_size = appearance.font_size(12.0, scale);
                let frame_keyword = if let Some(kind) = frame_kind {
                    kind.to_owned()
                } else if notation == UmlNotation::Frame {
                    String::new()
                } else if notation == UmlNotation::CombinedFragment {
                    descriptor_operator.unwrap_or_else(|| "[operator]".to_owned())
                } else {
                    "ref".to_owned()
                };
                div()
                    .size_full()
                    .relative()
                    .child(
                        canvas(
                            |_, _, _| (),
                            move |bounds, (), window, _| {
                                paint_notation(
                                    bounds,
                                    notation,
                                    scale,
                                    (descriptor_width, descriptor_height),
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
                    .when(notation == UmlNotation::History, |element| {
                        element.child(
                            div()
                                .absolute()
                                .size_full()
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_size(px(18.0 * scale))
                                .text_color(border)
                                .child(history_label),
                        )
                    })
                    .when(
                        matches!(
                            notation,
                            UmlNotation::Frame
                                | UmlNotation::InteractionUse
                                | UmlNotation::CombinedFragment
                        ) && !frame_keyword.is_empty(),
                        |element| {
                            element.child(
                                div()
                                    .absolute()
                                    .left(px(8.0 * scale))
                                    .top(px(4.0 * scale))
                                    .text_size(px(descriptor_size))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(label_color)
                                    .child(frame_keyword),
                            )
                        },
                    )
                    .child(
                        div()
                            .absolute()
                            .left(px(8.0 * scale))
                            .right(px(8.0 * scale))
                            .when(external_label, |element| {
                                element.top_full().mt(px(3.0 * scale))
                            })
                            .when(!external_label && heading, |element| {
                                element.top(px(7.0 * scale))
                            })
                            .when(notation == UmlNotation::DeploymentNode, |element| {
                                element
                                    .top(gpui::relative(0.20))
                                    .right(gpui::relative(0.20))
                            })
                            .when(notation == UmlNotation::InteractionOperand, |element| {
                                element.top(px(8.0 * scale)).items_start()
                            })
                            .when(is_diagram_frame, |element| {
                                element
                                    .top(px(4.0 * scale))
                                    .left(px(frame_name_offset))
                                    .max_w(px((descriptor_width
                                        - frame_name_offset
                                        - 12.0 * scale)
                                        .max(1.0)))
                                    .max_h(px(descriptor_height - 6.0 * scale))
                                    .overflow_hidden()
                                    .items_start()
                            })
                            .when(notation == UmlNotation::CombinedFragment, |element| {
                                element.top(px(32.0 * scale))
                            })
                            .when(
                                !external_label
                                    && !heading
                                    && !is_diagram_frame
                                    && notation != UmlNotation::InteractionOperand,
                                |element| element.top_0().bottom_0().items_center(),
                            )
                            .when(
                                heading && alignment.0.text_vertical_align.is_some(),
                                |element| match notation {
                                    UmlNotation::Lifeline => element.bottom(gpui::relative(0.78)),
                                    UmlNotation::Partition => element.bottom(gpui::relative(0.84)),
                                    UmlNotation::CombinedFragment => {
                                        element.bottom(gpui::relative(0.82))
                                    }
                                    UmlNotation::TimingState => {
                                        element.bottom(gpui::relative(0.75))
                                    }
                                    _ => element.bottom(px(8.0 * scale)),
                                },
                            )
                            .flex()
                            .flex_col()
                            .items_center()
                            .justify_center()
                            .when_some(keyword, |element, label| {
                                element.child(
                                    div()
                                        .text_size(px(label_size))
                                        .text_color(label_color)
                                        .child(label),
                                )
                            })
                            .when_some(applied_stereotype, |element, label| {
                                element.child(
                                    div()
                                        .text_size(px(label_size))
                                        .text_color(label_color)
                                        .child(label),
                                )
                            })
                            .flex()
                            .flex_col()
                            .map(|element| alignment.align_column(element))
                            .when(
                                (is_diagram_frame || notation == UmlNotation::InteractionOperand)
                                    && alignment.0.text_align.is_none(),
                                |element| element.items_start(),
                            )
                            .child(title_area)
                            .when(
                                notation == UmlNotation::Instance && !self.slot_labels.is_empty(),
                                |element| {
                                    element.child(
                                        div()
                                            .w_full()
                                            .mt(px(6.0 * scale))
                                            .pt(px(4.0 * scale))
                                            .border_t_1()
                                            .border_color(border)
                                            .text_size(px(slot_font_size))
                                            .text_color(label_color)
                                            .children(
                                                self.slot_labels
                                                    .into_iter()
                                                    .map(|label| div().child(label)),
                                            ),
                                    )
                                },
                            ),
                    )
                    .into_any_element()
            }
            _ => div()
                .size_full()
                .flex()
                .flex_col()
                .map(|element| alignment.align_column(element))
                .child(title_area)
                .into_any_element(),
        };
        div()
            .size_full()
            .relative()
            .map(|element| alignment.apply_surface(element, scale))
            .when_some(font_family, |element, family| element.font_family(family))
            .child(figure)
            .when_some(signature, |element, signature| element.child(signature))
            .into_any_element()
    }
}
