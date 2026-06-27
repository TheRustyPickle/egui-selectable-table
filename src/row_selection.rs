use egui::Ui;
use egui::ahash::{HashMap, HashMapExt, HashSet};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::fmt::Write as _;
use std::hash::Hash;

use crate::{ColumnOperations, ColumnOrdering, SelectableRow, SelectableTable};

/// Functions related to selection of rows and columns
#[allow(clippy::too_many_lines)]
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
    pub(crate) fn select_single_row_cell(&mut self, id: i64, column_name: &F) {
        self.active_columns.insert(column_name.clone());
        self.active_rows.insert(id);

        let Some(target_index) = self.indexed_ids.get(&id) else {
            return;
        };
        let Some(target_row) = self.formatted_rows.get_mut(*target_index) else {
            return;
        };

        if self.select_full_row {
            self.active_columns.extend(self.all_columns.clone());
            target_row.selected_columns.extend(self.all_columns.clone());
        } else {
            target_row.selected_columns.insert(column_name.clone());
        }

        self.active_rows.insert(id);
    }

    /// Marks a row as selected, optionally selecting specific columns within the row.
    ///
    /// If a list of columns is provided, only those columns are marked as selected for the row.
    /// If no column list is provided, all columns in the row are marked as selected.
    ///
    /// # Parameters:
    /// - `id`: The unique identifier of the row to mark as selected.
    /// - `column`: An optional list of columns (`Vec<F>`) to mark as selected within the row. If `None`, all columns are selected.
    ///
    /// # Example:
    /// ```rust,ignore
    /// table.mark_row_as_selected(42, Some(vec!["Name", "Age"]));
    /// table.mark_row_as_selected(43, None); // Selects all columns in row 43
    /// ```
    pub fn mark_row_as_selected(&mut self, id: i64, column: Option<Vec<F>>) {
        let Some(target_index) = self.indexed_ids.get(&id) else {
            return;
        };

        let Some(target_row) = self.formatted_rows.get_mut(*target_index) else {
            return;
        };

        self.active_rows.insert(id);

        if let Some(column_list) = column {
            self.active_columns.extend(column_list.clone());

            target_row.selected_columns.extend(column_list);
        } else {
            self.active_columns.extend(self.all_columns.clone());

            target_row.selected_columns.extend(self.all_columns.clone());
        }
    }

    /// Called on every frame per cell that the pointer rests on during an active drag.
    ///
    /// Handles the core drag-selection logic:
    /// 1. Computes the column range from drag start to current pointer position
    /// 2. Updates `active_columns` to reflect exactly the rectangle's column set (not accumulated
    ///    across multiple drags). For Ctrl+drag this unions with existing columns on each row so
    ///    independent drag regions don't interfere.
    /// 3. Handles "backwards movement": when the mouse moves back through previously-selected
    ///    territory within a drag, it may need to unselect cells that are no longer in the
    ///    active path (the `row_contains_column` / `no_checking` block)
    /// 4. Calls `check_row_selection` bidirectionally to fill any rows skipped by fast mouse
    ///    movement between the drag start row and current row
    /// 5. Calls `remove_row_selection` to sync rows inside the drag range and clear rows that
    ///    fell out of it (only for non-Ctrl; Ctrl+drag preserves committed selections)
    pub(crate) fn select_dragged_row_cell(
        &mut self,
        id: i64,
        column_name: &F,
        is_ctrl_pressed: bool,
    ) {
        // If the pointer hasn't actually moved to a new cell since last frame, nothing to do
        if self.last_active_row == Some(id) && self.last_active_column == Some(column_name.clone())
        {
            return;
        }

        if self.formatted_rows.is_empty() {
            return;
        }

        self.beyond_drag_point = true;

        let drag_start = self.drag_started_on.clone().expect("Drag start not found");

        // Compute the column range from drag start to current position.
        let drag_start_num = self.column_to_num(&drag_start.1);
        let ongoing_column_num = self.column_to_num(column_name);

        let col_min = drag_start_num.min(ongoing_column_num);
        let col_max = drag_start_num.max(ongoing_column_num);

        let new_column_set: HashSet<F> = (col_min..=col_max)
            .map(|i| self.all_columns[i].clone())
            .collect();

        if is_ctrl_pressed {
            for col in &new_column_set {
                self.active_columns.insert(col.clone());
            }
        } else {
            self.active_columns = new_column_set;
        }

        let Some(current_row_index) = self.indexed_ids.get(&id).copied() else {
            return;
        };
        let Some(current_row) = self.formatted_rows.get_mut(current_row_index) else {
            return;
        };

        // Detect "backwards movement": if the current row already has this column
        // selected, the mouse may be moving back through previously-selected territory.
        // We need to unselect cells that are no longer on the active path.
        let row_contains_column = current_row.selected_columns.contains(column_name);

        let mut no_checking = false;
        // Backwards movement handling:
        //   Row: col1 col2 (mouse here) col3 col4
        // If mouse was on col4 last frame and is now on col2 on the same row,
        // col4 should be unselected from this row since the mouse retracted.
        // When crossing to a different row altogether, we clear the entire
        // previous row's selection so the drag rectangle resizes cleanly.
        if row_contains_column
            && self.last_active_row.is_some()
            && self.last_active_column.is_some()
        {
            if let (Some(last_active_column), Some(last_active_row)) =
                (self.last_active_column.clone(), self.last_active_row)
            {
                if &last_active_column != column_name && last_active_row == id {
                    current_row.selected_columns.remove(&last_active_column);
                    self.active_columns.remove(&last_active_column);
                }

                let Some(last_row_index) = self.indexed_ids.get(&last_active_row).copied() else {
                    return;
                };
                let Some(last_row) = self.formatted_rows.get_mut(last_row_index) else {
                    return;
                };

                self.last_active_row = Some(id);

                if id == last_row.id {
                    if &last_active_column != column_name {
                        self.last_active_column = Some(column_name.clone());
                    }
                } else {
                    no_checking = true;
                    last_row.selected_columns.clear();
                }
            }
        } else {
            // First time this drag touches this row. Apply the active column set.
            // For Ctrl+drag, extend (union) to preserve columns from prior selections.
            self.active_rows.insert(current_row.id);
            self.last_active_row = Some(id);
            self.last_active_column = Some(column_name.clone());
            if is_ctrl_pressed {
                current_row
                    .selected_columns
                    .extend(self.active_columns.clone());
            } else {
                current_row
                    .selected_columns
                    .clone_from(&self.active_columns);
            }
        }

        let Some(current_row_index) = self.indexed_ids.get(&id).copied() else {
            return;
        };

        let Some(drag_start_index) = self.indexed_ids.get(&drag_start.0).copied() else {
            return;
        };

        if !no_checking {
            // Walk both directions from current row toward drag start to fill
            // any rows skipped by fast mouse movement (missed pointer-in-cell events).
            self.check_row_selection(true, current_row_index, drag_start_index, is_ctrl_pressed);
            self.check_row_selection(false, current_row_index, drag_start_index, is_ctrl_pressed);
        }
        // Sync rows inside the drag range and clear rows that left it
        self.remove_row_selection(current_row_index, drag_start_index, is_ctrl_pressed);
    }

    /// Walks from the current row toward (or away from) the drag-start row, selecting
    /// intermediate rows to fill gaps from fast mouse movement. Stops when hitting either
    /// the drag-start row boundary or a row with no selection (end of previously-selected
    /// territory). Uses a loop instead of recursion to avoid stack overflow on large tables.
    fn check_row_selection(
        &mut self,
        check_previous: bool,
        index: usize,
        drag_start: usize,
        is_ctrl_pressed: bool,
    ) {
        let mut idx = index;
        loop {
            if idx == 0 && check_previous {
                return;
            }
            if idx + 1 == self.formatted_rows.len() && !check_previous {
                return;
            }
            idx = if check_previous { idx - 1 } else { idx + 1 };

            let Some(current_row) = self.formatted_rows.get(idx) else {
                return;
            };

            // Consider a row "unselected" when we've walked past the drag-start
            // boundary AND the row has no columns selected (meaning it's beyond
            // the previously-selected range). Rows between current and drag-start
            // are always treated as selected (unselected_row = false) so they
            // get filled in even if they were missed by fast mouse movement.
            let unselected_row = if (check_previous && idx >= drag_start)
                || (!check_previous && idx <= drag_start)
            {
                false
            } else {
                current_row.selected_columns.is_empty()
            };

            if unselected_row {
                return;
            }

            let Some(target_row) = self.formatted_rows.get_mut(idx) else {
                return;
            };

            if self.select_full_row {
                target_row.selected_columns.extend(self.all_columns.clone());
            } else if is_ctrl_pressed {
                target_row
                    .selected_columns
                    .extend(self.active_columns.clone());
            } else {
                target_row.selected_columns.clone_from(&self.active_columns);
            }
            self.active_rows.insert(target_row.id);
        }
    }

    /// Iterates all currently-active rows and syncs their selection state.
    /// Rows inside the drag range get their columns updated to match `active_columns`.
    /// For non-Ctrl drags, rows outside the range are cleared entirely.
    /// For Ctrl+drags, rows outside the range are left untouched (preserving prior selections).
    fn remove_row_selection(
        &mut self,
        current_index: usize,
        drag_start: usize,
        is_ctrl_pressed: bool,
    ) {
        let active_ids = self.active_rows.clone();
        for id in active_ids {
            let Some(ongoing_index) = self.indexed_ids.get(&id).copied() else {
                continue;
            };
            let Some(target_row) = self.formatted_rows.get_mut(ongoing_index) else {
                continue;
            };

            if current_index > drag_start {
                if ongoing_index >= drag_start && ongoing_index <= current_index {
                    if self.select_full_row {
                        target_row.selected_columns.extend(self.all_columns.clone());
                    } else if is_ctrl_pressed {
                        target_row
                            .selected_columns
                            .extend(self.active_columns.clone());
                    } else {
                        target_row.selected_columns.clone_from(&self.active_columns);
                    }
                } else if !is_ctrl_pressed {
                    target_row.selected_columns.clear();
                    self.active_rows.remove(&target_row.id);
                }
            } else if ongoing_index <= drag_start && ongoing_index >= current_index {
                if self.select_full_row {
                    target_row.selected_columns.extend(self.all_columns.clone());
                } else if is_ctrl_pressed {
                    target_row
                        .selected_columns
                        .extend(self.active_columns.clone());
                } else {
                    target_row.selected_columns.clone_from(&self.active_columns);
                }
            } else if !is_ctrl_pressed {
                target_row.selected_columns.clear();
                self.active_rows.remove(&target_row.id);
            }
        }
    }

    /// Unselects all currently selected rows and columns.
    ///
    /// Clears the selection in both rows and columns, and resets internal tracking of active rows
    /// and columns. After this call, there will be no selected rows or columns in the table.
    ///
    /// # Panics:
    /// This method will panic if the indexed ID or the corresponding row cannot be found.
    ///
    /// # Example:
    /// ```rust,ignore
    /// table.unselect_all(); // Unselects everything in the table.
    /// ```
    pub fn unselect_all(&mut self) {
        for id in &self.active_rows {
            if let Some(id_index) = self.indexed_ids.get(id)
                && let Some(target_row) = self.formatted_rows.get_mut(*id_index)
            {
                target_row.selected_columns.clear();
            }
        }
        self.active_columns.clear();
        self.last_active_row = None;
        self.last_active_column = None;
        self.active_rows.clear();
    }

    /// Selects all rows and columns in the table.
    ///
    /// After calling this method, all rows will have all columns selected and visible immediately.
    ///
    /// # Example:
    /// ```rust,ignore
    /// table.select_all(); // Selects all rows and columns.
    /// ```
    pub fn select_all(&mut self) {
        self.active_columns.extend(self.all_columns.clone());
        self.active_rows.clear();
        self.last_active_row = None;
        self.last_active_column = None;

        for row in &mut self.formatted_rows {
            if row.selected_columns.len() != self.all_columns.len() {
                row.selected_columns.extend(self.all_columns.clone());
            }
            self.active_rows.insert(row.id);
        }
    }

    /// Retrieves the currently selected rows.
    ///
    /// This method returns a vector of the rows that have one or more columns selected.
    ///
    /// If the `select_full_row` flag is enabled, it will ensure that all columns are selected for
    /// each active row.
    ///
    /// # Returns:
    /// A `Vec` of `SelectableRow` instances that are currently selected.
    ///
    /// # Example:
    /// ```rust,ignore
    /// let selected_rows = table.get_selected_rows();
    /// ```
    pub fn get_selected_rows(&mut self) -> Vec<SelectableRow<Row, F>> {
        let mut selected_rows = Vec::new();
        if self.select_full_row {
            self.active_columns.extend(self.all_columns.clone());
        }

        // Cannot use active rows to iter as that does not maintain any proper format
        for row in &self.formatted_rows {
            if row.selected_columns.is_empty() {
                continue;
            }
            selected_rows.push(row.clone());

            // We already got all the active rows if this matches
            if selected_rows.len() == self.active_rows.len() {
                break;
            }
        }
        selected_rows
    }

    /// Retrieves the currently selected rows but in no particular order. Can be faster than
    /// [`get_selected_rows`](#method.get_selected_rows) as it uses rayon for parallel processing
    /// and only checks the active rows instead of every single row.
    ///
    /// This method returns a vector of the rows that have one or more columns selected.
    ///
    /// If the `select_full_row` flag is enabled, it will ensure that all columns are selected for
    /// each active row.
    ///
    /// # Returns:
    /// A `Vec` of `SelectableRow` instances that are currently selected.
    ///
    /// # Example:
    /// ```rust,ignore
    /// let selected_rows = table.get_selected_rows();
    /// ```
    pub fn get_selected_rows_unsorted(&mut self) -> Vec<SelectableRow<Row, F>> {
        if self.select_full_row {
            self.active_columns.extend(self.all_columns.clone());
        }

        self.active_rows
            .par_iter()
            .map(|row_id| {
                let row_index = self
                    .indexed_ids
                    .get(row_id)
                    .expect("Could not get id index");
                let target_row = self
                    .formatted_rows
                    .get(*row_index)
                    .expect("Could not get row");

                target_row.clone()
            })
            .collect()
    }

    /// Copies selected cells to the system clipboard in a tabular format.
    ///
    /// This method copies only the selected cells from each row to the clipboard, and ensures
    /// that the column widths align for better readability when pasted into a text editor or spreadsheet.
    ///
    /// # Parameters:
    /// - `ui`: The UI context used for clipboard interaction.
    ///
    /// # Example:
    /// ```rust,ignore
    /// table.copy_selected_cells(&mut ui);
    /// ```
    pub fn copy_selected_cells(&mut self, ui: &mut Ui) {
        let mut selected_rows = Vec::new();
        if self.select_full_row {
            self.active_columns.extend(self.all_columns.clone());
        }

        let mut column_max_length = HashMap::new();

        // Iter through all the rows and find the rows that have at least one column as selected.
        // Keep track of the biggest length of a value of a column
        // active rows cannot be used here because hashset does not maintain an order.
        // So iterating will give the rows in a different order than what is shown in the ui
        for row in &self.formatted_rows {
            if row.selected_columns.is_empty() {
                continue;
            }

            for column in &self.active_columns {
                if row.selected_columns.contains(column) {
                    let column_text = column.column_text(&row.row_data);
                    let field_length = column_text.len();
                    let entry = column_max_length.entry(column).or_insert(0);
                    if field_length > *entry {
                        column_max_length.insert(column, field_length);
                    }
                }
            }
            selected_rows.push(row);
            // We already got all the active rows if this matches
            if selected_rows.len() == self.active_rows.len() {
                break;
            }
        }

        let mut to_copy = String::new();

        // Target is to ensure a fixed length after each column value of a row.
        // If for example highest len is 10 but the current row's
        // column value is 5, we will add the column value and add 5 more space after that
        // to ensure alignment
        for row in selected_rows {
            let mut ongoing_column = self.first_column();
            let mut row_text = String::new();
            loop {
                if self.active_columns.contains(&ongoing_column)
                    && row.selected_columns.contains(&ongoing_column)
                {
                    let column_text = ongoing_column.column_text(&row.row_data);
                    let _ = write!(
                        row_text,
                        "{:<width$}",
                        column_text,
                        width = column_max_length[&ongoing_column] + 1
                    );
                } else if self.active_columns.contains(&ongoing_column)
                    && !row.selected_columns.contains(&ongoing_column)
                {
                    let _ = write!(
                        row_text,
                        "{:<width$}",
                        "",
                        width = column_max_length[&ongoing_column] + 1
                    );
                }
                if self.last_column() == ongoing_column {
                    break;
                }
                ongoing_column = self.next_column(&ongoing_column);
            }
            to_copy.push_str(&row_text);
            to_copy.push('\n');
        }
        ui.ctx().copy_text(to_copy);
    }

    /// Enables the selection of full rows in the table.
    ///
    /// After calling this method, selecting any column in a row will result in the entire row being selected.
    ///
    /// # Returns:
    /// A new instance of the table with full row selection enabled.
    ///
    /// # Example:
    /// ```rust,ignore
    /// let table = SelectableTable::new(vec![col1, col2, col3])
    ///     .select_full_row();
    /// ```
    #[must_use]
    pub const fn select_full_row(mut self) -> Self {
        self.select_full_row = true;
        self
    }

    /// Sets whether the table should select full rows when a column is selected.
    ///
    /// # Parameters:
    /// - `status`: `true` to enable full row selection, `false` to disable it.
    ///
    /// # Example:
    /// ```rust,ignore
    /// table.set_select_full_row(true); // Enable full row selection.
    /// ```
    pub const fn set_select_full_row(&mut self, status: bool) {
        self.select_full_row = status;
    }

    /// Returns the total number of currently selected rows.
    ///
    /// # Returns:
    /// - `usize`: The number of selected rows.
    ///
    /// # Example:
    /// ```rust,ignore
    /// let selected_count = table.get_total_selected_rows();
    /// println!("{} rows selected", selected_count);
    /// ```
    pub fn get_total_selected_rows(&mut self) -> usize {
        self.active_rows.len()
    }
}
