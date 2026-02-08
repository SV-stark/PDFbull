//! Charts and Data Visualization
//!
//! Professional chart generation for PDF documents:
//! - Line, bar, pie, area, scatter charts
//! - Chart customization (axes, legends, grids)
//! - Integration with Platypus framework
//! - Vector graphics export

use super::error::{EnhancedError, Result};

/// Chart type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChartType {
    Line,
    Bar,
    HorizontalBar,
    StackedBar,
    GroupedBar,
    Pie,
    Pie3D,
    Donut,
    Area,
    StackedArea,
    Scatter,
    Bubble,
    Spider,
    Radar,
    Candlestick,
    BoxPlot,
}

/// Data series for charts
#[derive(Debug, Clone)]
pub struct DataSeries {
    pub name: String,
    pub data: Vec<f64>,
    pub color: Option<(f32, f32, f32)>,
}

impl DataSeries {
    /// Create new data series
    pub fn new(name: impl Into<String>, data: Vec<f64>) -> Self {
        Self {
            name: name.into(),
            data,
            color: None,
        }
    }

    /// Set color
    pub fn with_color(mut self, r: f32, g: f32, b: f32) -> Self {
        self.color = Some((r, g, b));
        self
    }
}

/// Axis configuration
#[derive(Debug, Clone)]
pub struct Axis {
    pub title: String,
    pub show_grid: bool,
    pub show_ticks: bool,
    pub show_labels: bool,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub tick_interval: Option<f64>,
}

impl Default for Axis {
    fn default() -> Self {
        Self {
            title: String::new(),
            show_grid: true,
            show_ticks: true,
            show_labels: true,
            min: None,
            max: None,
            tick_interval: None,
        }
    }
}

/// Legend configuration
#[derive(Debug, Clone)]
pub struct Legend {
    pub show: bool,
    pub position: LegendPosition,
    pub font_size: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegendPosition {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    Left,
    Right,
    Top,
    Bottom,
}

impl Default for Legend {
    fn default() -> Self {
        Self {
            show: true,
            position: LegendPosition::TopRight,
            font_size: 10.0,
        }
    }
}

/// Chart builder
#[derive(Debug)]
pub struct Chart {
    pub chart_type: ChartType,
    pub title: String,
    pub width: f32,
    pub height: f32,
    pub series: Vec<DataSeries>,
    pub labels: Vec<String>,
    pub x_axis: Axis,
    pub y_axis: Axis,
    pub legend: Legend,
    pub show_values: bool,
}

impl Chart {
    /// Create new chart
    pub fn new(chart_type: ChartType) -> Self {
        Self {
            chart_type,
            title: String::new(),
            width: 400.0,
            height: 300.0,
            series: vec![],
            labels: vec![],
            x_axis: Axis::default(),
            y_axis: Axis::default(),
            legend: Legend::default(),
            show_values: false,
        }
    }

    /// Set title
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Set dimensions
    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Add data series
    pub fn add_series(&mut self, series: DataSeries) {
        self.series.push(series);
    }

    /// Set X-axis labels
    pub fn set_labels(&mut self, labels: Vec<String>) {
        self.labels = labels;
    }

    /// Set X-axis configuration
    pub fn x_axis(mut self, axis: Axis) -> Self {
        self.x_axis = axis;
        self
    }

    /// Set Y-axis configuration
    pub fn y_axis(mut self, axis: Axis) -> Self {
        self.y_axis = axis;
        self
    }

    /// Render chart to PDF
    pub fn render(&self, pdf_path: &str, page: u32, x: f32, y: f32) -> Result<()> {
        // TODO: Implement chart rendering
        // 1. Use plotters or similar to generate chart
        // 2. Export as PDF vector graphics
        // 3. Embed in PDF at specified position
        // 4. Add axes, labels, legend

        Ok(())
    }

    /// Export chart as standalone PDF
    pub fn save(&self, output_path: &str) -> Result<()> {
        // TODO: Create new PDF with chart
        Ok(())
    }
}

/// Line chart builder
pub struct LineChart {
    chart: Chart,
}

impl LineChart {
    /// Create new line chart
    pub fn new() -> Self {
        Self {
            chart: Chart::new(ChartType::Line),
        }
    }

    /// Build chart
    pub fn build(self) -> Chart {
        self.chart
    }
}

impl Default for LineChart {
    fn default() -> Self {
        Self::new()
    }
}

/// Bar chart builder
pub struct BarChart {
    chart: Chart,
}

impl BarChart {
    /// Create new bar chart
    pub fn new() -> Self {
        Self {
            chart: Chart::new(ChartType::Bar),
        }
    }

    /// Set stacked mode
    pub fn stacked(mut self) -> Self {
        self.chart.chart_type = ChartType::StackedBar;
        self
    }

    /// Set grouped mode
    pub fn grouped(mut self) -> Self {
        self.chart.chart_type = ChartType::GroupedBar;
        self
    }

    /// Build chart
    pub fn build(self) -> Chart {
        self.chart
    }
}

impl Default for BarChart {
    fn default() -> Self {
        Self::new()
    }
}

/// Pie chart builder
pub struct PieChart {
    chart: Chart,
    exploded: Vec<f32>,
}

impl PieChart {
    /// Create new pie chart
    pub fn new() -> Self {
        Self {
            chart: Chart::new(ChartType::Pie),
            exploded: vec![],
        }
    }

    /// Set 3D mode
    pub fn three_d(mut self) -> Self {
        self.chart.chart_type = ChartType::Pie3D;
        self
    }

    /// Set donut mode
    pub fn donut(mut self) -> Self {
        self.chart.chart_type = ChartType::Donut;
        self
    }

    /// Explode slice
    pub fn explode_slice(&mut self, index: usize, distance: f32) {
        if self.exploded.len() <= index {
            self.exploded.resize(index + 1, 0.0);
        }
        self.exploded[index] = distance;
    }

    /// Build chart
    pub fn build(self) -> Chart {
        self.chart
    }
}

impl Default for PieChart {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_series() {
        let series = DataSeries::new("Sales", vec![10.0, 20.0, 30.0]).with_color(1.0, 0.0, 0.0);
        assert_eq!(series.name, "Sales");
        assert_eq!(series.data.len(), 3);
        assert!(series.color.is_some());
    }

    #[test]
    fn test_chart_creation() {
        let chart = Chart::new(ChartType::Line)
            .title("Test Chart")
            .size(500.0, 400.0);
        assert_eq!(chart.title, "Test Chart");
        assert_eq!(chart.width, 500.0);
        assert_eq!(chart.height, 400.0);
    }

    #[test]
    fn test_line_chart() {
        let chart = LineChart::new().build();
        assert_eq!(chart.chart_type, ChartType::Line);
    }

    #[test]
    fn test_bar_chart() {
        let chart = BarChart::new().stacked().build();
        assert_eq!(chart.chart_type, ChartType::StackedBar);
    }

    #[test]
    fn test_pie_chart() {
        let chart = PieChart::new().three_d().build();
        assert_eq!(chart.chart_type, ChartType::Pie3D);
    }
}
