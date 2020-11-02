use iced_graphics::canvas::{Fill, FillRule, Frame, LineCap, LineJoin, Path, Stroke};
use iced_graphics::widget::canvas::path::Builder;
use iced_graphics::{self, Backend, Defaults, Primitive};
use iced_native::layout::{Layout, Limits, Node};
use iced_native::{
    self, Align, Background, Clipboard, Column, Event, Hasher, Length, Point, Rectangle, Row, Size, Text,
};
use iced_native::{Color, Vector};
use lyon_geom::{QuadraticBezierSegment, Scalar, Segment};
use smallvec::{smallvec, SmallVec};
use std::ops::Deref;
use std::ops::DerefMut;
use vek::Vec2;

#[derive(Debug)]
pub struct ProjectionResult {
    pub t: f32,
    pub distance: f32,
}

pub trait ConnectionSegment {
    fn build_segment(&self, builder: &mut Builder);

    /// Find the closest point on the segment to the provided point
    fn project_point(&self, query: Vec2<f32>) -> ProjectionResult;
}

/// A Bezier curve segment of degree $`n`$ and points $`\mathbf{P}_0, \mathbf{P}_1, \ldots, \mathbf{P}_n`$ is defined as
/// ```math
/// \mathbf{B}_{n}(t) = \sum_{i=0}^n {n\choose i}(1 - t)^{n - i}t^i\mathbf{P}_i,\quad t \in [0; 1]
/// ```
///
/// A quadratic ($`n = 2`$) Bezier curve segment simplifies to
/// ```math
/// \mathbf{B}_{2}(t) = (1-t)^2\mathbf{P}_0 + 2t(1-t)\mathbf{P}_1 + t^2\mathbf{P}_2
/// ```
impl ConnectionSegment for QuadraticBezierSegment<f32> {
    fn build_segment(&self, builder: &mut Builder) {
        builder.move_to(self.from.to_array().into());
        builder.quadratic_curve_to(self.ctrl.to_array().into(), self.to.to_array().into());
    }

    /// The task of finding the closest point on the curve $`\mathbf{B}_n(t)`$ to point $`\mathbf{Q}`$ consists of finding
    /// $`t`$
    /// ```math
    /// \min_{t\in[0;1]}f_n(t), \quad f_n(t)=\left|\mathbf{B}_n(t)-P\right|^2
    /// ```
    ///
    /// Which, for $`n = 2`$, can be solved by inspecting the roots of the first derivative
    /// ```math
    /// 0 = f'_2(t) = 4\left((1-t)^2\overrightarrow{QP_0} + 2t(1-t)\overrightarrow{QP_1} + t^2\overrightarrow{QP_2}\right)\cdot\newline
    /// \quad\cdot\left((t-1)\overrightarrow{QP_0} + (1-2t)\overrightarrow{QP_1} + t\overrightarrow{QP_2}\right)
    /// ```
    ///
    /// And expanding to get the coefficients for the 3rd degree polynomial
    /// ```math
    /// 0 = f_2(t) = 4[ \newline
    /// \quad t^3(\overrightarrow{QP_0}^2 - 4\overrightarrow{QP_0}\cdot\overrightarrow{QP_1} + 2\overrightarrow{QP_0}\cdot\overrightarrow{QP_2} + 4\overrightarrow{QP_1}^2 - 4\overrightarrow{QP_1}\cdot\overrightarrow{QP_2} + \overrightarrow{QP_2}^2) \newline
    /// \quad - t^2(3\overrightarrow{QP_0}^2 - 9\overrightarrow{QP_0}\cdot\overrightarrow{QP_1} + 3\overrightarrow{QP_2}\cdot\overrightarrow{QP_0} + 6\overrightarrow{QP_1}^2 - 3\overrightarrow{QP_2}\cdot\overrightarrow{QP_1}) \newline
    /// \quad + t(3\overrightarrow{QP_0}^2 - 6\overrightarrow{QP_0}\cdot\overrightarrow{QP_1} + \overrightarrow{QP_2}\cdot\overrightarrow{QP_0} + 2\overrightarrow{QP_1}^2) \newline
    /// \quad - \overrightarrow{QP_0}^2 + \overrightarrow{QP_1}\cdot\overrightarrow{QP_0} \newline
    /// ]
    /// ```
    ///
    /// This polynomial can be solved analytically. Roots $`t`$ within the range $`[0;1]`$ as well as the
    /// boundaries $`t = 0`$, $`t = 1`$ are considered for the solution of the overall task.
    ///
    /// The final solution is the one that minimizes the distance from $`\mathbf{Q}`$.
    fn project_point(&self, query: Vec2<f32>) -> ProjectionResult {
        let q: Vec2<f32> = Vec2::from(self.from.to_array()) - query;
        let r: Vec2<f32> = Vec2::from(self.ctrl.to_array()) - query;
        let s: Vec2<f32> = Vec2::from(self.to.to_array()) - query;
        let q2 = q.dot(q);
        let r2 = r.dot(r);
        let s2 = s.dot(s);
        let qr = q.dot(r);
        let qs = q.dot(s);
        let rs = r.dot(s);

        #[rustfmt::skip]
        let roots = roots::find_roots_cubic(
                   q2 - 4.0 * qr + 2.0 * qs + 4.0 * r2 - 4.0 * rs + s2,
            -3.0 * q2 + 9.0 * qr - 3.0 * qs - 6.0 * r2 + 3.0 * rs,
             3.0 * q2 - 6.0 * qr +       qs + 2.0 * r2,
                  -q2 +       qr,
        );

        roots
            .as_ref()
            .iter()
            .filter(|&&t| t >= 0.0 && t <= 1.0)
            .chain([0.0, 1.0].iter())
            .copied()
            .map(|t| ProjectionResult {
                t,
                distance: Vec2::from(self.sample(t).to_array()).distance_squared(query),
            })
            .min_by(|a, b| std::cmp::PartialOrd::partial_cmp(&a.distance, &b.distance).unwrap())
            .unwrap()
    }
}

