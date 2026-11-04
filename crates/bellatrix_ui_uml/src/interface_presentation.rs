use crate::class::ViewAppearance;
use bellatrix_design_system::Palette;
use bellatrix_mde_cordis::LineMode;
use bellatrix_mde_cordis::presentation_contract::InterfacePresentation;
use bellatrix_mde_cordis::projection::InterfaceIconGlyph;
use gpui::{
    AnyElement, Bounds, IntoElement, PathBuilder, Pixels, SharedString, Window, canvas, div, point,
    prelude::*, px,
};

pub fn render_interface_icon(
    glyphs: Vec<InterfaceIconGlyph>,
    appearance: ViewAppearance,
    palette: Palette,
    selected: bool,
    scale: f32,
) -> AnyElement {
    canvas(
        |_, _, _| (),
        move |bounds, (), window, _| {
            if !scale.is_finite() || scale <= 0.0 {
                return;
            }
            for glyph in &glyphs {
                let (center, radius, start, sweep, filled) = match glyph {
                    InterfaceIconGlyph::Ball { center, radius } => {
                        (*center, *radius, 0.0, std::f64::consts::TAU, true)
                    }
                    InterfaceIconGlyph::Socket {
                        center,
                        radius,
                        start_angle,
                        sweep_angle,
                    } => (*center, *radius, *start_angle, *sweep_angle, false),
                };
                if radius <= 0.0
                    || ![center.x, center.y, radius, start, sweep]
                        .iter()
                        .all(|value| value.is_finite())
                {
                    continue;
                }
                let steps = if filled { 64 } else { 32 };
                let points = (0..=steps)
                    .map(|step| {
                        let angle = start + sweep * f64::from(step) / f64::from(steps);
                        let x = (center.x + radius * angle.cos()) * f64::from(scale);
                        let y = (center.y + radius * angle.sin()) * f64::from(scale);
                        if !x.is_finite()
                            || !y.is_finite()
                            || !(x as f32).is_finite()
                            || !(y as f32).is_finite()
                        {
                            return None;
                        }
                        Some(point(
                            bounds.origin.x + px(x as f32),
                            bounds.origin.y + px(y as f32),
                        ))
                    })
                    .collect::<Option<Vec<_>>>();
                let Some(points) = points else {
                    continue;
                };
                if filled {
                    let mut fill = PathBuilder::fill();
                    fill.add_polygon(&points, true);
                    match fill.build() {
                        Ok(path) => window.paint_path(path, appearance.fill(palette.panel)),
                        Err(error) => log::error!("UML interface icon fill: {error}"),
                    }
                }
                let mut outline = PathBuilder::stroke(px(appearance.width(1.0, scale, selected)));
                let pattern = appearance.resolve_stroke_pattern(LineMode::Solid, scale);
                if !pattern.is_empty() {
                    outline = outline.dash_array(&pattern);
                }
                outline.add_polygon(&points, filled);
                match outline.build() {
                    Ok(path) => window.paint_path(path, appearance.stroke(palette, selected)),
                    Err(error) => log::error!("UML interface icon outline: {error}"),
                }
            }
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
    .into_any_element()
}

pub(crate) struct InterfaceFigure {
    pub(crate) presentation: InterfacePresentation,
    pub(crate) title: AnyElement,
    pub(crate) stereotype: Option<SharedString>,
    pub(crate) appearance: ViewAppearance,
    pub(crate) palette: Palette,
    pub(crate) selected: bool,
    pub(crate) scale: f32,
    pub(crate) height: f64,
}

impl InterfaceFigure {
    pub(crate) fn render(self) -> AnyElement {
        let appearance = self.appearance;
        let stroke_appearance = appearance.clone();
        let palette = self.palette;
        let scale = self.scale;
        let selected = self.selected;
        let presentation = self.presentation;
        let symbol_height = (self.height as f32 * 0.5).min(40.0) * scale;
        let font_family = appearance.0.font_family.clone();
        div()
            .size_full()
            .relative()
            .flex()
            .flex_col()
            .map(|element| appearance.apply_surface(element, scale))
            .when_some(font_family, |element, family| element.font_family(family))
            .child(
                canvas(
                    |_, _, _| (),
                    move |bounds, (), window, _| {
                        paint_interface_notation(
                            bounds,
                            presentation,
                            &stroke_appearance,
                            palette,
                            selected,
                            scale,
                            window,
                        );
                    },
                )
                .w_full()
                .h(px(symbol_height))
                .flex_shrink_0(),
            )
            .child(
                div()
                    .min_w_0()
                    .min_h_0()
                    .w_full()
                    .flex_1()
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .text_center()
                    .map(|element| appearance.align_column(element))
                    .when_some(self.stereotype, |element, label| {
                        element.child(
                            div()
                                .w_full()
                                .text_size(px(appearance.font_size(10.0, scale)))
                                .text_color(appearance.text(palette.muted))
                                .map(|element| appearance.align_text(element))
                                .child(label),
                        )
                    })
                    .child(div().w_full().min_w_0().child(self.title)),
            )
            .into_any_element()
    }
}

fn paint_interface_notation(
    bounds: Bounds<Pixels>,
    presentation: InterfacePresentation,
    appearance: &ViewAppearance,
    palette: Palette,
    selected: bool,
    scale: f32,
    window: &mut Window,
) {
    let width = f32::from(bounds.size.width);
    let height = f32::from(bounds.size.height);
    let stroke_width = appearance.width(1.0, scale, selected);
    let inset = stroke_width / 2.0;
    let radius = (12.0 * scale)
        .min((width - stroke_width).max(0.0) / 4.0)
        .min((height - stroke_width - 4.0 * scale).max(0.0) / 2.0);
    if radius <= 0.0 {
        return;
    }
    let center_x = width / 2.0;
    let center_y = height / 2.0;
    let position = |x, y| point(bounds.origin.x + px(x), bounds.origin.y + px(y));
    let provided = matches!(presentation, InterfacePresentation::Provided);
    let start_angle = if provided {
        0.0
    } else {
        std::f32::consts::FRAC_PI_2
    };
    let sweep = if provided {
        std::f32::consts::TAU
    } else {
        std::f32::consts::PI
    };
    let segments = if provided { 64 } else { 32 };
    let arc = (0..=segments)
        .map(|step| {
            let angle = start_angle + sweep * step as f32 / segments as f32;
            position(
                center_x + radius * angle.cos(),
                center_y + radius * angle.sin(),
            )
        })
        .collect::<Vec<_>>();
    // UML 2.5.1 §10.4.4: the required Interface is an open half-circle, never a filled disk.
    if provided {
        let mut fill = PathBuilder::fill();
        fill.add_polygon(&arc, true);
        match fill.build() {
            Ok(path) => window.paint_path(path, appearance.fill(palette.panel)),
            Err(error) => log::error!("UML provided interface fill: {error}"),
        }
    }
    let pattern = appearance.resolve_stroke_pattern(LineMode::Solid, scale);
    let mut outline = PathBuilder::stroke(px(stroke_width));
    if !pattern.is_empty() {
        outline = outline.dash_array(&pattern);
    }
    outline.add_polygon(
        &[
            position(inset, center_y),
            position(center_x - radius, center_y),
        ],
        false,
    );
    outline.add_polygon(&arc, provided);
    match outline.build() {
        Ok(path) => window.paint_path(path, appearance.stroke(palette, selected)),
        Err(error) => log::error!("UML interface outline: {error}"),
    }
}
