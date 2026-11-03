use crate::class::ViewAppearance;
use bellatrix_design_system::Palette;
use bellatrix_mde::{TimingMode, TimingPoint, TimingPresentation};
use gpui::{AnyElement, IntoElement, PathBuilder, canvas, div, point, prelude::*, px, relative};

pub(crate) struct TimingLifeline {
    pub(crate) title: AnyElement,
    pub(crate) presentation: Option<TimingPresentation>,
    pub(crate) appearance: ViewAppearance,
    pub(crate) palette: Palette,
    pub(crate) selected: bool,
    pub(crate) scale: f32,
    pub(crate) width: f64,
    pub(crate) height: f64,
}

struct TimingLabel {
    text: String,
    left: f32,
    top: f32,
    width: f32,
    height: f32,
    state: bool,
}

struct TimingLayout {
    width: f32,
    height: f32,
    paths: Vec<Vec<(f32, f32)>>,
    labels: Vec<TimingLabel>,
}

impl TimingLifeline {
    pub(crate) fn render(self) -> AnyElement {
        let presentation = self.presentation.unwrap_or(TimingPresentation {
            mode: TimingMode::State,
            points: Vec::new(),
        });
        let layout = TimingLayout::new(&presentation, self.width, self.height);
        let width = layout.width;
        let height = layout.height;
        let appearance = self.appearance;
        let stroke_appearance = appearance.clone();
        let scale = self.scale;
        let palette = self.palette;
        let selected = self.selected;
        let font_family = appearance.0.font_family.clone();
        let labels = layout.labels.into_iter().map(|label| {
            div()
                .absolute()
                .left(relative(label.left / width))
                .top(relative(label.top / height))
                .w(relative(label.width / width))
                .h(relative(label.height / height))
                .overflow_hidden()
                .flex()
                .items_center()
                .justify_center()
                .text_center()
                .when(label.state, |element| element.justify_end().text_right())
                .map(|element| appearance.align_row(element))
                .text_size(px(appearance.font_size(11.0, scale)))
                .text_color(appearance.text(palette.text))
                .child(label.text)
        });
        div()
            .size_full()
            .relative()
            .map(|element| appearance.apply_surface(element, scale))
            .when_some(font_family, |element, family| element.font_family(family))
            .when(appearance.0.fill.is_some(), |element| {
                element.bg(appearance.fill(palette.canvas))
            })
            .child(
                canvas(
                    |_, _, _| (),
                    move |bounds, (), window, _| {
                        let horizontal_scale = f32::from(bounds.size.width) / width;
                        let vertical_scale = f32::from(bounds.size.height) / height;
                        let dashes = stroke_appearance.resolve_stroke_pattern(false, scale);
                        let mut path =
                            PathBuilder::stroke(px(stroke_appearance.width(1.0, scale, selected)));
                        if !dashes.is_empty() {
                            path = path.dash_array(&dashes);
                        }
                        for vertices in layout.paths {
                            let points = vertices
                                .into_iter()
                                .map(|(x, y)| {
                                    point(
                                        bounds.origin.x + px(x * horizontal_scale),
                                        bounds.origin.y + px(y * vertical_scale),
                                    )
                                })
                                .collect::<Vec<_>>();
                            path.add_polygon(&points, false);
                        }
                        match path.build() {
                            Ok(path) => {
                                window.paint_path(path, stroke_appearance.stroke(palette, selected))
                            }
                            Err(error) => log::error!("UML timing lifeline: {error}"),
                        }
                    },
                )
                .absolute()
                .size_full(),
            )
            .children(labels)
            .child(
                div()
                    .absolute()
                    .left(px(8.0 * scale))
                    .right(px(8.0 * scale))
                    .top(px(4.0 * scale))
                    .h(px(36.0 * scale))
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .map(|element| appearance.align_column(element))
                    .child(self.title),
            )
            .into_any_element()
    }
}

