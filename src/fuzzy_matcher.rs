use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Matcher, Utf32Str};
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
    /// Performs a fuzzy search using specified columns across all rows and updates the displayed rows.
    ///
    /// This function filters the table rows based on a search `query` using `nucleo-matcher`
    /// crate.
    /// It checks the specified `column_list` for each row, concatenates their string representations,
    /// and scores them using the provided or generated `Pattern`. Only rows with a non-`None` score
    /// are retained.
    ///
    /// If a `limit` is provided, it will result at most `limit` rows.
    ///
    /// # Parameters:
    /// - `column_list`: A list of columns to search across. Does nothing if empty.
    /// - `query`: The search string. Does nothing if empty.
    /// - `limit`: Optional limit on the number of results returned. Does nothing if `0`. Defaults
    ///   to no limit
    /// - `pattern`: Optional precomputed fuzzy `Pattern`. Default pattern is created from the query using
    ///   case-insensitive matching and smart normalization.
    ///
    /// The search is relatively fast even with a million rows but it should not be called every
    /// frame and be used sparingly.
    ///
    /// To reset search results, call [`recreate_rows`](SelectableTable::recreate_rows).
    ///
    /// # Example:
    /// ```rust,ignore
    /// table.search_and_show(&vec![Column::Name, Column::Username], "john", Some(10), None);
    /// ```
    pub fn search_and_show(
        &mut self,
        column_list: &Vec<F>,
        query: &str,
        limit: Option<usize>,
        pattern: Option<Pattern>,
    ) {
        if query.is_empty() {
            return;
        }

        if column_list.is_empty() {
            return;
        }

        if let Some(limit) = limit
            && limit == 0
        {
            return;
        }

        let pattern = pattern.map_or_else(
            || Pattern::parse(query, CaseMatching::Ignore, Normalization::Smart),
            |pattern| pattern,
        );

        let mut buf = Vec::new();
        let mut row_data: Vec<SelectableRow<Row, F>> = Vec::new();

        for val in self.rows.values() {
            let mut string_val = String::new();

            for column in column_list {
                let value = column.column_text(&val.row_data);
                string_val.push_str(&value);
                string_val.push(' ');
            }

            if pattern
                .score(Utf32Str::new(&string_val, &mut buf), &mut self.matcher)
                .is_some()
            {
                row_data.push(val.clone());

                if let Some(max) = limit {
                    if row_data.len() >= max {
                        break;
                    }
                }
            }
        }

        self.formatted_rows.clear();
        self.active_rows.clear();
        self.active_columns.clear();

        row_data.par_sort_by(|a, b| {
            let ordering = self.sorted_by.order_by(&a.row_data, &b.row_data);
            match self.sort_order {
                SortOrder::Ascending => ordering,
                SortOrder::Descending => ordering.reverse(),
            }
        });

        self.indexed_ids = row_data
            .par_iter()
            .enumerate()
            .map(|(index, row)| (row.id, index))
            .collect();

        self.formatted_rows = row_data;
    }

    /// Sets a custom matcher to use for fuzzy searching rows
    ///
    /// This allows the table to use a custom `Matcher` from `nucleo-matcher` crate
    /// for searching/filtering through rows based on the input text. Use this to change
    /// the search algorithm or tweak scoring behavior.
    ///
    /// # Parameters:
    /// - `matcher`: The matcher instance to use for row filtering.
    ///
    /// # Returns:
    /// - `Self`: The modified table with the specified matcher applied.
    ///
    /// # Example:
    /// ```rust,ignore
    /// let matcher = Matcher::default();
    /// let table = SelectableTable::new(columns)
    ///     .matcher(matcher);
    /// ```
    #[must_use]
    pub fn matcher(mut self, matcher: Matcher) -> Self {
        self.matcher = matcher;
        self
    }

    /// Replaces the current matcher with a new one.
    ///
    /// This method allows updating the fuzzy search matcher dynamically.
    ///
    /// # Parameters:
    /// - `matcher`: The new matcher instance to set.
    ///
    /// # Example:
    /// ```rust,ignore
    /// table.set_matcher(new_matcher);
    /// ```
    pub fn set_matcher(&mut self, matcher: Matcher) {
        self.matcher = matcher;
    }
}