pub struct Segments<T: Segment> {
    pub segments: SmallVec<[T; 2]>,
}

impl<T: Segment> Segments<T> {
    pub fn new(segments: SmallVec<[T; 2]>) -> Self {
        assert!(segments.len() > 0, "Cannot create Segments without any segments.");
        Self { segments }
    }

    pub fn sample(&self, t: f32) -> Vec2<T::Scalar> {
        assert!(t >= 0.0 && t <= 1.0, "Parameter t out of bounds when sampling Segments.");

        if t == 1.0 {
            self.segments[self.segments.len() - 1].sample(T::Scalar::ONE).to_array().into()
        } else {
            let ts = t * self.segments.len() as f32;
            let segment_index = ts.floor() as usize;
            let segment = &self.segments[segment_index];

            segment.sample(T::Scalar::value(ts.fract())).to_array().into()
        }
    }
}

impl<T: Segment> Deref for Segments<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        &self.segments[..]
    }
}

impl<T: Segment> DerefMut for Segments<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.segments[..]
    }
}

impl<T: Segment + ConnectionSegment> Segments<T> {
    pub fn build_segments(&self, builder: &mut Builder) {
        for segment in &self.segments {
            segment.build_segment(builder);
        }
    }

    pub fn project_point(&self, query: Vec2<f32>) -> ProjectionResult {
        self.segments
            .iter()
            .enumerate()
            .map(|(index, segment)| {
                let mut projection = segment.project_point(query);
                projection.t = (projection.t + index as f32) / self.segments.len() as f32;
                projection
            })
            .min_by(|a, b| std::cmp::PartialOrd::partial_cmp(&a.distance, &b.distance).unwrap())
            .unwrap()
    }
}

pub fn get_connection_curve(from: Vec2<f32>, to: Vec2<f32>) -> Segments<QuadraticBezierSegment<f32>> {
    const CONTROL_POINT_DISTANCE_SLOPE: f32 = 1.0 / 3.0;
    const CONTROL_POINT_DISTANCE_ABS_SOFTNESS: f32 = 32.0;
    const CONTROL_POINT_DISTANCE_MAX_SHARPNESS: f32 = 0.01;
    const CONTROL_POINT_DISTANCE_MAX: f32 = 64.0;

    let mid = (from + to) / 2.0;
    let control_point_distance = (to - from)
        .map(|coord_delta| {
            softminabs(
                CONTROL_POINT_DISTANCE_ABS_SOFTNESS,
                CONTROL_POINT_DISTANCE_MAX_SHARPNESS,
                CONTROL_POINT_DISTANCE_MAX,
                coord_delta * CONTROL_POINT_DISTANCE_SLOPE,
            )
        })
        .sum();

    let control_from = from + Vec2::new(control_point_distance, 0.0);
    let control_to = to - Vec2::new(control_point_distance, 0.0);

    Segments {
        segments: smallvec![
            QuadraticBezierSegment {
                from: from.into_array().into(),
                ctrl: control_from.into_array().into(),
                to: mid.into_array().into(),
            },
            QuadraticBezierSegment {
                from: mid.into_array().into(),
                ctrl: control_to.into_array().into(),
                to: to.into_array().into(),
            }
        ],
    }
}

/// https://www.desmos.com/calculator/hmhxxjxnld
pub fn softmax(min: f32, sharpness: f32, x: f32) -> f32 {
    let min = min as f64;
    let sharpness = sharpness as f64;
    let x = x as f64;
    let result = ((1.0 + (sharpness * (x - min)).exp()).ln() / sharpness) + min;

    result as f32
}

