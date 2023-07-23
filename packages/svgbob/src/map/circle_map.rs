use crate::{
    buffer::{CellBuffer, Contacts, Span},
    fragment,
    fragment::{Arc, Circle},
    Cell, Point, Settings,
};
use indexmap::IndexMap;
use once_cell::sync::Lazy;
use std::{
    collections::{BTreeMap, HashMap},
    iter::FromIterator,
};

/// skip the first 3 circles for constructing our arcs, otherwise it will just be a mess
pub const CIRCLES_TO_SKIP_FOR_ARC: usize = 3;

// These are circle map, when a group is detected to have these set of characters
// arrange together in such this way, then endorse them as a circle
// Each of these character formation will have a certain circle parameters: center, and radius.
//

/// ```ignore
///      0 1 2 3 4           B C D
///     0┌─┬─┬─┬─┐        A┌─┬─┬─┬─┐E
///     1├─┼─┼─┼─┤         │ │ │ │ │
///     2├─┼─┼─┼─┤        F├─G─H─I─┤J
///     3├─┼─┼─┼─┤         │ │ │ │ │
///     4├─┼─┼─┼─┤        K├─L─M─N─┤O
///     5├─┼─┼─┼─┤         │ │ │ │ │
///     6├─┼─┼─┼─┤        P├─Q─R─S─┤T
///     7├─┼─┼─┼─┤         │ │ │ │ │
///     8└─┴─┴─┴─┘        U└─┴─┴─┴─┘Y
/// ```                      V W X
/// (
/// art - the ascii art of the circle, the empty space is automatically removed
/// edge_case - where the edge from the left most cell of the circle
/// f32 - how many cell from the left most to the center.x of the circle
/// f32 - how many cell from the top most to the center.y of the circle,
/// Cell - center cell in arc2
/// )
static CIRCLE_ART_MAP: Lazy<Vec<(&'static str, Horizontal, f32, f32, Cell)>> =
    Lazy::new(|| {
        vec![
            // CIRCLE_0
            //center 1,0,k, radius = 0.5
            // 2 cell width , radius formula: (n-1)/2 = (2-1)/2 = 0.5
            // where n is the number of cells used
            //  if edge_case starts at edge then n is added by 1.
            //  vert_mid: half (0.5/1.0)
            //  cx_lies: edge
            //  cy_lies: mid
            //
            //  edge_case: start_half = 0.5, start_edge = 0.0
            //  if radius +  edge_case has 0.5 then mid, 0.0 then edge
            //
            //
            (
                r#"
            ()
            "#,
                Horizontal::Half,
                1.0,
                0.5,
                Cell::new(0, 0),
            ),
            // CIRCLE_1
            //center = 1,1,m radius = 1.0
            // 3 cell width, (n-1)/2 = (3-1)/2  = 1.0
            // vert_mid: half  (0.5/1.0)
            // cx_lies: mid
            // cy_lies: mid
            (
                r#"
            (_)
            "#,
                Horizontal::Half,
                1.5,
                0.5,
                Cell::new(1, 0),
            ),
            // CIRCLE_2
            //center = 1,1,o radius = 1.5,
            // 4 cell width, (n-1)/2 = (4-1)/2 = 1.5
            // vert_mid: 3/4 (1.5/2.0)
            // cx_lies: edge
            // cy_lies: mid
            (
                r#"
             __
            (__)
            "#,
                Horizontal::Half,
                2.0,
                1.5,
                Cell::new(1, 1),
            ),
            // CIRCLE_3
            //center: 2,1,m radius: 2.0
            //  5 cell width, (n-1)/2 = (5-1)/2 = 2.0
            //  vert_mid: half (1.5/3.0)
            // cx_lies: mid
            // cy_lies: mid
            //
            // shared x,  if starts at half and offset_center_x * 2.0 is odd
            // shared y
            (
                r#"
             ,-.
            (   )
             `-'
            "#,
                Horizontal::Half,
                2.5,
                1.5,
                Cell::new(2, 1),
            ),
            // CIRCLE_4
            //center: 2,1,o radius: 2.5
            // 6 cell width, (n-1)/2 = (6-1)/2 = 2.5
            // vert_mid: half (1.5/3.0)
            //  cx_lies: edge
            //  cy_lies: mid
            //
            //   no shared x, offset_center_x * 2.0 is even
            //   shared y
            (
                r#"
             .--.
            (    )
             `--'
            "#,
                Horizontal::Half,
                3.0,
                1.5,
                Cell::new(2, 1),
            ),
            // CIRCLE_5
            //center: 3,2,m radius: 3.0
            // 7 cell width, (n-1)/2 = (7-1)/2 = 3.0
            // vert_mid: 2.5/4
            // cx_lies: mid
            // cy_lies: mid
            //
            // shared x
            // shared y
            (
                r#"
               _
             .' '.
            (     )
             `._.'
            "#,
                Horizontal::Half,
                3.5,
                2.5,
                Cell::new(3, 2),
            ),
            // CIRCLE_6
            //center: 3,2,o radius: 3.5
            // 8 cell width, (n-1)/2 = (8-1)/2 = 3.5
            // vert_mid: 2.5/4
            //  cx_lies: edge
            //  cy_lies: mid
            //
            //  no shared x
            //  shared y
            (
                r#"
               __
             ,'  '.
            (      )
             `.__.'
            "#,
                Horizontal::Half,
                4.0,
                2.5,
                Cell::new(3, 2),
            ),
            // CIRCLE_7
            //center: 4,2,m radius:4.0
            // 9 cell width, (n-1)/2 = (9-1)/2 = 4.0
            // vert_mid: half (2.5/5.0)
            // cx_lies: mid
            // cy_lies: mid
            //
            // shared x
            // shared y
            (
                r#"
               ___
             ,'   '.
            (       )
             `.   .'
               `-'
            "#,
                Horizontal::Half,
                4.5,
                2.5,
                Cell::new(4, 2),
            ),
            // circle 8 and up can be divided into 4 quadrants these quadrants can be arcs and can be used as
            // rounded edge with larger radius for rouded rect
            // CIRCLE_8
            //center: 4,2,w radius: 4.5
            // start_edge: true
            // 9 cell width, (n-0)/2 = (9-0)/2 = 4.5
            // vert_mid:  3.0/5.0
            //  cx_lies: mid
            //  cy_lies: edge
            //
            //  shared x
            //  no shared y
            (
                r#"
               ___
             ,'   `.
            /       \
            \       /
             `.___.'
            "#,
                Horizontal::LeftEdge,
                4.5,
                3.0,
                Cell::new(4, 2),
            ),
            // CIRCLE_9
            //center: 4,2,y radius: 5.0
            //start_edge: true
            // 10 cell width, (n-0)/2 = (10-0)/2 = 5.0
            // vert_mid:  3.0/5.0
            //  cx_lies: edge
            //  cy_lies: edge
            //
            //  no shared x, if offset_center_x * 2.0 is even
            //  no shared y, if the offset_center_y * 2.0 is even
            (
                r#"
               ____
             ,'    `.
            /        \
            \        /
             `.____.'
            "#,
                Horizontal::LeftEdge,
                5.0,
                3.0,
                Cell::new(4, 2),
            ),
            // CIRCLE_10
            //center:5,3,o radius: 5.5
            // 12 cell width, (n-1)/2 = (12-1)/2 = 5.5
            // vert_mid:  3.5/6.0
            //  cx_lies: edge
            //  cy_lies: mid
            //
            //  no shared x
            //  shared y
            (
                r#"
                ____
              .'    `.
             /        \
            (          )
             \        /
              `.____.'
            "#,
                Horizontal::Half,
                6.0,
                3.5,
                Cell::new(5, 3),
            ),
            // CIRCLE_11
            //center:6,3,m radius: 6.0
            // 13 cell width, (n-1)/2 = (13-1)/2 = 6.0
            // vert_mid: 3.5/6.0
            // cx_lies: mid
            // cy_lies: mid
            //
            // shared x
            // shared y
            (
                r#"
                _____
              ,'     `.
             /         \
            (           )
             \         /
              `._____.'
            "#,
                Horizontal::Half,
                6.5,
                3.5,
                Cell::new(6, 3),
            ),
            // CIRCLE_12
            // center: 6,3,y radius: 6.5
            // vert_mid: 4.0/7.0
            //  cx_lies: edge
            //  cy_lies: edge
            //
            //  no shared x
            //  no shared y
            (
                r#"
                ______
              ,'      `.
             /          \
            |            |
            |            |
             \          /
              `.______.'
            "#,
                Horizontal::Half,
                7.0,
                4.0,
                Cell::new(6, 3),
            ),
            // CIRCLE_13
            //center: 7,3,w radius: 7.0
            //vert_mid: 4.0/7.0
            //  cx_lies: mid
            //  cy_lies: edge
            //
            //  shared x
            //  no shared y
            (
                r#"
                _______
              ,'       `.
             /           \
            |             |
            |             |
             \           /
              `._______.'
            "#,
                Horizontal::Half,
                7.5,
                4.0,
                Cell::new(7, 3),
            ),
            // CIRCLE_14
            //center: 7,4,o radius: 7.5
            //vert_mid: 4.5/8.0
            //  cx_lies: edge
            //  cy_lies: mid
            //
            //  no shared x
            //  shared y
            (
                r#"
                ________
              ,'        `.
             /            \
            |              |
            |              |
            |              |
             \            /
              `.________.'
            "#,
                Horizontal::Half,
                8.0,
                4.5,
                Cell::new(7, 4),
            ),
            // CIRCLE_15
            //center: 8,4,m radius: 8.0
            //vert_mid: 4.5/9.0
            // cx_lies: mid
            // cy_lies: mid
            //
            // shared x
            // shared y
            (
                r#"
                __-----__
              ,'         `.
             /             \
            |               |
            |               |
            |               |
             \             /
              `.         .'
                `-------'
            "#,
                Horizontal::Half,
                8.5,
                4.5,
                Cell::new(8, 4),
            ),
            // CIRCLE_16
            //center: 8,4,o radius: 8.5
            // vert_mid:  4.5/9.0
            //  cx_lies: edge
            //  cy_lies: mid
            //
            //  no shared x
            //  shared y
            (
                r#"
                .--------.
              ,'          `.
             /              \
            |                |
            |                |
            |                |
             \              /
              `.          .'
                `--------'
            "#,
                Horizontal::Half,
                9.0,
                4.5,
                Cell::new(8, 4),
            ),
            // CIRCLE_17
            //center:9,5,m radius: 9.0
            //vert_mid: 5.5/10.0
            // cx_lies: mid
            // cy_lies: mid
            //
            //  shared x
            //  shared y
            (
                r#"
                _.-'''''-._
              ,'           `.
             /               \
            .                 .
            |                 |
            |                 |
            |                 |
             \               /
              `._         _.'
                 '-.....-'
            "#,
                Horizontal::Half,
                9.5,
                5.5,
                Cell::new(9, 5),
            ),
            // CIRCLE_18
            // center: 9,5,o radius: 9.5
            // vert_mid:  5.5/10.0
            //  cx_lies: edge
            //  cy_lies: mid
            //
            // no shared x
            // shared y
            //
            (
                r#"
                _.-''''''-._
              ,'            `.
             /                \
            .                  .
            |                  |
            |                  |
            |                  |
             \                /
              `._          _.'
                 '-......-'
            "#,
                Horizontal::Half,
                10.0,
                5.5,
                Cell::new(9, 5),
            ),
            // CIRCLE_19
            // center: 10,5,m radius: 10
            // vert_mid: 5.5/10.0
            // cx_lies: mid
            // cy_lies: mid
            //
            // shared x
            // shared y
            (
                r#"
                _.-'''''''-._
              ,'             `.
             /                 \
            .                   .
            |                   |
            |                   |
            |                   |
             \                 /
              `._           _.'
                 '-.......-'
            "#,
                Horizontal::Half,
                10.5,
                5.5,
                Cell::new(10, 5),
            ),
            // CIRCLE_20
            // center: 10,5,o radius: 10.5
            // vert_mid: 5.5/11.0
            //  cx_lies: edge
            //  cy_lies: mid
            //
            // no shared x
            // shared y
            (
                r#"
                _.-''''''''-._
              ,'              `.
             /                  \
            .                    .
            |                    |
            |                    |
            |                    |
            |                    |
             \                  /
              `._            _.'
                 '-........-'
            "#,
                Horizontal::Half,
                11.0,
                5.5,
                Cell::new(10, 5),
            ),
            // CIRCLE_21
            // center: 10,5,m radius: 11
            // radius = (n-1)/2 = (23-1)/2 = 11
            // vert_mid: 5.5/11.0
            // cx_lies: mid
            // cy_lies: mid
            //
            // shared x
            // shared y
            (
                r#"
                _.-'''''''''-._
              ,'               `.
             /                   \
            .                     .
            |                     |
            |                     |
            |                     |
            |                     |
             \                   /
              `._             _.'
                 '-.........-'
            "#,
                Horizontal::Half,
                11.5,
                5.5,
                Cell::new(11, 5),
            ),
        ]
    });

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
/// edge cases of the circle art
pub enum Horizontal {
    /// circle arc is touching the left edge of the first cell
    /// ie: if the left most cell is `/` then it is touching the dge
    LeftEdge,
    /// if the left most cell is `(` or `|` then it starts at half the cell
    Half,
}

pub struct CircleArt {
    /// the ascii art of the circel
    /// empty lines are ignored
    /// empty vertical columns are ignored
    ascii_art: &'static str,
    start_edge: Horizontal,
    /// distance in cell units, from the left edge of the ascii art to the center.x of the circle
    offset_center_x: f32,
    /// distance in cell units, from the top edge of the ascii art to the center.y of the circle
    offset_center_y: f32,
}

impl CircleArt {
    /// calculate the centel cell of this circle art
    /// based on offset center
    fn center_cell(&self) -> Cell {
        let mut center_cell_x = self.offset_center_x - self.edge_increment_x();

        // if no shared x (meaning even number of ascii art along x axis)
        // we want to use the cell before it as the center cell
        if !self.is_shared_x() {
            center_cell_x -= 0.5;
        }
        let mut center_cell_y = self.offset_center_y;
        if !self.is_shared_y() {
            center_cell_y -= 0.5;
        }
        Cell::new(center_cell_x.floor() as i32, center_cell_y.floor() as i32)
    }

    /// returns the width in cells of the ascii art of this circle
    fn width(&self) -> f32 {
        let cb = CellBuffer::from(self.ascii_art);
        let (lo, hi) = cb.bounds().expect("circle must have bounds");
        match self.start_edge {
            Horizontal::LeftEdge => (hi.x - lo.x) as f32 + 1.0,
            Horizontal::Half => (hi.x - lo.x) as f32,
        }
    }

    fn center(&self) -> Point {
        let center_x = self.radius() + self.edge_increment_x();
        let center_y = self.offset_center_y * 2.0;
        Point::new(center_x, center_y)
    }

    fn edge_increment_x(&self) -> f32 {
        match self.start_edge {
            Horizontal::LeftEdge => 0.0,
            Horizontal::Half => 0.5,
        }
    }

    fn radius(&self) -> f32 {
        self.width() / 2.0
    }

    fn diameter(&self) -> i32 {
        (self.radius() * 2.0).floor() as i32
    }

    /// center cell at x will be shared if it is on the odd count
    fn is_shared_x(&self) -> bool {
        self.offset_center_x * 2.0 % 2.0 == 1.0
    }

    /// center cell at y will be shared if it is on the odd count
    fn is_shared_y(&self) -> bool {
        self.offset_center_y * 2.0 % 2.0 == 1.0
    }
}

pub struct ArcSpans {
    diameter: i32,
    arc_spans: Vec<(Arc, Span)>,
}

static CIRCLE_MAP: Lazy<Vec<CircleArt>> = Lazy::new(|| {
    Vec::from_iter(CIRCLE_ART_MAP.iter().enumerate().map(
        |(
            ndx,
            (art, edge_case, offset_center_x, offset_center_y, arc2_center),
        )| {
            CircleArt {
                ascii_art: *art,
                start_edge: *edge_case,
                offset_center_x: *offset_center_x,
                offset_center_y: *offset_center_y,
            }
        },
    ))
});

/// The fragments for each of the circle
/// Calculate the span and get the group fragments
static FRAGMENTS_CIRCLE: Lazy<Vec<(Vec<Contacts>, Circle)>> = Lazy::new(|| {
    Vec::from_iter(CIRCLE_MAP.iter().map(|circle_art| {
        (
            circle_art_to_group(circle_art.ascii_art),
            Circle::new(circle_art.center(), circle_art.radius(), false),
        )
    }))
});

/// map of circle spans and their radius
pub static DIAMETER_CIRCLE: Lazy<HashMap<i32, (Point, Span)>> =
    Lazy::new(|| {
        HashMap::from_iter(CIRCLE_MAP.iter().map(|circle_art| {
            let cb = CellBuffer::from(circle_art.ascii_art);
            let mut spans: Vec<Span> = cb.into();
            assert_eq!(spans.len(), 1);
            let span = spans.remove(0).localize();
            (circle_art.diameter(), (circle_art.center(), span))
        }))
    });

/// There is only 1 span per circle, and localized
pub static CIRCLES_SPAN: Lazy<IndexMap<Circle, Span>> = Lazy::new(|| {
    IndexMap::from_iter(CIRCLE_MAP.iter().map(|circle_art| {
        let cb = CellBuffer::from(circle_art.ascii_art);
        let mut spans: Vec<Span> = cb.into();
        assert_eq!(spans.len(), 1);
        let span = spans.remove(0).localize();
        (
            Circle::new(circle_art.center(), circle_art.radius(), false),
            span,
        )
    }))
});

/// top_left      top       top_right
///               p2
///          arc2  |    arc1
///                |
///    left  p3----+----- p1 right
///                |
///          arc3  |   arc4
///               p4
/// bottom_left  bottom   bottom_right
///
/// (diameter, quarter arcs)
pub static QUARTER_ARC_SPAN: Lazy<BTreeMap<i32, ArcSpans>> = Lazy::new(|| {
    BTreeMap::from_iter(CIRCLE_MAP.iter().skip(CIRCLES_TO_SKIP_FOR_ARC).map(
        |circle_art| {
            let span = circle_art_to_span(circle_art.ascii_art);
            let bounds = span.cell_bounds().expect("must have bounds");
            let top_left = bounds.top_left();
            let bottom_right = bounds.bottom_right();
            let top_right = bounds.top_right();
            let bottom_left = bounds.bottom_left();

            let center = circle_art.center();
            let radius = circle_art.radius();

            let p1 = Point::new(center.x + radius, center.y);
            let p2 = Point::new(center.x, center.y - radius);
            let p3 = Point::new(center.x - radius, center.y);
            let p4 = Point::new(center.x, center.y + radius);

            let arc2_center = circle_art.center_cell();

            let span1_center = Cell::new(
                (center.x.floor() / Cell::width()) as i32,
                arc2_center.y,
            );
            let span2_center = arc2_center;
            let span3_center = Cell::new(
                arc2_center.x,
                (center.y.floor() / Cell::height()) as i32,
            );
            let span4_center = Cell::new(
                (center.x.floor() / Cell::width()) as i32,
                (center.y.floor() / Cell::height()) as i32,
            );

            let bounds1 = Cell::rearrange_bound(span1_center, top_right);
            let bounds2 = Cell::rearrange_bound(top_left, span2_center);
            let bounds3 = Cell::rearrange_bound(bottom_left, span3_center);
            let bounds4 = Cell::rearrange_bound(span4_center, bottom_right);

            let span1 = span.extract(bounds1.0, bounds1.1).localize();
            let span2 = span.extract(bounds2.0, bounds2.1).localize();
            let span3 = span.extract(bounds3.0, bounds3.1).localize();
            let span4 = span.extract(bounds4.0, bounds4.1).localize();

            let arc1_start = bounds1.0.localize_point(p1);
            let arc1_end = bounds1.0.localize_point(p2);

            let arc2_start = bounds2.0.localize_point(p2);
            let arc2_end = bounds2.0.localize_point(p3);

            let arc3_start = bounds3.0.localize_point(p3);
            let arc3_end = bounds3.0.localize_point(p4);

            let arc4_start = bounds4.0.localize_point(p4);
            let arc4_end = bounds4.0.localize_point(p1);

            let arc1 = Arc::new(arc1_start, arc1_end, radius);
            let arc2 = Arc::new(arc2_start, arc2_end, radius);
            let arc3 = Arc::new(arc3_start, arc3_end, radius);
            let arc4 = Arc::new(arc4_start, arc4_end, radius);

            let diameter = circle_art.diameter();
            (
                diameter,
                ArcSpans {
                    diameter,
                    arc_spans: vec![
                        (arc1, span1),
                        (arc2, span2),
                        (arc3, span3),
                        (arc4, span4),
                    ],
                },
            )
        },
    ))
});

pub static HALF_ARC_SPAN: Lazy<BTreeMap<i32, ArcSpans>> = Lazy::new(|| {
    BTreeMap::from_iter(CIRCLE_MAP.iter().skip(CIRCLES_TO_SKIP_FOR_ARC).map(
        |circle_art| {
            let span = circle_art_to_span(circle_art.ascii_art);
            let bounds = span.cell_bounds().expect("must have bounds");

            let top_left = bounds.top_left();
            let bottom_right = bounds.bottom_right();
            let top_right = bounds.top_right();
            let bottom_left = bounds.bottom_left();

            assert_eq!(top_left.y, top_right.y);
            assert_eq!(top_left.x, bottom_left.x);
            assert_eq!(top_right.x, bottom_right.x);
            assert_eq!(bottom_left.y, bottom_right.y);

            let center_cell = circle_art.center_cell();

            let center = circle_art.center();
            let radius = circle_art.radius();

            let p1 = Point::new(center.x + radius, center.y);
            let p2 = Point::new(center.x, center.y - radius);
            let p3 = Point::new(center.x - radius, center.y);
            let p4 = Point::new(center.x, center.y + radius);

            let arc2_center = center_cell;

            let span1_center = Cell::new(
                (center.x.floor() / Cell::width()) as i32,
                arc2_center.y,
            );
            let span2_center = arc2_center;
            let span3_center = Cell::new(
                arc2_center.x,
                (center.y.floor() / Cell::height()) as i32,
            );
            let span4_center = Cell::new(
                (center.x.floor() / Cell::width()) as i32,
                (center.y.floor() / Cell::height()) as i32,
            );

            let top_tangent = Cell::new(top_right.x, span1_center.y);
            let bottom_tangent = Cell::new(bottom_left.x, span3_center.y);
            let left_tangent = Cell::new(span2_center.x, top_left.y);
            let right_tangent = Cell::new(span1_center.x, top_right.y);

            let bounds_top_half = Cell::rearrange_bound(top_left, top_tangent);
            let bounds_bottom_half =
                Cell::rearrange_bound(bottom_tangent, bottom_right);
            let bounds_left_half =
                Cell::rearrange_bound(left_tangent, bottom_left);
            let bounds_right_half =
                Cell::rearrange_bound(right_tangent, bottom_right);

            let span_top_half = span
                .extract(bounds_top_half.0, bounds_top_half.1)
                .localize();
            let span_bottom_half = span
                .extract(bounds_bottom_half.0, bounds_bottom_half.1)
                .localize();
            let span_left_half = span
                .extract(bounds_left_half.0, bounds_left_half.1)
                .localize();
            let span_right_half = span
                .extract(bounds_right_half.0, bounds_right_half.1)
                .localize();

            let bottom_half_start = bounds_bottom_half.0.localize_point(p3);
            let bottom_half_end = bounds_bottom_half.0.localize_point(p1);

            let right_half_start = bounds_right_half.0.localize_point(p4);
            let right_half_end = bounds_right_half.0.localize_point(p2);

            let arc_top_half = Arc::new(p1, p3, radius);
            let arc_bottom_half =
                Arc::new(bottom_half_start, bottom_half_end, radius);
            let arc_left_half = Arc::new(p2, p4, radius);
            let arc_right_half =
                Arc::new(right_half_start, right_half_end, radius);

            let diameter = circle_art.diameter();
            (
                diameter,
                ArcSpans {
                    diameter,
                    arc_spans: vec![
                        (arc_top_half, span_top_half),
                        (arc_bottom_half, span_bottom_half),
                        (arc_left_half, span_left_half),
                        (arc_right_half, span_right_half),
                    ],
                },
            )
        },
    ))
});

pub static THREE_QUARTERS_ARC_SPAN: Lazy<BTreeMap<i32, ArcSpans>> =
    Lazy::new(|| {
        BTreeMap::from_iter(
            CIRCLE_MAP
                .iter()
                .skip(CIRCLES_TO_SKIP_FOR_ARC)
                .map(|circle_art| {
                    let span = circle_art_to_span(circle_art.ascii_art);
                    let bounds = span.cell_bounds().expect("must have bounds");
                    let top_left = bounds.top_left();
                    let bottom_right = bounds.bottom_right();
                    let top_right = bounds.top_right();
                    let bottom_left = bounds.bottom_left();

                    let center = circle_art.center();
                    let radius = circle_art.radius();

                    let p1 = Point::new(center.x + radius, center.y);
                    let p2 = Point::new(center.x, center.y - radius);
                    let p3 = Point::new(center.x - radius, center.y);
                    let p4 = Point::new(center.x, center.y + radius);

                    let arc2_center = circle_art.center_cell();

                    let span1_center = Cell::new(
                        (center.x.floor() / Cell::width()) as i32,
                        arc2_center.y,
                    );
                    let span2_center = arc2_center;
                    let span3_center = Cell::new(
                        arc2_center.x,
                        (center.y.floor() / Cell::height()) as i32,
                    );
                    let span4_center = Cell::new(
                        (center.x.floor() / Cell::width()) as i32,
                        (center.y.floor() / Cell::height()) as i32,
                    );

                    let top_tangent = Cell::new(top_right.x, span1_center.y);
                    let bottom_tangent =
                        Cell::new(bottom_left.x, span3_center.y);
                    let left_tangent = Cell::new(span2_center.x, top_left.y);
                    let right_tangent = Cell::new(span1_center.x, top_right.y);

                    let bounds1 =
                        Cell::rearrange_bound(span1_center, top_right);
                    let bounds2 = Cell::rearrange_bound(top_left, span2_center);
                    let bounds3 =
                        Cell::rearrange_bound(bottom_left, span3_center);
                    let bounds4 =
                        Cell::rearrange_bound(span4_center, bottom_right);

                    let span1 = span.extract(bounds1.0, bounds1.1);
                    let span2 = span.extract(bounds2.0, bounds2.1);
                    let span3 = span.extract(bounds3.0, bounds3.1);
                    let span4 = span.extract(bounds4.0, bounds4.1);

                    let span_123 = span1
                        .merge_no_check(&span2)
                        .merge_no_check(&span3)
                        .localize();
                    let span_234 = span2
                        .merge_no_check(&span3)
                        .merge_no_check(&span4)
                        .localize();
                    let span_341 = span3
                        .merge_no_check(&span4)
                        .merge_no_check(&span1)
                        .localize();
                    let span_412 = span4
                        .merge_no_check(&span1)
                        .merge_no_check(&span2)
                        .localize();

                    let arc_123 = Arc::major(p1, p4, radius);
                    let arc_234 = Arc::major(p2, p1, radius);
                    let arc_341 = Arc::major(p3, p2, radius);
                    let arc_412 = Arc::major(p4, p3, radius);

                    let diameter = circle_art.diameter();
                    (
                        diameter,
                        ArcSpans {
                            diameter,
                            arc_spans: vec![
                                (arc_123, span_123),
                                (arc_234, span_234),
                                (arc_341, span_341),
                                (arc_412, span_412),
                            ],
                        },
                    )
                }),
        )
    });

pub static FLATTENED_QUARTER_ARC_SPAN: Lazy<
    BTreeMap<DiameterArc, (Arc, Span)>,
> = Lazy::new(|| {
    BTreeMap::from_iter(QUARTER_ARC_SPAN.iter().flat_map(
        |(diameter, arc_spans)| {
            arc_spans.arc_spans.iter().enumerate().map(
                move |(arc_index, arc_span)| {
                    (
                        DiameterArc {
                            diameter: *diameter,
                            arc: arc_index,
                        },
                        arc_span.clone(),
                    )
                },
            )
        },
    ))
});

pub static FLATTENED_HALF_ARC_SPAN: Lazy<BTreeMap<DiameterArc, (Arc, Span)>> =
    Lazy::new(|| {
        BTreeMap::from_iter(HALF_ARC_SPAN.iter().flat_map(
            |(diameter, arc_spans)| {
                arc_spans.arc_spans.iter().enumerate().map(
                    move |(arc_index, arc_span)| {
                        (
                            DiameterArc {
                                diameter: *diameter,
                                arc: arc_index,
                            },
                            arc_span.clone(),
                        )
                    },
                )
            },
        ))
    });

pub static FLATTENED_THREE_QUARTERS_ARC_SPAN: Lazy<
    BTreeMap<DiameterArc, (Arc, Span)>,
> = Lazy::new(|| {
    BTreeMap::from_iter(THREE_QUARTERS_ARC_SPAN.iter().flat_map(
        |(diameter, arc_spans)| {
            arc_spans.arc_spans.iter().enumerate().map(
                move |(arc_index, arc_span)| {
                    (
                        DiameterArc {
                            diameter: *diameter,
                            arc: arc_index,
                        },
                        arc_span.clone(),
                    )
                },
            )
        },
    ))
});

#[derive(Default, Hash, PartialEq, Eq)]
pub struct DiameterArc {
    /// the arc diameter
    diameter: i32,
    /// arc number
    arc: usize,
}
impl Ord for DiameterArc {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.diameter
            .cmp(&other.diameter)
            .then(self.arc.cmp(&other.arc))
    }
}

impl PartialOrd for DiameterArc {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

fn circle_art_to_group(art: &str) -> Vec<Contacts> {
    circle_art_to_span(art).into()
}

fn circle_art_to_span(art: &str) -> Span {
    let cell_buffer = CellBuffer::from(art);
    let mut spans: Vec<Span> = cell_buffer.into();
    assert_eq!(spans.len(), 1);
    spans.remove(0).localize()
}

pub fn endorse_circle_span(search: &Span) -> Option<(&Circle, Span)> {
    CIRCLES_SPAN.iter().rev().find_map(|(circle, span)| {
        let search_localized = search.clone().localize();
        let (matched, unmatched) = is_subset_of(span, &search_localized);
        if matched {
            let unmatched_cell_chars = search
                .iter()
                .enumerate()
                .filter_map(|(i, cell_char)| {
                    if unmatched.contains(&i) {
                        Some(*cell_char)
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            Some((circle, Span::from(unmatched_cell_chars)))
        } else {
            None
        }
    })
}

pub fn endorse_quarter_arc_span(search: &Span) -> Option<(&Arc, Span)> {
    FLATTENED_QUARTER_ARC_SPAN.iter().rev().find_map(
        |(_diameter, (arc, span))| {
            let search_localized = search.clone().localize();
            let (matched, unmatched) = is_subset_of(span, &search_localized);
            if matched {
                let unmatched_cell_chars = search
                    .iter()
                    .enumerate()
                    .filter_map(|(i, cell_char)| {
                        if unmatched.contains(&i) {
                            Some(*cell_char)
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();
                Some((arc, Span::from(unmatched_cell_chars)))
            } else {
                None
            }
        },
    )
}

pub fn endorse_half_arc_span(search: &Span) -> Option<(&Arc, Span)> {
    FLATTENED_HALF_ARC_SPAN
        .iter()
        .rev()
        .find_map(|(_diameter, (arc, span))| {
            let search_localized = search.clone().localize();
            let (matched, unmatched) = is_subset_of(span, &search_localized);
            if matched {
                let unmatched_cell_chars = search
                    .iter()
                    .enumerate()
                    .filter_map(|(i, cell_char)| {
                        if unmatched.contains(&i) {
                            Some(*cell_char)
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();
                Some((arc, Span::from(unmatched_cell_chars)))
            } else {
                None
            }
        })
}

pub fn endorse_three_quarters_arc_span(search: &Span) -> Option<(&Arc, Span)> {
    FLATTENED_THREE_QUARTERS_ARC_SPAN.iter().rev().find_map(
        |(_diameter, (arc, span))| {
            let search_localized = search.clone().localize();
            let (matched, unmatched) = is_subset_of(span, &search_localized);
            if matched {
                let unmatched_cell_chars = search
                    .iter()
                    .enumerate()
                    .filter_map(|(i, cell_char)| {
                        if unmatched.contains(&i) {
                            Some(*cell_char)
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();
                Some((arc, Span::from(unmatched_cell_chars)))
            } else {
                None
            }
        },
    )
}

/// returns true if all the contacts in subset is in big_set
/// This also returns the indices of big_set that are not found in the subset
fn is_subset_of<T: PartialEq>(
    subset: &[T],
    big_set: &[T],
) -> (bool, Vec<usize>) {
    let mut unmatched = vec![];
    let mut matched = 0;
    for (_i, set) in subset.iter().enumerate() {
        if big_set.contains(set) {
            matched += 1;
        }
    }
    for (bi, bset) in big_set.iter().enumerate() {
        if !subset.contains(bset) {
            unmatched.push(bi);
        }
    }
    (matched == subset.len(), unmatched)
}

#[cfg(test)]
mod test_circle_map;
