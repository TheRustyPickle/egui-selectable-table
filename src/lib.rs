mod auto_reload;
mod auto_scroll;
mod row_modification;
mod row_selection;

use auto_reload::AutoReload;
pub use auto_scroll::AutoScroll;
use egui::ahash::{HashMap, HashMapExt, HashSet, HashSetExt};
use egui::{Event, Key, Label, Response, ScrollArea, Sense, Ui};
use egui_extras::{Column, TableBuilder, TableRow};
use std::cmp::Ordering;
use std::hash::Hash;

/// Enum representing the possible sort orders for table columns.
#[derive(Default, Clone, Copy)]
pub enum SortOrder {
    /// Sorts in ascending order (e.g., A to Z or 1 to 10).
    #[default]
    Ascending,
    /// Sorts in descending order (e.g., Z to A or 10 to 1).
    Descending,
}

/// Trait for defining how to order rows based on a specific column.
///
/// This trait should be implemented by users to specify how rows should be
/// compared for sorting purposes. The implementation can vary depending on
/// the type of column. For instance, string comparisons or numeric comparisons
/// can be handled differently depending on the column. Should only be implemented for Ascending
/// ordering, in case of Descending, it is handled internally.
///
/// # Example
/// Suppose you have a struct `MyRow` with fields like `user_id`, `name`, and `username`.
/// You could implement this trait for each column to specify how rows should be compared.
///
/// ```rust,ignore
/// impl ColumnOrdering<MyRow> for ColumnName {
///     fn order_by(&self, row_1: &MyRow, row_2: &MyRow) -> Ordering {
///         match self {
///             ColumnName::UserID => row_1.user_id.cmp(&row_2.user_id),
///             ColumnName::Name => row_1.name.cmp(&row_2.name),
///             ColumnName::Username => row_1.username.cmp(&row_2.username),
///         }
///     }
/// }
/// ```
pub trait ColumnOrdering<Row>
where
    Row: Clone + Send + Sync,
{
    /// Compare two rows and return the ordering result (`Ordering`).
    ///
    /// This function defines how to order two rows based on the specific column.
    /// It returns `Ordering::Less`, `Ordering::Equal`, or `Ordering::Greater`
    /// to indicate whether `row_1` should be placed before, after, or at the same
    /// position as `row_2` when sorting. Should only be implemented for ascending ordering, in
    /// case of Descending, it is handled internally.
    ///
    /// # Arguments
    /// * `row_1` - The first row for comparison.
    /// * `row_2` - The second row for comparison.
    ///
    /// # Returns
    /// * `Ordering` - Indicates the relative order between the two rows.
    fn order_by(&self, row_1: &Row, row_2: &Row) -> Ordering;
}

