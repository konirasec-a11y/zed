use bellatrix_mde_model::{Geometry, PathSegment, Point};

#[test]
fn translating_cubic_preserves_relative_curve_shape() {
    let mut path = Geometry::Path {
        start: Point { x: 1.0, y: 2.0 },
        segments: vec![PathSegment::Cubic {
            control1: Point { x: 3.0, y: 4.0 },
            control2: Point { x: 5.0, y: 6.0 },
            to: Point { x: 7.0, y: 8.0 },
        }],
    };
    path.translate(20.0, -10.0);
    assert_eq!(
        path,
        Geometry::Path {
            start: Point { x: 21.0, y: -8.0 },
            segments: vec![PathSegment::Cubic {
                control1: Point { x: 23.0, y: -6.0 },
                control2: Point { x: 25.0, y: -4.0 },
                to: Point { x: 27.0, y: -2.0 },
            }],
        }
    );
}
