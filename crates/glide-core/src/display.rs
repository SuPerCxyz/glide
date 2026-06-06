use serde::{Deserialize, Serialize};

/// A single display/monitor in the layout.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayInfo {
    /// Unique display ID (e.g. device_id + screen index).
    pub id: String,
    /// Device owning this display.
    pub device_id: String,
    /// Display name (e.g. "Built-in Display", "External Monitor").
    pub name: String,
    /// Native resolution width in pixels.
    pub width: i32,
    /// Native resolution height in pixels.
    pub height: i32,
    /// X position in the virtual desktop layout.
    pub x: i32,
    /// Y position in the virtual desktop layout.
    pub y: i32,
    /// DPI scale factor (1.0 = 100%, 1.5 = 150%, 2.0 = 200%).
    pub scale: f64,
    /// Whether this is the primary display.
    pub primary: bool,
}

impl DisplayInfo {
    /// Get the effective (scaled) resolution.
    pub fn effective_width(&self) -> i32 {
        (self.width as f64 / self.scale) as i32
    }

    pub fn effective_height(&self) -> i32 {
        (self.height as f64 / self.scale) as i32
    }

    /// Check if a point (x, y) is within this display's bounds.
    pub fn contains_point(&self, x: i32, y: i32) -> bool {
        x >= self.x
            && x < self.x + self.effective_width()
            && y >= self.y
            && y < self.y + self.effective_height()
    }

    /// Get the right edge X coordinate.
    pub fn right_edge(&self) -> i32 {
        self.x + self.effective_width()
    }

    /// Get the bottom edge Y coordinate.
    pub fn bottom_edge(&self) -> i32 {
        self.y + self.effective_height()
    }
}

/// The full virtual desktop layout.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayLayout {
    /// All displays in the layout.
    pub displays: Vec<DisplayInfo>,
}

impl DisplayLayout {
    pub fn new() -> Self {
        Self {
            displays: Vec::new(),
        }
    }

    /// Add a display to the layout.
    pub fn add_display(&mut self, display: DisplayInfo) {
        self.displays.push(display);
    }

    /// Remove a display by ID.
    pub fn remove_display(&mut self, id: &str) {
        self.displays.retain(|d| d.id != id);
    }

    /// Update a display's position.
    pub fn move_display(&mut self, id: &str, x: i32, y: i32) {
        if let Some(d) = self.displays.iter_mut().find(|d| d.id == id) {
            d.x = x;
            d.y = y;
        }
    }

    /// Update a display's resolution.
    pub fn resize_display(&mut self, id: &str, width: i32, height: i32) {
        if let Some(d) = self.displays.iter_mut().find(|d| d.id == id) {
            d.width = width;
            d.height = height;
        }
    }

    /// Update a display's DPI scale.
    pub fn set_scale(&mut self, id: &str, scale: f64) {
        if let Some(d) = self.displays.iter_mut().find(|d| d.id == id) {
            d.scale = scale.clamp(0.5, 4.0);
        }
    }

    /// Find the display containing a given point.
    pub fn find_display_at(&self, x: i32, y: i32) -> Option<&DisplayInfo> {
        self.displays.iter().find(|d| d.contains_point(x, y))
    }

    /// Map coordinates from one display to another (cross-screen mapping).
    /// Converts a point from source display coordinates to target display coordinates.
    pub fn map_coordinates(
        &self,
        src_display_id: &str,
        src_x: i32,
        src_y: i32,
        dst_display_id: &str,
    ) -> Option<(i32, i32)> {
        let src = self.displays.iter().find(|d| d.id == src_display_id)?;
        let dst = self.displays.iter().find(|d| d.id == dst_display_id)?;

        // Normalize to 0..1 range on source (using width-1 for inclusive pixel coords).
        let norm_x = if src.effective_width() > 1 {
            (src_x - src.x) as f64 / (src.effective_width() - 1) as f64
        } else {
            0.0
        };
        let norm_y = if src.effective_height() > 1 {
            (src_y - src.y) as f64 / (src.effective_height() - 1) as f64
        } else {
            0.0
        };

        // Map to destination coordinates.
        let dst_x = dst.x + (norm_x * (dst.effective_width() - 1) as f64).round() as i32;
        let dst_y = dst.y + (norm_y * (dst.effective_height() - 1) as f64).round() as i32;

        Some((dst_x, dst_y))
    }

