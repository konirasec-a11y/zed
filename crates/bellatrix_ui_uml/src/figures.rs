//! Non-classifier notations share editing, but not the classifier silhouette.
use crate::{
    class::{MemberInteractions, MemberRow, UmlClassifier, ViewAppearance, template_signature},
    inline_edit::UmlInlineNameEditor,
};
use bellatrix_design_system::Palette;
use bellatrix_mde::{
    UmlNotation, folded_note_layout, frame_heading_layout, port_label_layout, uml_header,
    uml_notation,
};
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
    pseudostate_kind: Option<String>,
    name: SharedString,
    applied_stereotype: Option<SharedString>,
    selected: bool,
    scale: f32,
    view_width: f64,
    appearance: ViewAppearance,
    edit: Option<Action>,
    editor: Option<(Entity<InputField>, Action, Action)>,
    template_parameters: Option<(Vec<MemberRow>, MemberInteractions, f64)>,
}
impl UmlFigure {
    pub fn supports(kind: &str) -> bool {
        kind == "uml.pseudostate"
            || matches!(
                uml_notation(kind),
                UmlNotation::PackageTab
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
                    | UmlNotation::ObjectNode
                    | UmlNotation::SendSignal
                    | UmlNotation::AcceptEvent
                    | UmlNotation::Partition
                    | UmlNotation::TimingState
                    | UmlNotation::History
                    | UmlNotation::Terminate
            )
    }
    pub fn new(kind: impl Into<String>, name: impl Into<SharedString>) -> Self {
        Self {
            kind: kind.into(),
            pseudostate_kind: None,
            name: name.into(),
            applied_stereotype: None,
            selected: false,
            scale: 1.0,
            view_width: 16.0,
            appearance: ViewAppearance::default(),
            edit: None,
            editor: None,
            template_parameters: None,
        }
    }
    pub fn pseudostate_kind(mut self, kind: &str) -> Self {
        self.pseudostate_kind = Some(kind.to_owned());
        self
    }
    fn notation(&self) -> UmlNotation {
        if self.kind != "uml.pseudostate" {
            return uml_notation(&self.kind);
        }
        match self.pseudostate_kind.as_deref().unwrap_or("initial") {
            "initial" | "junction" => UmlNotation::InitialNode,
            "choice" => UmlNotation::Decision,
            "shallowHistory" | "deepHistory" | "entryPoint" => UmlNotation::History,
            "exitPoint" => UmlNotation::FlowFinal,
            "terminate" => UmlNotation::Terminate,
            "fork" | "join" => UmlNotation::Fork,
            _ => UmlNotation::Unsupported,
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
    pub fn view_size(mut self, width: f64, _height: f64, scale: f32) -> Self {
        if width.is_finite() && width > 0.0 {
            self.view_width = width;
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
fn paint_frame_heading(
    bounds: Bounds<Pixels>,
    scale: f32,
    palette: Palette,
    selected: bool,
    appearance: &ViewAppearance,
    window: &mut Window,
) {
    let layout = frame_heading_layout(
        f64::from(bounds.size.width) / f64::from(scale),
        f64::from(bounds.size.height) / f64::from(scale),
    );
    let outline = layout.outline.map(|position| {
        point(
            bounds.origin.x + px(position.x as f32 * scale),
            bounds.origin.y + px(position.y as f32 * scale),
        )
    });
    let mut fill = PathBuilder::fill();
    fill.add_polygon(&outline, true);
    match fill.build() {
        Ok(path) => window.paint_path(path, appearance.fill(palette.panel_alt)),
        Err(error) => log::error!("UML frame heading fill: {error}"),
    }
    let mut border = PathBuilder::stroke(px(appearance.width(1.0, scale, selected)));
    border.add_polygon(&outline, true);
    match border.build() {
        Ok(path) => window.paint_path(path, appearance.stroke(palette, selected)),
        Err(error) => log::error!("UML frame heading outline: {error}"),
    }
}
// UML 2.5.1 §§9, 11, 14–19: these are notation outlines, not executable model state.
#[derive(Debug)]
struct FigurePath {
    points: Vec<(f32, f32)>,
    closed: bool,
    solid: bool,
    dashed: bool,
}
impl FigurePath {
    fn outline(points: &[(f32, f32)], closed: bool) -> Self {
        Self {
            points: points.to_vec(),
            closed,
            solid: false,
            dashed: false,
        }
    }
    fn ellipse(center: (f32, f32), radii: (f32, f32)) -> Self {
        let points = (0..64)
            .map(|index| {
                let angle = std::f32::consts::TAU * index as f32 / 64.0;
                (
                    center.0 + radii.0 * angle.cos(),
                    center.1 + radii.1 * angle.sin(),
                )
            })
            .collect();
        Self {
            points,
            closed: true,
            solid: false,
            dashed: false,
        }
    }
    fn solid(mut self) -> Self {
        self.solid = true;
        self
    }
    fn dashed(mut self) -> Self {
        self.dashed = true;
        self
    }
}
fn figure_paths(notation: UmlNotation, width: f32, height: f32) -> Vec<FigurePath> {
    use UmlNotation::*;
    let rectangle = || {
        FigurePath::outline(
            &[(0.02, 0.02), (0.98, 0.02), (0.98, 0.98), (0.02, 0.98)],
            true,
        )
    };
    let line = |points: &[(f32, f32)]| FigurePath::outline(points, false);
    let circle = |radius: f32| {
        let diameter = width.min(height) * radius;
        FigurePath::ellipse((0.5, 0.5), (diameter / width, diameter / height))
    };
    match notation {
        Actor => {
            let radius = (width * 0.14).min(height * 0.13);
            vec![
                FigurePath::ellipse((0.5, 0.16), (radius / width, radius / height)),
                line(&[(0.5, 0.29), (0.5, 0.61)]),
                line(&[(0.22, 0.39), (0.78, 0.39)]),
                line(&[(0.24, 0.88), (0.5, 0.61), (0.76, 0.88)]),
            ]
        }
        UseCase => vec![FigurePath::ellipse((0.5, 0.5), (0.48, 0.48))],
        Collaboration => vec![FigurePath::ellipse((0.5, 0.5), (0.48, 0.48)).dashed()],
        DeploymentNode => vec![
            FigurePath::outline(
                &[
                    (0.02, 0.16),
                    (0.18, 0.02),
                    (0.98, 0.02),
                    (0.98, 0.84),
                    (0.82, 0.98),
                    (0.02, 0.98),
                ],
                true,
            ),
            line(&[(0.02, 0.16), (0.82, 0.16), (0.98, 0.02)]),
            line(&[(0.82, 0.16), (0.82, 0.98)]),
        ],
        InitialNode => vec![circle(0.43).solid()],
        FinalNode => vec![circle(0.46), circle(0.30).solid()],
        FlowFinal | History => {
            let mut paths = vec![circle(0.46)];
            if notation == FlowFinal {
                let radius = width.min(height) * 0.29;
                let horizontal = radius / width;
                let vertical = radius / height;
                paths.push(line(&[
                    (0.5 - horizontal, 0.5 - vertical),
                    (0.5 + horizontal, 0.5 + vertical),
                ]));
                paths.push(line(&[
                    (0.5 - horizontal, 0.5 + vertical),
                    (0.5 + horizontal, 0.5 - vertical),
                ]));
            }
            paths
        }
        Terminate => vec![
            line(&[(0.12, 0.12), (0.88, 0.88)]),
            line(&[(0.12, 0.88), (0.88, 0.12)]),
        ],
        Decision => vec![FigurePath::outline(
            &[(0.5, 0.02), (0.98, 0.5), (0.5, 0.98), (0.02, 0.5)],
            true,
        )],
        Fork => vec![rectangle().solid()],
        Lifeline => vec![
            FigurePath::outline(
                &[(0.02, 0.02), (0.98, 0.02), (0.98, 0.22), (0.02, 0.22)],
                true,
            ),
            line(&[(0.5, 0.22), (0.5, 0.98)]).dashed(),
        ],
        Execution | Instance | ObjectNode => vec![rectangle()],
        SendSignal => vec![FigurePath::outline(
            &[
                (0.02, 0.02),
                (0.76, 0.02),
                (0.98, 0.5),
                (0.76, 0.98),
                (0.02, 0.98),
            ],
            true,
        )],
        AcceptEvent => vec![FigurePath::outline(
            &[
                (0.02, 0.02),
                (0.98, 0.02),
                (0.98, 0.98),
                (0.02, 0.98),
                (0.24, 0.5),
            ],
            true,
        )],
        Partition => vec![rectangle(), line(&[(0.02, 0.16), (0.98, 0.16)])],
        InteractionUse | CombinedFragment => vec![
            rectangle(),
            line(&[(0.02, 0.18), (0.28, 0.18), (0.34, 0.12), (0.34, 0.02)]),
        ],
        TimingState => vec![line(&[
            (0.02, 0.72),
            (0.28, 0.72),
            (0.36, 0.28),
            (0.64, 0.28),
            (0.72, 0.72),
            (0.98, 0.72),
        ])],
        _ => Vec::new(),
    }
}
fn paint_notation(
    bounds: Bounds<Pixels>,
    notation: UmlNotation,
    scale: f32,
    palette: Palette,
    selected: bool,
    appearance: &ViewAppearance,
    window: &mut Window,
) {
    let width = f32::from(bounds.size.width).max(1.0);
    let height = f32::from(bounds.size.height).max(1.0);
    let stroke = appearance.stroke(palette, selected);
    for path in figure_paths(notation, width, height) {
        let points = path
            .points
            .iter()
            .map(|(x, y)| {
                point(
                    bounds.origin.x + px(x * width),
                    bounds.origin.y + px(y * height),
                )
            })
            .collect::<Vec<_>>();
        if path.closed {
            let mut fill = PathBuilder::fill();
            fill.add_polygon(&points, true);
            match fill.build() {
                Ok(path_geometry) => window.paint_path(
                    path_geometry,
                    if path.solid {
                        stroke
                    } else {
                        appearance.fill(palette.panel)
                    },
                ),
                Err(error) => log::error!("UML notation fill: {error}"),
            }
        }
        let mut outline = PathBuilder::stroke(px(appearance.width(1.0, scale, selected)));
        let dashes = appearance
            .0
            .dash_pattern
            .iter()
            .filter(|value| value.is_finite() && **value > 0.0)
            .map(|value| px(*value as f32 * scale))
            .collect::<Vec<_>>();
        if !dashes.is_empty() {
            outline = outline.dash_array(&dashes);
        } else if path.dashed {
            outline = outline.dash_array(&[px(6.0 * scale), px(4.0 * scale)]);
        }
        outline.add_polygon(&points, path.closed);
        match outline.build() {
            Ok(path_geometry) => window.paint_path(path_geometry, stroke),
            Err(error) => log::error!("UML notation outline: {error}"),
        }
    }
}

impl RenderOnce for UmlFigure {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let notation = self.notation();
        let palette = Palette::current(cx);
        let scale = self.scale;
        let port_label = port_label_layout(self.view_width);
        let selected = self.selected;
        let signature = self
            .template_parameters
            .and_then(|(rows, interactions, width)| {
                template_signature(rows, width, interactions, palette, selected)
            });
        let appearance = self.appearance;
        let alignment = appearance.clone();
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
            .map(|element| alignment.align_text(element))
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
            .when(notation == UmlNotation::Instance, |element| {
                element.underline()
            })
            .child(title);
        let figure = match notation {
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
                            .bottom(px(8.0 * scale))
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
                .border(px(appearance.width(1.0, scale, selected)))
                .border_color(border)
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
                        .map(|element| alignment.align_row(element))
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
                        .max_w_full()
                        .max_h_full()
                        .relative()
                        .child(
                            canvas(
                                |_, _, _| (),
                                move |bounds, (), window, _| {
                                    paint_frame_heading(
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
                                .left(px(8.0 * scale))
                                .right(px(16.0 * scale))
                                .top_0()
                                .bottom_0()
                                .flex()
                                .items_center()
                                .overflow_hidden()
                                .map(|element| alignment.align_row(element))
                                .child(title_area),
                        ),
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
                        .flex()
                        .flex_col()
                        .map(|element| alignment.align_column(element))
                        .child(title_area),
                )
                .into_any_element(),
            UmlNotation::Action | UmlNotation::State => div()
                .size_full()
                .rounded(px(appearance.0.corner_radius.unwrap_or(14.0) as f32 * scale))
                .border(px(appearance.width(1.0, scale, selected)))
                .border_color(border)
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
            UmlNotation::Actor
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
                let history_label = match self.pseudostate_kind.as_deref() {
                    Some("deepHistory") => "H*",
                    Some("entryPoint") => "",
                    _ => "H",
                };
                let keyword = uml_header(&self.kind, None).keyword;
                let label_color = appearance.text(palette.text);
                let label_size = appearance.font_size(10.0, scale);
                let frame_keyword = if self.kind == "uml.interaction" {
                    "sd"
                } else {
                    "ref"
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
                    .when(notation == UmlNotation::InteractionUse, |element| {
                        element.child(
                            div()
                                .absolute()
                                .left(px(8.0 * scale))
                                .top(px(4.0 * scale))
                                .text_size(px(10.0 * scale))
                                .text_color(border)
                                .child(frame_keyword),
                        )
                    })
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
                            .when(!external_label && !heading, |element| {
                                element.top_0().bottom_0().items_center()
                            })
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
                            .when_some(self.applied_stereotype, |element, label| {
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
                            .child(title_area),
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
    fn profile_and_extended_notations_have_dedicated_figures() {
        assert!(UmlFigure::supports("uml.profile"));
        for kind in [
            "uml.node",
            "uml.instance_specification",
            "uml.collaboration",
        ] {
            assert!(UmlFigure::supports(kind));
        }
    }


}
