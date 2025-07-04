use egui::ahash::{HashMap, HashSet, HashSetExt};
use rayon::prelude::*;
use std::hash::Hash;

use crate::{ColumnOperations, ColumnOrdering, SelectableRow, SelectableTable, SortOrder};

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
    /// Modify or add rows to the table. Changes are not immediately reflected in the UI.
    /// You must call [`recreate_rows`](#method.recreate_rows) or [`recreate_rows_no_unselect`](#method.recreate_rows_no_unselect) to apply these changes visually.
    ///
    /// # Parameters:
    /// - `table`: A closure that takes a mutable reference to the rows and optionally returns a new row.
    ///   If a row is returned, it will be added to the table.
    ///
    /// # Auto Reload:
    /// - Use [`auto_reload`](#method.auto_reload) to automatically refresh the UI after a specified
    ///   number of row modifications or additions.
    ///
    /// # Returns
    /// * `Option<i64>` - The row id that is used internally for the new row
    ///
    /// # Example:
    /// ```rust,ignore
    /// let new_row_id = table.add_modify_row(|rows| {
    ///     let my_row = rows.get_mut(row_id).unwrap();
    ///     // modify your row as necessary
    ///
    ///     let new_row = MyRow {
    ///         // Define your row values
    ///     };
    ///     Some(new_row) // Optionally add a new row
    /// });
    /// ```
    pub fn add_modify_row<Fn>(&mut self, table: Fn) -> Option<i64>
    where
        Fn: FnOnce(&mut HashMap<i64, SelectableRow<Row, F>>) -> Option<Row>,
    {
        let new_row = table(&mut self.rows);

        let mut to_return = None;

        if let Some(row) = new_row {
            let selected_columns = HashSet::new();
            let new_row = SelectableRow {
                row_data: row,
                id: self.last_id_used,
                selected_columns,
            };
            to_return = Some(self.last_id_used);
            self.rows.insert(new_row.id, new_row);
            self.last_id_used += 1;
        }

        let reload = self.auto_reload.increment_count();

        if reload {
            self.recreate_rows();
        }
        to_return
    }

    /// Modify only the rows currently displayed in the UI.
    ///
    /// This provides direct access to the currently formatted rows for lightweight updates.
    ///
    /// # Important:
    /// - This does **not** require calling `recreate_rows` to reflect changes in the UI.
    /// - **Do not delete rows** from inside this closure — doing so will **cause a panic** and break internal assumptions.
    ///   To safely delete a row, use [`add_modify_row`](#method.add_modify_row) and then call [`recreate_rows`](#method.recreate_rows) or [`recreate_rows_no_unselect`](#method.recreate_rows_no_unselect).
    /// - Can be used alongside [`add_modify_row`](#method.add_modify_row) to show updated data immediately.
    ///   When row recreation happens, the modified data will be preserved as long as it's updated via [`add_modify_row`](#method.add_modify_row).
    /// - Does not contribute toward [`auto_reload`](#method.auto_reload) count.
    ///
    /// # Parameters:
    /// - `table`: A closure that takes a mutable reference to the currently formatted rows and an index map.
    ///
    /// # Example:
    /// ```rust,ignore
    /// table.modify_shown_row(|formatted_rows, indexed_ids| {
    ///     let row_id = 0;
    ///     let target_index = indexed_ids.get(&row_id).unwrap();
    ///     let row = formatted_rows.get_mut(*target_index).unwrap();
    ///     // Safely modify row contents here
    /// });
    /// ```
    pub fn modify_shown_row<Fn>(&mut self, mut rows: Fn)
    where
        Fn: FnMut(&mut Vec<SelectableRow<Row, F>>, &HashMap<i64, usize>),
    {
        rows(&mut self.formatted_rows, &self.indexed_ids);
    }

    /// Adds a new row to the bottom of the table without applying any sorting logic.
    ///
    /// This method inserts the row as-is at the end of the table, assigns it a unique ID, and
    /// returns it as a `SelectableRow`. This does **not**
    /// require calling [`recreate_rows`](#method.recreate_rows) for the row to appear in the UI.
    ///
    /// # Parameters:
    /// - `row`: The data to insert into the table.
    ///
    /// # Returns:
    /// - `SelectableRow<Row, F>`: The newly added row wrapped in a `SelectableRow`.
    ///
    /// # Example:
    /// ```rust,ignore
    /// let row = Row::new(vec![cell1, cell2, cell3]);
    /// let added_row = table.add_unsorted_row(row);
    /// ```
    pub fn add_unsorted_row(&mut self, row: Row) -> SelectableRow<Row, F> {
        let selected_columns = HashSet::new();
        let new_row = SelectableRow {
            row_data: row,
            id: self.last_id_used,
            selected_columns,
        };

        self.formatted_rows.push(new_row.clone());
        self.indexed_ids
            .insert(new_row.id, self.formatted_rows.len() - 1);
        self.rows.insert(new_row.id, new_row.clone());
        self.last_id_used += 1;
        new_row
    }

    /// Sort the rows to the current sorting order and column and save them for later reuse
    pub(crate) fn sort_rows(&mut self) {
        let mut row_data: Vec<SelectableRow<Row, F>> =
            self.rows.par_iter().map(|(_, v)| v.clone()).collect();

        row_data.par_sort_by(|a, b| {
            let ordering = self.sorted_by.order_by(&a.row_data, &b.row_data);
            match self.sort_order {
                SortOrder::Ascending => ordering,
                SortOrder::Descending => ordering.reverse(),
            }
        });

        let indexed_data = row_data
            .par_iter()
            .enumerate()
            .map(|(index, row)| (row.id, index))
            .collect();

        self.indexed_ids = indexed_data;
        self.formatted_rows = row_data;
    }
}
