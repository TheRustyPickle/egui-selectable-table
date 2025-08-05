# egui-selectable-table

<a href="https://crates.io/crates/egui-selectable-table"><img src="https://img.shields.io/crates/v/egui-selectable-table.svg?style=flat-square&logo=rust&color=orange" alt="Crates version"/></a>
<a href="https://crates.io/crates/egui-selectable-table"><img src="https://img.shields.io/crates/d/egui-selectable-table?style=flat-square" alt="Downloads"/></a>
[![Docs](https://img.shields.io/docsrs/egui-selectable-table/latest)](https://docs.rs/egui-selectable-table)

A library for [egui](https://github.com/emilk/egui) to create tables with draggable cell and row selection.

[](https://github.com/user-attachments/assets/54aadfbf-e795-4948-933b-68c08dce6242)

## Features

- Individual cell or full-row selection while dragging
- Auto vertical table scrolling during drag with adjustable parameters
- Sort rows by clicking headers, both ascending and descending
- Customizable rows and header UI
- Built-in select all (Ctrl+A) and copy (Ctrl+C) functionality
- Capable of handling a substantial amount of rows (1M+) with proper settings
- Optional fuzzy matching for searching rows (see below)

## Optional Features

This crate includes an optional `fuzzy-matching` feature that enables fuzzy row search using the [nucleo-matcher](https://crates.io/crates/nucleo-matcher) crate.

To enable fuzzy matching:

```toml
[dependencies]
egui-selectable-table = { version = "0.4.0", features = ["fuzzy-matching"] }
```

## Usage

```rust
// See Demo folder for a complete example

use egui_selectable_table::{
    ColumnOperations, ColumnOrdering, SelectableRow, SelectableTable, SortOrder,
};
// other use imports

struct Config {
// anything you want to pass
}

struct MyRow {
  field_1: String,
// .. more fields
}
enum Column {
  Field1,
// .. more column names
}

// Implement both traits for row and column
impl ColumnOperations<MyRow, ColumnName, Config> for Column {
    // The text of a row based on the column
    fn column_text(&self, row: &MyRow) -> String {}
    // Create your own header or no header
    fn create_header(&self, ui: &mut Ui, sort_order: Option<SortOrder>, table: &mut SelectableTable<MyRow, Column, Config>) -> Option<Response> {}
    //Create your own table row UI
    fn create_table_row(&self, ui: &mut Ui, row: &SelectableRow<MyRow, Column>, selected: bool, table: &mut SelectableTable<MyRow, Column, Config>,) -> Response {}
}

impl ColumnOrdering<MyRow> for Column {
    fn order_by(&self, row_1: &MyRow, row_2: &MyRow) -> std::cmp::Ordering {
        match self {
            Column::Field1 => row_1.field_1.cmp(&row_2.field_1),
        }
    }
}

pub struct MainWindow {
    table: SelectableTable<MyRow, Column, Config>
}

impl MainWindow {
    pub fn new() -> Self {
        Self {
            table: SelectableTable::new(vec![Column::Field1])
        }
    }
}

impl eframe::App for MainWindow {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            self.table.show_ui(ui |table| {
              table.striped(true)
                .cell_layout(Layout::left_to_right(Align::Center))
                .column(Column::exact(column_size).clip(true))
            })
        });
    }
}

```

## Run Demo

The demo is accessible online via [this link](https://therustypickle.github.io/egui-selectable-table/)

- Clone the repository `git clone https://github.com/TheRustyPickle/egui-selectable-table`
- Move into the demo folder `cd egui-selectable-table/demo`

  - To run natively `cargo run --release`

  or

  - To run in wasm locally install the required target with `rustup target add wasm32-unknown-unknown`
  - Install Trunk with `cargo install --locked trunk`
  - `trunk serve` to run and visit `http://127.0.0.1:8080/`

## Contributing

Contributions, issues, and feature requests are welcome! If you'd like to contribute, please open a pull request.

## License

This project is licensed under the MIT License.