    /// Get the virtual desktop bounding box.
    pub fn bounding_box(&self) -> (i32, i32, i32, i32) {
        if self.displays.is_empty() {
            return (0, 0, 0, 0);
        }
        let mut min_x = i32::MAX;
        let mut min_y = i32::MAX;
        let mut max_x = i32::MIN;
        let mut max_y = i32::MIN;
        for d in &self.displays {
            min_x = min_x.min(d.x);
            min_y = min_y.min(d.y);
            max_x = max_x.max(d.right_edge());
            max_y = max_y.max(d.bottom_edge());
        }
        (min_x, min_y, max_x, max_y)
    }

    /// Check if two displays are adjacent (sharing an edge).
    pub fn are_adjacent(&self, id1: &str, id2: &str) -> bool {
        let d1 = match self.displays.iter().find(|d| d.id == id1) {
            Some(d) => d,
            None => return false,
        };
        let d2 = match self.displays.iter().find(|d| d.id == id2) {
            Some(d) => d,
            None => return false,
        };

        // Check right-left adjacency.
        let right_left =
            d1.right_edge() == d2.x && d1.y < d2.bottom_edge() && d1.bottom_edge() > d2.y;

        // Check left-right adjacency.
        let left_right =
            d2.right_edge() == d1.x && d2.y < d1.bottom_edge() && d2.bottom_edge() > d1.y;

        // Check top-bottom adjacency.
        let top_bottom =
            d1.bottom_edge() == d2.y && d1.x < d2.right_edge() && d1.right_edge() > d2.x;

        // Check bottom-top adjacency.
        let bottom_top =
            d2.bottom_edge() == d1.y && d2.x < d1.right_edge() && d2.right_edge() > d1.x;

        right_left || left_right || top_bottom || bottom_top
    }
}

impl Default for DisplayLayout {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_display(id: &str, x: i32, y: i32, w: i32, h: i32) -> DisplayInfo {
        DisplayInfo {
            id: id.to_string(),
            device_id: "test".to_string(),
            name: format!("Display {}", id),
            width: w,
            height: h,
            x,
            y,
            scale: 1.0,
            primary: id == "primary",
        }
    }

    #[test]
    fn test_display_contains_point() {
        let d = make_display("d1", 0, 0, 1920, 1080);
        assert!(d.contains_point(0, 0));
        assert!(d.contains_point(1919, 1079));
        assert!(!d.contains_point(1920, 1080));
        assert!(!d.contains_point(-1, -1));
    }

    #[test]
    fn test_display_scaled_resolution() {
        let mut d = make_display("d1", 0, 0, 1920, 1080);
        d.scale = 1.5;
        assert_eq!(d.effective_width(), 1280);
        assert_eq!(d.effective_height(), 720);
    }

    #[test]
    fn test_layout_add_remove() {
        let mut layout = DisplayLayout::new();
        layout.add_display(make_display("d1", 0, 0, 1920, 1080));
        layout.add_display(make_display("d2", 1920, 0, 2560, 1440));
        assert_eq!(layout.displays.len(), 2);

        layout.remove_display("d1");
        assert_eq!(layout.displays.len(), 1);
    }

    #[test]
    fn test_layout_move_display() {
        let mut layout = DisplayLayout::new();
        layout.add_display(make_display("d1", 0, 0, 1920, 1080));
        layout.move_display("d1", 100, 200);
        assert_eq!(layout.displays[0].x, 100);
        assert_eq!(layout.displays[0].y, 200);
    }