/// Trait for defining column-specific operations in a table UI.
///
/// This trait allows users to define how each column should behave within a table.
/// This includes how headers should be displayed, how each row in the table should be rendered,
/// and how to extract column-specific text.
///
/// # Type Parameters:
/// * `Row` - The type representing each row in the table.
/// * `F` - A type that identifies columns, usually an enum or a field type.
/// * `Conf` - Configuration type for the table, useful for passing additional settings.
///
/// # Requirements:
/// You must implement this trait to specify the behavior of each column within
/// the context of your table UI.
pub trait ColumnOperations<Row, F, Conf>
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
    /// Create the header UI for this column.
    ///
    /// This function is responsible for creating the visual representation of the column header.
    /// The `sort_order` argument indicates whether the column is currently used for sorting and, if so, in which
    /// direction (ascending or descending). You can customize the header appearance based on
    /// this information, for example by adding icons or text. Return `None` for no header.
    ///
    /// # Arguments
    /// * `ui` - A mutable reference to the UI context.
    /// * `sort_order` - An optional `SortOrder` representing the current sort state of the column.
    /// * `table` - A mutable reference to the `SelectableTable`, allowing you to interact with the table state.
    ///
    /// # Returns
    /// * `Option<Response>` - An optional response representing interaction with the UI.
    fn create_header(
        &self,
        ui: &mut Ui,
        sort_order: Option<SortOrder>,
        table: &mut SelectableTable<Row, F, Conf>,
    ) -> Option<Response>;

    /// Create the UI for a specific row in this column.
    ///
    /// This function is responsible for rendering the content of this column for a given row.
    /// It should handle user interactions like clicking or selection as necessary. Mutable table
    /// access is provided for modifyiing other rows as necessary.
    ///
    /// # Arguments
    /// * `ui` - A mutable reference to the UI context.
    /// * `row` - A reference to the current `SelectableRow` for this table.
    /// * `column_selected` - A boolean indicating whether this column is selected.
    /// * `table` - A mutable reference to the `SelectableTable` for modifying table data
    ///
    /// # Returns
    /// * `Response` - The result of the UI interaction for this row.
    fn create_table_row(
        &self,
        ui: &mut Ui,
        row: &SelectableRow<Row, F>,
        column_selected: bool,
        table: &mut SelectableTable<Row, F, Conf>,
    ) -> Response;

    /// Extract the text representation of the column for the given row.
    ///
    /// This function should return the appropriate text representation of this column
    /// for the given row. It can be used to display the data in a simplified form, such
    /// as for debugging or plain text rendering.
    ///
    /// # Arguments
    /// * `row` - A reference to the row from which to extract the column text.
    ///
    /// # Returns
    /// * `String` - The text representation of this column for the row.
    fn column_text(&self, row: &Row) -> String;
}

/// Represents a row in a table with selectable columns.
///
/// This struct is used to store the data of a row along with its unique identifier (`id`)
/// and the set of selected columns for this row.
///
/// # Type Parameters:
/// * `Row` - The type representing the data stored in each row.
/// * `F` - The type used to identify each column, typically an enum or a type with unique values.
///
/// # Fields:
/// * `row_data` - The actual data stored in the row.
/// * `id` - A unique identifier for the row.
/// * `selected_columns` - A set of columns that are selected in this row.
#[derive(Clone)]
pub struct SelectableRow<Row, F>
where
    Row: Clone + Send + Sync,
    F: Eq + Hash + Clone + Ord + Send + Sync + Default,
{
    pub row_data: Row,
    pub id: i64,
    pub selected_columns: HashSet<F>,
}

