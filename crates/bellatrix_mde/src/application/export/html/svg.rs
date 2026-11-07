//! Headless SVG rendering; geometry uses domain coordinates, never GPUI pixels.
use super::document::{
    DiagramPage, Documentation, Geometry, PathSegment, RenderedView, TextAlignment,
    TextVerticalAlignment,
};
use super::pages::{esc, safe_id};

fn color(input: Option<&str>, fallback: &str) -> String {
    let Some(input) = input else {
        return fallback.into();
    };
    let hex = input.starts_with('#')
        && matches!(input.len(), 4 | 5 | 7 | 9)
        && input[1..].bytes().all(|byte| byte.is_ascii_hexdigit());
    if hex || ["none", "black", "white", "transparent"].contains(&input) {
        input.into()
    } else {
        fallback.into()
    }
}
fn render_view(project: &Documentation, view: &RenderedView, name: &str) -> String {
    let fill = color(view.style.fill.as_deref(), "#303641");
    let stroke = color(view.style.stroke.as_deref(), "#788397");
    let text_fill = color(view.style.text_color.as_deref(), "#f2f5f9");
    let line_width = view.style.stroke_width.unwrap_or(1.5);
    let opacity = view.style.opacity.unwrap_or(1.0);
    let font_size = view.style.font_size.unwrap_or(13.0);
    let radius = view.style.corner_radius.unwrap_or(3.0);
    let dashes = view
        .style
        .dash_pattern
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(",");
    let dash_attr = if dashes.is_empty() {
        String::new()
    } else {
        format!(" stroke-dasharray=\"{dashes}\"")
    };
    let font_family = esc(view.style.font_family.as_deref().unwrap_or("system-ui"));
    let shape = match &view.geometry {
        Geometry::Layout => format!(
            "<g data-view-id=\"{}\" data-layout-status=\"unresolved\"><desc>View {} has unresolved layout. Authored bounds are preserved; no replacement shape is drawn.</desc></g>",
            esc(&view.id),
            esc(&view.id),
        ),
        Geometry::Rect {
            x,
            y,
            width,
            height,
        } => format!(
            "<rect x=\"{x}\" y=\"{y}\" width=\"{width}\" height=\"{height}\" rx=\"{radius}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{line_width}\"{dash_attr}/>{text}",
            text = render_text(
                view,
                (*x, *y, *width, *height),
                name,
                font_size,
                &font_family,
                &text_fill
            )
        ),
        Geometry::Ellipse { cx, cy, rx, ry } => format!(
            "<ellipse cx=\"{cx}\" cy=\"{cy}\" rx=\"{rx}\" ry=\"{ry}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{line_width}\"{dash_attr}/>{text}",
            text = render_text(
                view,
                (cx - rx, cy - ry, rx * 2.0, ry * 2.0),
                name,
                font_size,
                &font_family,
                &text_fill
            )
        ),
        Geometry::Polygon { points } => {
            let coordinates = points
                .iter()
                .map(|point| format!("{},{}", point.x, point.y))
                .collect::<Vec<_>>()
                .join(" ");
            format!(
                "<polygon points=\"{coordinates}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{line_width}\"{dash_attr}/>"
            )
        }
        Geometry::Path { start, segments } => {
            let mut path = format!("M {} {}", start.x, start.y);
            for segment in segments {
                match segment {
                    PathSegment::Line { to } => path.push_str(&format!(" L {} {}", to.x, to.y)),
                    PathSegment::Cubic {
                        control1,
                        control2,
                        to,
                    } => path.push_str(&format!(
                        " C {} {}, {} {}, {} {}",
                        control1.x, control1.y, control2.x, control2.y, to.x, to.y
                    )),
                }
            }
            format!(
                "<path d=\"{path}\" fill=\"none\" stroke=\"{stroke}\" stroke-width=\"{line_width}\"{dash_attr}/>"
            )
        }
    };
    let shadow = if view.style.show_shadow == Some(true) {
        " filter=\"url(#view-shadow)\""
    } else {
        ""
    };
    let shape = format!("<g opacity=\"{opacity}\"{shadow}>{shape}</g>");
    if let Some(subject) = &view.subject
        && project.element(subject).is_some()
    {
        return format!(
            "<a href=\"../elements/{}.html\">{shape}</a>",
            safe_id(subject)
        );
    }
    shape
}
fn render_text(
    view: &RenderedView,
    bounds: (f64, f64, f64, f64),
    text: &str,
    size: f64,
    family: &str,
    color: &str,
) -> String {
    let (left, top, width, height) = bounds;
    let padding = 6.0_f64.min(width / 2.0).min(height / 2.0);
    let (x, anchor) = match view.style.text_align {
        Some(TextAlignment::Left) => (left + padding, "start"),
        Some(TextAlignment::Right) => (left + width - padding, "end"),
        _ => (left + width / 2.0, "middle"),
    };
    let (y, baseline) = match view.style.text_vertical_align {
        Some(TextVerticalAlignment::Top) => (top + padding, "hanging"),
        Some(TextVerticalAlignment::Bottom) => (top + height - padding, "text-after-edge"),
        _ => (top + height / 2.0, "central"),
    };
    format!(
        "<text x=\"{x}\" y=\"{y}\" text-anchor=\"{anchor}\" dominant-baseline=\"{baseline}\" font-size=\"{size}\" font-family=\"{family}\" fill=\"{color}\">{}</text>",
        esc(text)
    )
}
pub fn draw(project: &Documentation, diagram: &DiagramPage) -> String {
    let mut result = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {} {}\" role=\"img\" aria-label=\"{}\">",
        diagram.width,
        diagram.height,
        esc(&diagram.name)
    );
    result.push_str("<defs><filter id=\"view-shadow\" x=\"-20%\" y=\"-20%\" width=\"150%\" height=\"150%\"><feDropShadow dx=\"2\" dy=\"3\" stdDeviation=\"2\" flood-opacity=\"0.25\"/></filter></defs>");
    result.push_str("<rect width=\"100%\" height=\"100%\" fill=\"#24282e\"/>");
    let mut views = diagram.views.iter().collect::<Vec<_>>();
    let unresolved_layouts = views
        .iter()
        .filter(|view| matches!(view.geometry, Geometry::Layout))
        .count();
    views.sort_by_key(|view| view.z_index);
    for view in views {
        result.push_str(&render_view(project, view, &view.label));
    }
    if unresolved_layouts > 0 {
        result.push_str(&format!(
            "<g role=\"note\" data-export-warning=\"unresolved-layout\"><title>Incomplete diagram rendering</title><text x=\"12\" y=\"24\" font-family=\"system-ui\" font-size=\"14\" fill=\"#ffd166\">Warning: {unresolved_layouts} view(s) have unresolved layout and are not drawn. Resolved subviews are included.</text></g>"
        ));
    }
    result.push_str("</svg>");
    result
}
