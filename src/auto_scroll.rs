use egui::{Pos2, Rect};
use std::hash::Hash;

use crate::{ColumnOperations, ColumnOrdering, SelectableTable};

/// Handles automatic scrolling when dragging items near the edges of the table's view.
///
/// The `AutoScroll` struct allows the table to automatically scroll when the user drags items
/// near the top or bottom edge of the view. It provides configurable parameters such as
/// the speed of scrolling and the distances from the edges at which scrolling is triggered.
pub struct AutoScroll {
    /// The current vertical scroll offset.
    pub scroll_offset: f32,
    /// Whether auto-scrolling is enabled or disabled.
    pub enabled: bool,
    /// The minimum distance from the top edge before auto-scrolling starts. Extra space due to the
    /// header being in the way. Default: 200.0
    pub distance_from_min: f32,
    /// The minimum distance from the bottom edge before auto-scrolling starts. Default: 120.0
    pub distance_from_max: f32,
    /// The maximum speed at which auto-scrolling occurs. Default: 30.0
    pub max_speed: f32,
}

impl Default for AutoScroll {
    fn default() -> Self {
        Self {
            scroll_offset: 0.0,
            enabled: false,
            distance_from_min: 200.0,
            distance_from_max: 120.0,
            max_speed: 30.0,
        }
    }
}

impl AutoScroll {
    /// Creates a new instance of `AutoScroll` with the option to enable or disable auto-scrolling.
    ///
    /// # Parameters:
    /// - `enabled`: Whether auto-scrolling should be enabled.
    ///
    /// # Example:
    /// ```rust,ignore
    /// let auto_scroll = AutoScroll::new(true); // Enables auto-scrolling
    /// ```
    #[must_use]
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            ..Default::default()
        }
    }

    /// Sets the minimum distance from the top edge at which auto-scrolling is triggered.
    ///
    /// # Parameters:
    /// - `distance`: The distance from the top edge in pixels.
    ///
    /// # Returns:
    /// An updated instance of `AutoScroll` with the specified `distance_from_min` value.
    ///
    /// # Considerations:
    /// - Add some extra distance due to the header being in the way of selection
    ///
    /// # Example:
    /// ```rust,ignore
    /// let auto_scroll = AutoScroll::new(true).distance_from_min(100.0); // Auto-scrolls when 100 pixels from top
    /// ```
    #[must_use]
    pub const fn distance_from_min(mut self, distance: f32) -> Self {
        self.distance_from_min = distance;
        self
    }
    /// Sets the minimum distance from the bottom edge at which auto-scrolling is triggered.
    ///
    /// # Parameters:
    /// - `distance`: The distance from the bottom edge in pixels.
    ///
    /// # Returns:
    /// An updated instance of `AutoScroll` with the specified `distance_from_max` value.
    ///
    /// # Example:
    /// ```rust,ignore
    /// let auto_scroll = AutoScroll::new(true).distance_from_max(80.0); // Auto-scrolls when 80 pixels from bottom
    /// ```
    #[must_use]
    pub const fn distance_from_max(mut self, distance: f32) -> Self {
        self.distance_from_max = distance;
        self
    }

    /// Sets the maximum scroll speed when auto-scrolling is triggered.
    ///
    /// # Parameters:
    /// - `speed`: The maximum scroll speed
    ///
    /// # Returns:
    /// An updated instance of `AutoScroll` with the specified `max_speed`.
    ///
    /// # Example:
    /// ```rust,ignore
    /// let auto_scroll = AutoScroll::new(true).max_speed(50.0); // Sets the max scroll speed to 50.0
    /// ```
    #[must_use]
    pub const fn max_speed(mut self, speed: f32) -> Self {
        self.max_speed = speed;
        self
    }

    /// Calculate the position based on the rectangle and return the new vertical offset
    pub(crate) fn start_scroll(&mut self, max_rect: Rect, pointer: Option<Pos2>) -> Option<f32> {
        if !self.enabled {
            return None;
        }

        if let Some(loc) = pointer {
            let pointer_y = loc.y;

            // Min gets a bit more space as the header is along the way
            let min_y = max_rect.min.y + self.distance_from_min;
            let max_y = max_rect.max.y - self.distance_from_max;

            // Check if the pointer is within the allowed Y range
            let within_y = pointer_y >= min_y && pointer_y <= max_y;

            // Whether the mouse is above the minimum y point
            let above_y = pointer_y < min_y;
            // Whether the mouse is below the maximum y point
            let below_y = pointer_y > max_y;

            let max_speed = self.max_speed;

            // Only scroll if the pointer is outside the allowed Y range
            if !within_y {
                let distance: f32;
                let direction: f32; // -1 for upwards, 1 for downwards

                if above_y {
                    // If above, calculate distance from min_y and scroll upwards
                    distance = (min_y - pointer_y).abs();
                    direction = -1.0; // Scroll up
                } else if below_y {
                    // If below, calculate distance from max_y and scroll downwards
                    distance = (pointer_y - max_y).abs();
                    direction = 1.0; // Scroll down
                } else {
                    return None;
                }

                // Scale the speed by distance, with a cap at max_speed
                let speed_factor = max_speed * (distance / 100.0).clamp(0.1, 1.0);

                self.scroll_offset += direction * speed_factor;

                // Ensure scroll offset doesn't go negative
                if self.scroll_offset < 0.0 {
                    self.scroll_offset = 0.0;
                }

                return Some(self.scroll_offset);
            }
        }
        None
    }
}

