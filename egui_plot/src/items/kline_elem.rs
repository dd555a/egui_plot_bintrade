use egui::emath::NumExt as _;
use egui::epaint::{Color32, CornerRadius, RectShape, Shape, Stroke};

use crate::{KlinePlot, Cursor, PlotPoint, PlotTransform};

use super::{Orientation, PlotConfig, RectElement, add_rulers_and_text, highlighted_color};

/// Contains the values of a single box in a box plot.
#[derive(Clone, Debug, PartialEq)]
pub struct KlineData{
    /// Value of lower whisker (typically minimum).
    ///
    /// The whisker is not drawn if `lower_whisker >= o`.
    pub o: f64,

    /// Value of lower box threshold (typically 25% quartile)
    pub h: f64,

    /// Value of middle line in box (typically v)
    pub l: f64,

    /// Value of upper box threshold (typically 75% quartile)
    pub c: f64,

    /// Value of upper whisker (typically maximum)
    ///
    /// The whisker is not drawn if `h <= c`.
    pub v: f64,
}

impl KlineData{
    pub fn new(
        o: f64,
        h: f64,
        l: f64,
        c: f64,
        v: f64,
    ) -> Self {
        Self {
            l,
            o,
            v,
            c,
            h,
        }
    }
}

/// A box in a [`BoxPlot`] diagram.
///
/// This is a low-level graphical element; it will not compute quartiles and whiskers, letting one
/// use their preferred formula. Use [`Points`][`super::Points`] to draw the outliers.
#[derive(Clone, Debug, PartialEq)]
pub struct KlinePlotPoint{
    /// Name of plot element in the diagram (annotated by default formatter).
    pub name: String,

    /// Which direction the box faces in the diagram.
    pub orientation: Orientation,

    /// Position on the argument (input) axis -- X if vertical, Y if horizontal.
    pub argument: f64,

    /// Values of the box
    pub spread: KlineData,

    /// Thickness of the box
    pub box_width: f64,

    /// Width of the whisker at minimum/maximum
    pub whisker_width: f64,

    /// Line width and color
    pub stroke: Stroke,

    /// Fill color
    pub fill: Color32,
}

impl KlinePlotPoint{
    /// Create a box element. Its `orientation` is set by its [`BoxPlot`] parent.
    ///
    /// Check [`KlinePlotPoint`] fields for detailed description.
    pub fn new(argument: f64, spread: KlineData) -> Self {
        Self {
            argument,
            orientation: Orientation::default(),
            name: String::default(),
            spread,
            box_width: 0.25,
            whisker_width: 0.15,
            stroke: Stroke::new(1.0, Color32::TRANSPARENT),
            fill: Color32::TRANSPARENT,
        }
    }

    /// Name of this box element.
    #[allow(clippy::needless_pass_by_value)]
    #[inline]
    pub fn name(mut self, name: impl ToString) -> Self {
        self.name = name.to_string();
        self
    }

    /// Add a custom stroke.
    #[inline]
    pub fn stroke(mut self, stroke: impl Into<Stroke>) -> Self {
        self.stroke = stroke.into();
        self
    }

    /// Add a custom fill color.
    #[inline]
    pub fn fill(mut self, color: impl Into<Color32>) -> Self {
        self.fill = color.into();
        self
    }

    /// Set the box width.
    #[inline]
    pub fn box_width(mut self, width: f64) -> Self {
        self.box_width = width;
        self
    }

    /// Set the whisker width.
    #[inline]
    pub fn whisker_width(mut self, width: f64) -> Self {
        self.whisker_width = width;
        self
    }

    /// Set orientation of the element as vertical. Argument axis is X.
    #[inline]
    pub fn vertical(mut self) -> Self {
        self.orientation = Orientation::Vertical;
        self
    }

    /// Set orientation of the element as horizontal. Argument axis is Y.
    #[inline]
    pub fn horizontal(mut self) -> Self {
        self.orientation = Orientation::Horizontal;
        self
    }