/// Do not google images for this function (or do at your own risk)
/// https://www.desmos.com/calculator/miwhjandre
///
/// `softness` describes the radius around the origin in which the result is smooth
fn softabs(softness: f32, x: f32) -> f32 {
    let abs_x = x.abs();

    if abs_x < softness {
        ((x / softness).powi(2) + 1.0) * 0.5 * softness
    } else {
        abs_x
    }
}

/// Do not google images for this function (or do at your own risk)
/// https://www.desmos.com/calculator/miwhjandre
///
/// Variant of `softabs` where f(0) = 0
///
/// https://www.desmos.com/calculator/dxybnuifuw
pub fn softabs2(softness: f32, x: f32) -> f32 {
    let abs_x = x.abs();

    if abs_x < softness {
        (x / softness).powi(2) * 0.5 * softness
    } else {
        abs_x - 0.5 * softness
    }
}

/// A combination of softabs2 and softmax to limit the maximum value
/// https://www.desmos.com/calculator/1j5pkbmxd8
pub fn softminabs(abs_softness: f32, max_sharpness: f32, max: f32, x: f32) -> f32 {
    softmax(-max, max_sharpness, 0.0) - softmax(-max, max_sharpness, -softabs2(abs_softness, x))
}

pub fn draw_point(position: Vec2<f32>, color: Color, radius: f32) -> Primitive {
    let connection_point_center = radius + 1.0; // extra pixel for anti aliasing
    let frame_size = connection_point_center * 2.0;
    let mut frame = Frame::new([frame_size, frame_size].into());
    let path = Path::new(|builder| {
        builder.circle([connection_point_center, connection_point_center].into(), radius);
    });

    frame.fill(&path, Fill { color, rule: FillRule::NonZero });

    Primitive::Translate {
        translation: (position - Vec2::new(connection_point_center, connection_point_center))
            .into_array()
            .into(),
        content: Box::new(frame.into_geometry().into_primitive()),
    }
}

pub fn draw_bounds(layout: Layout<'_>, color: Color) -> Primitive {
    // let layout_position = Vector::new(layout.position().x, layout.position().y);
    // let layout_size = Vector::new(layout.bounds().size().width, layout.bounds().size().height);

    // Primitive::Group {
    //     primitives: vec![
    //         draw_point(
    //             layout_position,
    //             color,
    //         ),
    //         draw_point(
    //             layout_position + layout_size,
    //             color,
    //         ),
    //     ],
    // }
    Primitive::Quad {
        bounds: layout.bounds(),
        background: Background::Color(Color::TRANSPARENT),
        border_radius: 0,
        border_width: 1,
        border_color: color,
    }
}

pub fn partial_min<T: PartialOrd>(a: T, b: T) -> T {
    if a < b {
        a
    } else {
        b
    }
}

pub fn partial_max<T: PartialOrd>(a: T, b: T) -> T {
    if a < b {
        b
    } else {
        a
    }
}

pub trait RectangleExt: Sized {
    fn from_min_max(min: Vec2<f32>, max: Vec2<f32>) -> Self;
    fn grow(&self, right: f32, up: f32, left: f32, down: f32) -> Self;
    fn min(&self) -> Vec2<f32>;
    fn max(&self) -> Vec2<f32>;
    fn vertices(&self) -> [Vec2<f32>; 4];

    fn grow_symmetrical(&self, horizontally: f32, vertically: f32) -> Self {
        self.grow(horizontally, vertically, horizontally, vertically)
    }

    fn grow_uniform(&self, amount: f32) -> Self {
        self.grow(amount, amount, amount, amount)
    }
}

impl RectangleExt for Rectangle {
    fn from_min_max(min: Vec2<f32>, max: Vec2<f32>) -> Self {
        Self::new(min.into_array().into(), (max - min).into_array().into())
    }

    fn grow(&self, right: f32, up: f32, left: f32, down: f32) -> Self {
        Self {
            x: self.x - left,
            y: self.y - up,
            width: self.width + left + right,
            height: self.height + up + down,
        }
    }

    fn min(&self) -> Vec2<f32> {
        Vec2::new(self.x, self.y)
    }

    fn max(&self) -> Vec2<f32> {
        Vec2::new(self.x + self.width, self.y + self.height)
    }

    fn vertices(&self) -> [Vec2<f32>; 4] {
        let min = self.min();
        let max = self.max();

        [
            Vec2::new(min[0], max[1]),
            Vec2::new(max[0], max[1]),
            Vec2::new(max[0], min[1]),
            Vec2::new(min[0], min[1]),
        ]
    }
}

pub trait PathBuilderExt {
    fn line_segment_loop(&mut self, line_segments: &[Vec2<f32>]);
}

impl PathBuilderExt for iced_graphics::widget::canvas::path::Builder {
    fn line_segment_loop(&mut self, vertices: &[Vec2<f32>]) {
        if vertices.len() < 2 {
            return;
        }

        self.move_to(vertices.last().unwrap().into_array().into());

        for vertex in vertices {
            self.line_to(vertex.into_array().into());
        }
    }
}