    #[test]
    fn test_layout_resize_display() {
        let mut layout = DisplayLayout::new();
        layout.add_display(make_display("d1", 0, 0, 1920, 1080));
        layout.resize_display("d1", 2560, 1440);
        assert_eq!(layout.displays[0].width, 2560);
        assert_eq!(layout.displays[0].height, 1440);
    }

    #[test]
    fn test_find_display_at() {
        let mut layout = DisplayLayout::new();
        layout.add_display(make_display("left", 0, 0, 1920, 1080));
        layout.add_display(make_display("right", 1920, 0, 2560, 1440));

        assert_eq!(layout.find_display_at(500, 500).unwrap().id, "left");
        assert_eq!(layout.find_display_at(2000, 500).unwrap().id, "right");
        assert!(layout.find_display_at(5000, 5000).is_none());
    }

    #[test]
    fn test_map_coordinates_same_resolution() {
        let mut layout = DisplayLayout::new();
        layout.add_display(make_display("a", 0, 0, 1920, 1080));
        layout.add_display(make_display("b", 1920, 0, 1920, 1080));

        // Map center of A to B.
        let (bx, by) = layout.map_coordinates("a", 960, 540, "b").unwrap();
        assert_eq!(bx, 1920 + 960); // 2880
        assert_eq!(by, 540);
    }

    #[test]
    fn test_map_coordinates_different_resolution() {
        let mut layout = DisplayLayout::new();
        layout.add_display(make_display("a", 0, 0, 1920, 1080));
        layout.add_display(make_display("b", 1920, 0, 2560, 1440));

        // Map top-left of A to B.
        let (bx, by) = layout.map_coordinates("a", 0, 0, "b").unwrap();
        assert_eq!(bx, 1920);
        assert_eq!(by, 0);

        // Map bottom-right of A to B.
        let (bx, by) = layout.map_coordinates("a", 1919, 1079, "b").unwrap();
        assert_eq!(bx, 1920 + 2560 - 1); // 4479
        assert_eq!(by, 1439);
    }

    #[test]
    fn test_adjacent_displays() {
        let mut layout = DisplayLayout::new();
        layout.add_display(make_display("left", 0, 0, 1920, 1080));
        layout.add_display(make_display("right", 1920, 0, 1920, 1080));
        layout.add_display(make_display("far", 5000, 0, 1920, 1080));

        assert!(layout.are_adjacent("left", "right"));
        assert!(layout.are_adjacent("right", "left"));
        assert!(!layout.are_adjacent("left", "far"));
    }

    #[test]
    fn test_adjacent_vertical() {
        let mut layout = DisplayLayout::new();
        layout.add_display(make_display("top", 0, 0, 1920, 1080));
        layout.add_display(make_display("bottom", 0, 1080, 1920, 1080));

        assert!(layout.are_adjacent("top", "bottom"));
        assert!(layout.are_adjacent("bottom", "top"));
    }

    #[test]
    fn test_bounding_box() {
        let mut layout = DisplayLayout::new();
        layout.add_display(make_display("a", 0, 0, 1920, 1080));
        layout.add_display(make_display("b", 1920, 0, 2560, 1440));

        let (min_x, min_y, max_x, max_y) = layout.bounding_box();
        assert_eq!(min_x, 0);
        assert_eq!(min_y, 0);
        assert_eq!(max_x, 1920 + 2560);
        assert_eq!(max_y, 1440);
    }

    #[test]
    fn test_set_scale() {
        let mut layout = DisplayLayout::new();
        layout.add_display(make_display("d1", 0, 0, 3840, 2160));
        layout.set_scale("d1", 2.0);
        assert_eq!(layout.displays[0].scale, 2.0);
        assert_eq!(layout.displays[0].effective_width(), 1920);
        assert_eq!(layout.displays[0].effective_height(), 1080);
    }
}