    pub(super) fn add_shapes(
        &self,
        transform: &PlotTransform,
        highlighted: bool,
        shapes: &mut Vec<Shape>,
    ) {
        let (stroke, fill) = if highlighted {
            highlighted_color(self.stroke, self.fill)
        } else {
            (self.stroke, self.fill)
        };

        let rect = transform.rect_from_values(
            &self.point_at(self.argument - self.box_width / 2.0, self.spread.o),
            &self.point_at(self.argument + self.box_width / 2.0, self.spread.c),
        );
        let rect = Shape::Rect(RectShape::new(
            rect,
            CornerRadius::ZERO,
            fill,
            stroke,
            egui::StrokeKind::Inside,
        ));
        shapes.push(rect);

        let line_between = |v1, v2| {
            Shape::line_segment(
                [
                    transform.position_from_point(&v1),
                    transform.position_from_point(&v2),
                ],
                stroke,
            )
        };
        let v = line_between(
            self.point_at(self.argument - self.box_width / 2.0, self.spread.v),
            self.point_at(self.argument + self.box_width / 2.0, self.spread.v),
        );
        shapes.push(v);

        if self.spread.h > self.spread.c {
            let high_whisker = line_between(
                self.point_at(self.argument, self.spread.c),
                self.point_at(self.argument, self.spread.h),
            );
            shapes.push(high_whisker);
            if self.box_width > 0.0 {
                let high_whisker_end = line_between(
                    self.point_at(
                        self.argument - self.whisker_width / 2.0,
                        self.spread.h,
                    ),
                    self.point_at(
                        self.argument + self.whisker_width / 2.0,
                        self.spread.h,
                    ),
                );
                shapes.push(high_whisker_end);
            }
        }

        if self.spread.l < self.spread.o {
            let low_whisker = line_between(
                self.point_at(self.argument, self.spread.o),
                self.point_at(self.argument, self.spread.l),
            );
            shapes.push(low_whisker);
            if self.box_width > 0.0 {
                let low_whisker_end = line_between(
                    self.point_at(
                        self.argument - self.whisker_width / 2.0,
                        self.spread.l,
                    ),
                    self.point_at(
                        self.argument + self.whisker_width / 2.0,
                        self.spread.l,
                    ),
                );
                shapes.push(low_whisker_end);
            }
        }
    }

    pub(super) fn add_rulers_and_text(
        &self,
        parent: &KlinePlot,
        plot: &PlotConfig<'_>,
        shapes: &mut Vec<Shape>,
        cursors: &mut Vec<Cursor>,
    ) {
        let text: Option<String> = parent
            .element_formatter
            .as_ref()
            .map(|fmt| fmt(self, parent));

        add_rulers_and_text(self, plot, text, shapes, cursors);
    }
}

impl RectElement for KlinePlotPoint {
    fn name(&self) -> &str {
        self.name.as_str()
    }

    fn bounds_min(&self) -> PlotPoint {
        let argument = self.argument - self.box_width.max(self.whisker_width) / 2.0;
        let value = self.spread.l;
        self.point_at(argument, value)
    }

    fn bounds_max(&self) -> PlotPoint {
        let argument = self.argument + self.box_width.max(self.whisker_width) / 2.0;
        let value = self.spread.h;
        self.point_at(argument, value)
    }

    fn values_with_ruler(&self) -> Vec<PlotPoint> {
        let v = self.point_at(self.argument, self.spread.v);
        let q1 = self.point_at(self.argument, self.spread.o);
        let q3 = self.point_at(self.argument, self.spread.c);
        let upper = self.point_at(self.argument, self.spread.h);
        let lower = self.point_at(self.argument, self.spread.l);

        vec![v, q1, q3, upper, lower]
    }

    fn orientation(&self) -> Orientation {
        self.orientation
    }

    fn corner_value(&self) -> PlotPoint {
        self.point_at(self.argument, self.spread.h)
    }

    fn default_values_format(&self, transform: &PlotTransform) -> String {
        let scale = transform.dvalue_dpos();
        let scale = match self.orientation {
            Orientation::Horizontal => scale[0],
            Orientation::Vertical => scale[1],
        };
        let y_decimals = ((-scale.abs().log10()).ceil().at_least(0.0) as usize)
            .at_most(6)
            .at_least(1);
        format!(
            "Max = {max:.decimals$}\
             \nQuartile 3 = {q3:.decimals$}\
             \nMedian = {med:.decimals$}\
             \nQuartile 1 = {q1:.decimals$}\
             \nMin = {min:.decimals$}",
            max = self.spread.h,
            q3 = self.spread.c,
            med = self.spread.v,
            q1 = self.spread.o,
            min = self.spread.l,
            decimals = y_decimals
        )
    }
}
