use std::hash::Hash;

use crate::{ColumnOperations, ColumnOrdering, SelectableTable};

#[derive(Default)]
pub struct AutoReload {
    pub reload_after: Option<u32>,
    pub reload_count: u32,
}
impl AutoReload {
    /// Increase the current reload count and return bool based on if it is equal or above the count it is
    /// supposed to reload at
    pub(crate) const fn increment_count(&mut self) -> bool {
        self.reload_count += 1;
        if let Some(count) = self.reload_after {
            let reload = self.reload_count >= count;
            if reload {
                self.reload_count = 0;
            }
            reload
        } else {
            false
        }
    }
}

/// Enables or configures auto-reloading behavior in the table view.
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
    /// Enable automatic row recreation in the UI after a specified number of modifications or new rows.
    ///
    /// This function configures the table to automatically recreate its displayed rows when a certain
    /// number of new rows or row modifications have been made. Can be useful without having to
    /// manually keep track of modifications or when to reload.
    ///
    /// # Parameters:
    /// - `count`: The number of updates (inserts or modifications) that will trigger an automatic row recreation.
    ///     - If `count` is high, the table will refresh less frequently, leading to potentially outdated rows being shown in the UI for longer.
    ///     - If `count` is too low, frequent row recreation may result in performance overhead.
    ///
    /// # Considerations:
    /// - Tune the `count` value based on the expected rate of updates. For instance, if new rows or modifications
    ///   occur at a rate of 1000 rows per second, a `count` between 500 and 1000 may offer the best balance between
    ///   performance and up-to-date display.
    ///
    /// # Example:
    /// ```rust,ignore
    /// let table = SelectableTable::new(vec![col1, col2, col3])
    ///     .config(my_config).auto_reload(Some(500));
    /// ```
    #[must_use]
    pub const fn auto_reload(mut self, count: u32) -> Self {
        self.auto_reload.reload_after = Some(count);
        self.auto_reload.reload_count = 0;
        self
    }
    /// Manually set the auto-reload threshold. This lets you change the threshold dynamically.
    ///
    /// # Parameters:
    /// - `count`: Optionally specify how many updates (new or modified rows) should occur before rows are automatically recreated.
    ///     - If `None` is provided, auto-reloading is disabled.
    ///
    /// # Example:
    /// ```rust,ignore
    /// table.set_auto_reload(Some(1000)); // Reload after 1000 updates.
    /// table.set_auto_reload(None); // Disable auto-reloading.
    /// ```
    pub const fn set_auto_reload(&mut self, count: Option<u32>) {
        self.auto_reload.reload_after = count;
        self.auto_reload.reload_count = 0;
    }
}