impl TimingLayout {
    fn new(presentation: &TimingPresentation, width: f64, height: f64) -> Self {
        let width = width as f32;
        let height = height as f32;
        let width = if width.is_finite() {
            width.max(1.0)
        } else {
            480.0
        };
        let height = if height.is_finite() {
            height.max(1.0)
        } else {
            180.0
        };
        let left = match presentation.mode {
            TimingMode::State => (width * 0.26).min(140.0),
            TimingMode::Value => (width * 0.04).min(20.0),
        };
        let right = width * 0.97;
        let top = (height * 0.28).max(34.0).min(height * 0.50);
        let bottom = height * 0.70;
        let axis = height * 0.82;
        let mut layout = Self {
            width,
            height,
            paths: vec![vec![(left, axis), (right, axis)]],
            labels: Vec::new(),
        };
        if let Err(error) = presentation.validate() {
            log::error!("UML timing presentation: {error}");
            return layout;
        }
        let Some(first) = presentation.points.first() else {
            return layout;
        };
        let (minimum, maximum) = presentation
            .points
            .iter()
            .fold((first.time, first.time), |(minimum, maximum), point| {
                (minimum.min(point.time), maximum.max(point.time))
            });
        // Normalize before subtracting so finite authored extremes cannot overflow the range.
        let magnitude = minimum.abs().max(maximum.abs()).max(1.0);
        let minimum = minimum / magnitude;
        let range = maximum / magnitude - minimum;
        // The unlabelled right tail keeps the final value legible without inventing another event.
        let tail = match presentation.mode {
            TimingMode::State => 12.0_f32.min((right - left) * 0.10),
            TimingMode::Value => 70.0_f32.min((right - left) * 0.25),
        };
        let positions = presentation
            .points
            .iter()
            .map(|point| {
                let fraction = if range > 0.0 {
                    ((point.time / magnitude - minimum) / range) as f32
                } else {
                    0.0
                };
                left + fraction * (right - left - tail)
            })
            .collect::<Vec<_>>();
        let mut tick_times = Vec::new();
        for (point, x) in presentation.points.iter().zip(&positions) {
            if tick_times.contains(&point.time) {
                continue;
            }
            tick_times.push(point.time);
            layout.paths.push(vec![(*x, axis - 3.0), (*x, axis + 3.0)]);
            layout.labels.push(TimingLabel {
                text: point.time.to_string(),
                left: (*x - 30.0).max(0.0),
                top: axis + 4.0,
                width: 60.0_f32.min(width - (*x - 30.0).max(0.0)),
                height: (height - axis - 4.0).max(0.0),
                state: false,
            });
        }
        match presentation.mode {
            TimingMode::State => layout.build_state_trace(
                &presentation.points,
                &positions,
                left,
                right,
                (top, bottom),
            ),
            TimingMode::Value => {
                layout.build_value_trace(&presentation.points, &positions, right, (top, bottom))
            }
        }
        layout
    }

    fn build_state_trace(
        &mut self,
        points: &[TimingPoint],
        positions: &[f32],
        left: f32,
        right: f32,
        vertical: (f32, f32),
    ) {
        let mut values = Vec::new();
        for point in points {
            if !values.contains(&point.value.as_str()) {
                values.push(point.value.as_str());
            }
        }
        let row = |index: usize| {
            if values.len() == 1 {
                (vertical.0 + vertical.1) / 2.0
            } else {
                vertical.0 + (vertical.1 - vertical.0) * index as f32 / (values.len() - 1) as f32
            }
        };
        let row_height = ((vertical.1 - vertical.0) / values.len().max(1) as f32).min(24.0);
        for (index, value) in values.iter().enumerate() {
            self.labels.push(TimingLabel {
                text: (*value).to_owned(),
                left: 6.0,
                top: row(index) - row_height / 2.0,
                width: (left - 14.0).max(0.0),
                height: row_height,
                state: true,
            });
        }
        let mut trace = Vec::new();
        let mut previous_y = None;
        for (point, x) in points.iter().zip(positions) {
            if let Some(index) = values.iter().position(|value| *value == point.value) {
                let y = row(index);
                if let Some(previous_y) = previous_y {
                    trace.push((*x, previous_y));
                }
                trace.push((*x, y));
                previous_y = Some(y);
            }
        }
        if let Some(y) = previous_y {
            trace.push((right, y));
            self.paths.push(trace);
        }
    }

    fn build_value_trace(
        &mut self,
        points: &[TimingPoint],
        positions: &[f32],
        right: f32,
        vertical: (f32, f32),
    ) {
        let Some((first, first_x)) = points.first().zip(positions.first()) else {
            return;
        };
        let center = (vertical.0 + vertical.1) / 2.0;
        let half_height = ((vertical.1 - vertical.0) / 2.0).min(16.0);
        let mut upper_y = center - half_height;
        let mut lower_y = center + half_height;
        let mut upper = vec![(*first_x, upper_y)];
        let mut lower = vec![(*first_x, lower_y)];
        let mut current_value = &first.value;
        let mut label_start = *first_x;
        let mut previous_x = *first_x;
        for (index, (point, x)) in points.iter().zip(positions).enumerate().skip(1) {
            if point.value == *current_value {
                previous_x = *x;
                continue;
            }
            let next_x = positions.get(index + 1).copied().unwrap_or(right);
            let crossing = 7.0_f32
                .min((*x - previous_x).abs() / 3.0)
                .min((next_x - *x).abs() / 3.0);
            let direction = if *x < previous_x { -1.0 } else { 1.0 };
            let before = *x - crossing * direction;
            let after = *x + crossing * direction;
            upper.extend([(before, upper_y), (after, lower_y)]);
            lower.extend([(before, lower_y), (after, upper_y)]);
            self.add_value_label(current_value, label_start, before, center, half_height);
            std::mem::swap(&mut upper_y, &mut lower_y);
            current_value = &point.value;
            label_start = after;
            previous_x = *x;
        }
        upper.push((right, upper_y));
        lower.push((right, lower_y));
        self.add_value_label(current_value, label_start, right, center, half_height);
        self.paths.extend([upper, lower]);
    }

    fn add_value_label(
        &mut self,
        value: &str,
        start: f32,
        end: f32,
        center: f32,
        half_height: f32,
    ) {
        self.labels.push(TimingLabel {
            text: value.to_owned(),
            left: start.min(end) + 3.0,
            top: center - half_height + 1.0,
            width: ((end - start).abs() - 6.0).max(0.0),
            height: (half_height * 2.0 - 2.0).max(0.0),
            state: false,
        });
    }
}
