use crate::class::ViewAppearance;
use bellatrix_design_system::Palette;
use bellatrix_mde::{UmlNotation, folded_note_layout, frame_heading_layout};
use gpui::{Bounds, PathBuilder, Pixels, Window, point, px};

pub(crate) fn paint_folded_paper(
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
    let dashes = appearance.resolve_stroke_pattern(false, scale);
    let mut border = PathBuilder::stroke(px(appearance.width(1.0, scale, selected)));
    if !dashes.is_empty() {
        border = border.dash_array(&dashes);
    }
    border.add_polygon(&outline, true);
    border.add_polygon(&fold, false);
    match border.build() {
        Ok(path) => window.paint_path(path, stroke),
        Err(error) => log::error!("UML paper outline: {error}"),
    }
}
pub(crate) fn paint_frame_heading(
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
    let dashes = appearance.resolve_stroke_pattern(false, scale);
    let mut border = PathBuilder::stroke(px(appearance.width(1.0, scale, selected)));
    if !dashes.is_empty() {
        border = border.dash_array(&dashes);
    }
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
fn build_figure_paths(
    notation: UmlNotation,
    width: f32,
    height: f32,
    descriptor: (f32, f32),
) -> Vec<FigurePath> {
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
        InteractionOperand => vec![line(&[(0.02, 0.02), (0.98, 0.02)]).dashed()],
        InteractionUse | CombinedFragment => {
            let left = (1.0 / width).min(0.25);
            let top = (1.0 / height).min(0.25);
            let right = (descriptor.0 / width).clamp(left, 1.0 - left);
            let bottom = (descriptor.1 / height).clamp(top, 1.0 - top);
            let cut_x = (8.0 / width).min(right / 3.0);
            let cut_y = (8.0 / height).min(bottom / 3.0);
            vec![
                FigurePath::outline(
                    &[
                        (left, top),
                        (1.0 - left, top),
                        (1.0 - left, 1.0 - top),
                        (left, 1.0 - top),
                    ],
                    true,
                ),
                line(&[
                    (left, bottom),
                    (right - cut_x, bottom),
                    (right, bottom - cut_y),
                    (right, top),
                ]),
            ]
        }
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
pub(crate) fn paint_notation(
    bounds: Bounds<Pixels>,
    notation: UmlNotation,
    scale: f32,
    descriptor: (f32, f32),
    palette: Palette,
    selected: bool,
    appearance: &ViewAppearance,
    window: &mut Window,
) {
    let width = f32::from(bounds.size.width).max(1.0);
    let height = f32::from(bounds.size.height).max(1.0);
    let stroke = appearance.stroke(palette, selected);
    for path in build_figure_paths(notation, width, height, descriptor) {
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
                    if path.solid && appearance.0.fill.is_none() {
                        stroke
                    } else {
                        appearance.fill(palette.panel)
                    },
                ),
                Err(error) => log::error!("UML notation fill: {error}"),
            }
        }
        let mut outline = PathBuilder::stroke(px(appearance.width(1.0, scale, selected)));
        let dashes = appearance.resolve_stroke_pattern(path.dashed, scale);
        if !dashes.is_empty() {
            outline = outline.dash_array(&dashes);
        }
        outline.add_polygon(&points, path.closed);
        match outline.build() {
            Ok(path_geometry) => window.paint_path(path_geometry, stroke),
            Err(error) => log::error!("UML notation outline: {error}"),
        }
    }
}