/// A table structure that hold data for performing selection on drag, sorting, and displaying rows and more.
///
/// # Type Parameters
/// * `Row` - The type representing each row in the table.
/// * `F` - A type used to identify columns, often an enum or field type.
/// * `Conf` - Configuration type for additional table settings passed by the user. This is made available anytime when creating or modifying rows
pub struct SelectableTable<Row, F, Conf>
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
    /// List of all columns available in the table.
    all_columns: Vec<F>,
    /// Maps each column to its index in the table for quick lookup.
    column_number: HashMap<F, usize>,
    /// Stores all rows in the table, keyed by their unique ID.
    rows: HashMap<i64, SelectableRow<Row, F>>,
    /// The current set of formatted rows for display.
    formatted_rows: Vec<SelectableRow<Row, F>>,
    /// The column currently being used to sort the table.
    sorted_by: F,
    /// The current sort order (ascending or descending).
    sort_order: SortOrder,
    /// Tracks where a drag operation started in the table, if any.
    drag_started_on: Option<(i64, F)>,
    /// The columns that have at least 1 row with the column as selected
    active_columns: HashSet<F>,
    /// The rows that have at least 1 column as selected
    active_rows: HashSet<i64>,
    /// The last row where the pointer was
    last_active_row: Option<i64>,
    /// The last column where the pointer was
    last_active_column: Option<F>,
    /// Whether the pointer moved from the dragged point at least once
    beyond_drag_point: bool,
    /// Map of the row IDs to the indices of `formatted_rows`
    indexed_ids: HashMap<i64, usize>,
    /// The last ID that was used for a new row in the table.
    last_id_used: i64,
    /// Handles auto scroll operation when dragging
    auto_scroll: AutoScroll,
    /// Handles auto recreating the displayed rows with the latest data
    auto_reload: AutoReload,
    /// Whether to select the entire row when dragging and selecting instead of a single cell
    select_full_row: bool,
    /// Whether to add a horizontal scrollbar
    horizontal_scroll: bool,
    /// Additional Parameters passed by you, available when creating new rows or header. Can
    /// contain anything implementing the `Default` trait
    pub config: Conf,
    /// Whether to add the row serial column to the table
    add_serial_column: bool,
    /// The row height for the table, defaults to 25.0
    row_height: f32,
}

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
    /// Creates a new `SelectableTable` with the provided columns in a specified order.
    ///
    /// # Parameters:
    /// - `columns`: A `Vec<F>` representing the columns. Columns must be passed in the correct order (e.g., 1 to 10).
    ///
    /// # Returns:
    /// - A new instance of `SelectableTable`.
    ///
    /// # Example:
    /// ```rust,ignore
    /// let table = SelectableTable::new(vec![col1, col2, col3]);
    /// ```
    #[must_use]
    pub fn new(columns: Vec<F>) -> Self {
        let all_columns = columns.clone();
        let mut column_number = HashMap::new();

        for (index, col) in columns.into_iter().enumerate() {
            column_number.insert(col, index);
        }
        Self {
            all_columns,
            column_number,
            last_id_used: 0,
            rows: HashMap::new(),
            formatted_rows: Vec::new(),
            sorted_by: F::default(),
            sort_order: SortOrder::default(),
            drag_started_on: None,
            active_columns: HashSet::new(),
            active_rows: HashSet::new(),
            last_active_row: None,
            last_active_column: None,
            beyond_drag_point: false,
            indexed_ids: HashMap::new(),
            auto_scroll: AutoScroll::default(),
            auto_reload: AutoReload::default(),
            select_full_row: false,
            horizontal_scroll: false,
            config: Conf::default(),
            add_serial_column: false,
            row_height: 25.0,
        }
    }

    /// Updates the table's configuration with the given `conf`.
    ///
    /// # Parameters:
    /// - `conf`: The new configuration of type `Conf`, which is user-defined and allows
    ///   passing data to help with row/table modification.
    ///
    /// # Example:
    /// ```rust,ignore
    /// table.set_config(my_config);
    /// ```
    pub fn set_config(&mut self, conf: Conf) {
        self.config = conf;
    }

    /// Sets a configuration in a builder-style pattern.
    ///
    /// # Parameters:
    /// - `conf`: A configuration of type `Conf`. The user can pass any data to help with row creation or modification.
    ///
    /// # Returns:
    /// - The updated `SelectableTable` with the new configuration applied.
    ///
    /// # Example:
    /// ```rust,ignore
    /// let table = SelectableTable::new(vec![col1, col2, col3]).config(my_config);
    /// ```
    #[must_use]
    pub fn config(mut self, conf: Conf) -> Self {
        self.config = conf;
        self
    }

    /// Clears all rows from the table, including the displayed ones
    ///
    /// # Example:
    /// ```rust,ignore
    /// table.clear_all_rows();
    /// ```
    pub fn clear_all_rows(&mut self) {
        self.rows.clear();
        self.formatted_rows.clear();
        self.active_rows.clear();
        self.active_columns.clear();
        self.last_id_used = 0;
    }

    /// Displays the UI for the table and uses the provided `TableBuilder` for creating the table UI.
    ///
    /// # Parameters:
    /// - `ui`: The UI context where the table will be rendered.
    /// - `table_builder`: A closure that receives and modifies the `TableBuilder`.
    ///
    /// # Example:
    /// ```rust,ignore
    /// table.show_ui(ui, |builder| builder.column(column1));
    /// ```
    pub fn show_ui<Fn>(&mut self, ui: &mut Ui, table_builder: Fn)
    where
        Fn: FnOnce(TableBuilder) -> TableBuilder,
    {
        let is_ctrl_pressed = ui.ctx().input(|i| i.modifiers.ctrl);
        let key_a_pressed = ui.ctx().input(|i| i.key_pressed(Key::A));
        let copy_initiated = ui.ctx().input(|i| i.events.contains(&Event::Copy));
        let ctx = ui.ctx().clone();

        if copy_initiated {
            self.copy_selected_cells(ui);
        }
        if is_ctrl_pressed && key_a_pressed {
            self.select_all();
        }

        let pointer = ui.input(|i| i.pointer.hover_pos());
        let max_rect = ui.max_rect();

        if self.horizontal_scroll {
            ScrollArea::horizontal().show(ui, |ui| {
                let mut table = TableBuilder::new(ui);

                if self.add_serial_column {
                    table = table.column(Column::initial(25.0).clip(true));
                }

                table = table_builder(table);

                if self.drag_started_on.is_some() {
                    if let Some(offset) = self.auto_scroll.start_scroll(max_rect, pointer) {
                        table = table.vertical_scroll_offset(offset);
                        ctx.request_repaint();
                    }
                }

                let output = table
                    .header(20.0, |header| {
                        self.build_head(header);
                    })
                    .body(|body| {
                        body.rows(self.row_height, self.formatted_rows.len(), |row| {
                            let index = row.index();
                            self.build_body(row, index);
                        });
                    });
                let scroll_offset = output.state.offset.y;
                self.update_scroll_offset(scroll_offset);
            });
        } else {
            let mut table = TableBuilder::new(ui);

            if self.add_serial_column {
                table = table.column(Column::initial(25.0).clip(true));
            }

            table = table_builder(table);

            if self.drag_started_on.is_some() {
                if let Some(offset) = self.auto_scroll.start_scroll(max_rect, pointer) {
                    table = table.vertical_scroll_offset(offset);
                    ctx.request_repaint();
                }
            }

            let output = table
                .header(20.0, |header| {
                    self.build_head(header);
                })
                .body(|body| {
                    body.rows(self.row_height, self.formatted_rows.len(), |row| {
                        let index = row.index();
                        self.build_body(row, index);
                    });
                });
            let scroll_offset = output.state.offset.y;
            self.update_scroll_offset(scroll_offset);
        }
    }

    fn build_head(&mut self, mut header: TableRow) {
        if self.add_serial_column {
            header.col(|ui| {
                ui.add_sized(ui.available_size(), Label::new(""));
            });
        }
        for column_name in &self.all_columns.clone() {
            header.col(|ui| {
                let sort_order = if &self.sorted_by == column_name {
                    Some(self.sort_order)
                } else {
                    None
                };

                let Some(resp) = column_name.create_header(ui, sort_order, self) else {
                    return;
                };

                // Response click sense is not forced. So if a header should not be used
                // for sorting, without click there won't be any actions.

                if resp.clicked() {
                    let is_selected = &self.sorted_by == column_name;
                    if is_selected {
                        self.change_sort_order();
                    } else {
                        self.change_sorted_by(column_name);
                    }
                    self.recreate_rows();
                }
            });
        }
    }

    fn build_body(&mut self, mut row: TableRow, index: usize) {
        let row_data = self.formatted_rows[index].clone();

        if self.add_serial_column {
            row.col(|ui| {
                ui.add_sized(ui.available_size(), Label::new(format!("{}", index + 1)));
            });
        }
        self.handle_table_body(row, &row_data);
    }

    /// Change the current sort order from ascending to descending and vice versa. Will unselect
    /// all selected rows
    fn change_sort_order(&mut self) {
        self.unselect_all();
        if matches!(self.sort_order, SortOrder::Ascending) {
            self.sort_order = SortOrder::Descending;
        } else {
            self.sort_order = SortOrder::Ascending;
        }
    }

    /// Change the column that is currently being used for sorting. Will unselect all rows
    fn change_sorted_by(&mut self, sort_by: &F) {
        self.unselect_all();
        self.sorted_by = sort_by.clone();
        self.sort_order = SortOrder::default();
    }

    /// Recreates the rows shown in the UI for the next frame load.
    ///
    /// # Important:
    /// - Any direct modifications made using [`modify_shown_row`](#method.modify_shown_row)
    ///   will be **cleared** when this is called.
    ///   To preserve changes, use [`add_modify_row`](#method.add_modify_row) to update row data instead.
    ///
    /// # Performance:
    /// - Should be used sparingly for large datasets, as frequent calls can lead to performance issues.
    /// - Consider calling after every X number of row updates, depending on update frequency,
    ///   or use [`auto_reload`](#method.auto_reload) for automatic reload.
    ///
    /// # Example:
    /// ```rust,ignore
    /// table.recreate_rows();
    /// ```
    pub fn recreate_rows(&mut self) {
        self.formatted_rows.clear();
        self.active_rows.clear();
        self.active_columns.clear();
        self.sort_rows();
    }

    /// Recreates the rows shown in the UI for the next frame load.
    ///
    /// This function refreshes the internal row state by clearing and re-sorting the rows
    /// similar to [`recreate_rows`](#method.recreate_rows), but it **preserves** the currently
    /// selected rows and re-applies the active column selection to them.
    ///
    /// Useful when the UI needs to be refreshed without resetting user interaction state.
    ///
    /// # Important:
    /// - Any direct modifications made to `formatted_rows` using [`modify_shown_row`](#method.modify_shown_row)
    ///   will be **cleared** when this is called.
    ///   To preserve changes, use [`add_modify_row`](#method.add_modify_row) to update row data instead.
    ///
    /// # Performance:
    /// - Should be used sparingly for large datasets, as frequent calls can lead to performance issues.
    /// - Consider calling after every X number of row updates, depending on update frequency,
    ///   or use [`auto_reload`](#method.auto_reload) for automatic reload.
    ///
    /// # Example:
    /// ```rust,ignore
    /// table.recreate_rows_no_unselect();
    /// ```
    pub fn recreate_rows_no_unselect(&mut self) {
        self.formatted_rows.clear();
        self.sort_rows();

        for row in &self.active_rows {
            let Some(target_index) = self.indexed_ids.get(row) else {
                continue;
            };
            self.formatted_rows[*target_index]
                .selected_columns
                .clone_from(&self.active_columns);
        }
    }

    /// The first column that was passed by the user
    fn first_column(&self) -> F {
        self.all_columns[0].clone()
    }

    /// The last column that was passed by the user
    fn last_column(&self) -> F {
        self.all_columns[self.all_columns.len() - 1].clone()
    }

    /// Convert a number to a column value
    fn column_to_num(&self, column: &F) -> usize {
        *self
            .column_number
            .get(column)
            .expect("Not in the column list")
    }

    /// Get the next column of the provided column
    fn next_column(&self, column: &F) -> F {
        let current_column_num = self.column_to_num(column);
        if current_column_num == self.all_columns.len() - 1 {
            self.all_columns[0].clone()
        } else {
            self.all_columns[current_column_num + 1].clone()
        }
    }

    /// Get the previous column of the provided column
    fn previous_column(&self, column: &F) -> F {
        let current_column_num = self.column_to_num(column);
        if current_column_num == 0 {
            self.all_columns[self.all_columns.len() - 1].clone()
        } else {
            self.all_columns[current_column_num - 1].clone()
        }
    }

    /// Builds the table's Body section
    fn handle_table_body(&mut self, mut row: TableRow, row_data: &SelectableRow<Row, F>) {
        for column_name in &self.all_columns.clone() {
            row.col(|ui| {
                let selected = row_data.selected_columns.contains(column_name);
                let mut resp = column_name.create_table_row(ui, row_data, selected, self);

                // Drag sense is forced otherwise there is no point of this library.
                resp = resp.interact(Sense::drag());

                if resp.drag_started() {
                    // If CTRL is not pressed down and the mouse right click is not pressed, unselect all cells
                    // Right click for context menu
                    if !ui.ctx().input(|i| i.modifiers.ctrl)
                        && !ui.ctx().input(|i| i.pointer.secondary_clicked())
                    {
                        self.unselect_all();
                    }
                    self.drag_started_on = Some((row_data.id, column_name.clone()));
                }

                let pointer_released = ui.input(|a| a.pointer.primary_released());

                if pointer_released {
                    self.last_active_row = None;
                    self.last_active_column = None;
                    self.drag_started_on = None;
                    self.beyond_drag_point = false;
                }

                if resp.clicked() {
                    // If CTRL is not pressed down and the mouse right click is not pressed, unselect all cells
                    if !ui.ctx().input(|i| i.modifiers.ctrl)
                        && !ui.ctx().input(|i| i.pointer.secondary_clicked())
                    {
                        self.unselect_all();
                    }
                    self.select_single_row_cell(row_data.id, column_name);
                }

                if ui.ui_contains_pointer() && self.drag_started_on.is_some() {
                    if let Some(drag_start) = self.drag_started_on.as_ref() {
                        // Only call drag either when not on the starting drag row/column or went beyond the
                        // drag point at least once. Otherwise normal click would be considered as drag
                        if drag_start.0 != row_data.id
                            || &drag_start.1 != column_name
                            || self.beyond_drag_point
                        {
                            let is_ctrl_pressed = ui.ctx().input(|i| i.modifiers.ctrl);
                            self.select_dragged_row_cell(row_data.id, column_name, is_ctrl_pressed);
                        }
                    }
                }
            });
        }
    }

    /// Returns the total number of rows currently being displayed in the UI.
    ///
    /// # Returns:
    /// - `usize`: The number of rows that are formatted and ready for display.
    pub const fn total_displayed_rows(&self) -> usize {
        self.formatted_rows.len()
    }

    /// Returns the total number of rows in the table (both displayed and non-displayed).
    ///
    /// # Returns:
    /// - `usize`: The total number of rows stored in the table, regardless of whether they are being displayed or not.
    pub fn total_rows(&self) -> usize {
        self.rows.len()
    }

    /// Provides a reference to the rows currently being displayed in the UI.
    ///
    /// # Returns:
    /// - `&Vec<SelectableRow<Row, F>>`: A reference to the vector of formatted rows ready for display.
    pub const fn get_displayed_rows(&self) -> &Vec<SelectableRow<Row, F>> {
        &self.formatted_rows
    }

    /// Provides a reference to all rows in the table, regardless of whether they are displayed.
    ///
    /// # Returns:
    /// - `&HashMap<i64, SelectableRow<Row, F>>`: A reference to the entire collection of rows in the table.
    pub const fn get_all_rows(&self) -> &HashMap<i64, SelectableRow<Row, F>> {
        &self.rows
    }

    /// Adds a serial column to the table.
    ///
    /// The serial column is automatically generated and displayed at the very left of the table.
    /// It shows the row number (starting from 1) for each row.
    ///
    /// # Returns:
    /// - `Self`: The modified table with the serial column enabled.
    ///
    /// # Example:
    /// ```rust,ignore
    /// let table = SelectableTable::new(vec![col1, col2, col3])
    ///     .config(my_config).serial_column();
    /// ```
    #[must_use]
    pub const fn serial_column(mut self) -> Self {
        self.add_serial_column = true;
        self
    }

    /// Add a horizontal scrollbar to the table
    ///
    /// # Returns:
    /// - `Self`: The modified table with the serial column enabled.
    ///
    /// # Example:
    /// ```rust,ignore
    /// let table = SelectableTable::new(vec![col1, col2, col3])
    ///     .horizontal_scroll();
    /// ```
    #[must_use]
    pub const fn horizontal_scroll(mut self) -> Self {
        self.horizontal_scroll = true;
        self
    }

    /// Sets the height rows in the table.
    ///
    /// # Parameters:
    /// - `height`: The desired height for each row in logical points.
    ///
    /// # Returns:
    /// - `Self`: The modified table with the specified row height applied.
    ///
    /// # Example:
    /// ```rust,ignore
    /// let table = SelectableTable::new(vec![col1, col2, col3])
    ///     .row_height(24.0);
    /// ```
    #[must_use]
    pub const fn row_height(mut self, height: f32) -> Self {
        self.row_height = height;
        self
    }
}