/// Enables or configures auto-scrolling behavior in the table view.
impl<Row, F, Conf> SelectableTable<Row, F, Conf>
where
    Row: Clone + Send + Sync,
    F: Eq
        + Hash
        + Clone
        + Ord
        + Send
        + Sync
        + Default
        + ColumnOperations<Row, F, Conf>
        + ColumnOrdering<Row>,
    Conf: Default,
{
    pub(crate) const fn update_scroll_offset(&mut self, offset: f32) {
        self.auto_scroll.scroll_offset = offset;
    }

    /// Enables auto-scrolling when dragging near the edges of the view.
    ///
    /// # Returns:
    /// An updated instance of the table with auto-scrolling enabled.
    ///
    /// # Example:
    /// ```rust,ignore
    /// let table = SelectableTable::new(vec![col1, col2, col3]).auto_scroll()
    /// ```
    #[must_use]
    pub const fn auto_scroll(mut self) -> Self {
        self.auto_scroll.enabled = true;
        self
    }
    /// Sets the maximum scroll speed for auto-scrolling.
    ///
    /// # Parameters:
    /// - `speed`: The maximum scroll speed (in pixels per frame) when auto-scrolling is active.
    ///
    /// # Returns:
    /// An updated instance of the table with the new scroll speed.
    ///
    /// # Example:
    /// ```rust,ignore
    /// let table = SelectableTable::new(vec![col1, col2, col3])
    ///     .auto_scroll().scroll_speed(50.0);
    /// ```
    #[must_use]
    pub const fn scroll_speed(mut self, speed: f32) -> Self {
        self.auto_scroll.max_speed = speed;
        self
    }

    /// Configures the auto-scrolling behavior by providing a new `AutoScroll` instance.
    ///
    /// # Parameters:
    /// - `scroll`: A custom `AutoScroll` instance with defined scroll behavior.
    ///
    /// # Returns:
    /// An updated instance of the table with the provided `AutoScroll` configuration.
    ///
    /// # Example:
    /// ```rust,ignore
    /// let scroll_settings = AutoScroll::new(true).max_speed(50.0);
    /// let table = SelectableTable::new(vec![col1, col2, col3])
    ///     .set_auto_scroll(scroll_settings);
    /// ```
    #[must_use]
    pub const fn set_auto_scroll(mut self, scroll: AutoScroll) -> Self {
        self.auto_scroll = scroll;
        self
    }
    /// Updates the table's auto-scrolling settings with a new `AutoScroll` instance.
    ///
    /// # Parameters:
    /// - `scroll`: The new `AutoScroll` settings to apply.
    ///
    /// This method is used when you need to change the auto-scroll behavior at runtime.
    ///
    /// # Example:
    /// ```rust,ignore
    /// let new_scroll_settings = AutoScroll::new(true).max_speed(60.0);
    /// table.update_auto_scroll(new_scroll_settings); // Update the auto-scroll settings during runtime
    /// ```
    pub const fn update_auto_scroll(&mut self, scroll: AutoScroll) {
        self.auto_scroll = scroll;
    }
}
