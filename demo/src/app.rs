use eframe::{App, CreationContext, Frame};
use egui::ahash::{HashSet, HashSetExt};
use egui::{
    Align, Button, CentralPanel, Context, Layout, Slider, TextEdit, ThemePreference, Ui, Visuals,
};
use egui_extras::Column;
use egui_selectable_table::{
    AutoScroll, ColumnOperations, ColumnOrdering, SelectableRow, SelectableTable, SortOrder,
};
use egui_theme_lerp::ThemeAnimator;
use strum::IntoEnumIterator;
use strum_macros::{Display, EnumIter};

#[derive(Default, Clone, Copy)]
pub struct Config {
    counting_ongoing: bool,
}

pub struct MainWindow {
    search_text: String,
    select_entire_row: bool,
    add_rows: bool,
    auto_scrolling: bool,
    row_to_add: u64,
    row_num: u64,
    row_count: u64,
    scroll_speed: f32,
    reload_counter: u32,
    table: SelectableTable<TableRow, TableColumns, Config>,
    conf: Config,
    theme_animator: ThemeAnimator,
    search_column_list: HashSet<TableColumns>,
}

impl MainWindow {
    pub fn new(cc: &CreationContext) -> Self {
        cc.egui_ctx
            .options_mut(|a| a.theme_preference = ThemePreference::Light);

        let all_columns = TableColumns::iter().collect();

        // Auto reload after each 10k table row add or modification
        let table = SelectableTable::new(all_columns)
            .auto_reload(10_000)
            .auto_scroll()
            .horizontal_scroll()
            .no_ctrl_a_capture();

        MainWindow {
            search_text: String::new(),
            select_entire_row: false,
            add_rows: false,
            auto_scrolling: true,
            row_to_add: 0,
            row_num: 0,
            row_count: 0,
            scroll_speed: 30.0,
            reload_counter: 0,
            table,
            conf: Config::default(),
            theme_animator: ThemeAnimator::new(Visuals::light(), Visuals::dark()),
            search_column_list: HashSet::new(),
        }
    }
}

impl App for MainWindow {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        let theme_emoji = if !self.theme_animator.animation_done {
            if self.theme_animator.theme_1_to_2 {
                "☀"
            } else {
                "🌙"
            }
        } else if self.theme_animator.theme_1_to_2 {
            "🌙"
        } else {
            "☀"
        };

        CentralPanel::default().show(ctx, |ui| {
            if self.theme_animator.anim_id.is_none() {
                self.theme_animator.create_id(ui);
            } else {
                self.theme_animator.animate(ctx);
            }

            ui.horizontal(|ui| {
                if ui.button(theme_emoji).clicked() {
                    self.theme_animator.start();
                }

                ui.separator();
                ui.label("Total Rows to add:");
                ui.add(Slider::new(&mut self.row_count, 10000..=1_000_000));

                let button_enabled = !self.add_rows;
                let button = ui.add_enabled(button_enabled, Button::new("Create Rows"));
                if button.clicked() {
                    self.add_rows = true;
                    self.row_to_add = self.row_count;

                    // Clear previously added rows
                    self.table.clear_all_rows();
                    self.table.set_auto_reload(Some(self.reload_counter));
                    self.conf.counting_ongoing = true;
                };
                ui.separator();
                if ui
                    .checkbox(&mut self.select_entire_row, "Select Entire Row?")
                    .changed()
                {
                    self.table.set_select_full_row(self.select_entire_row);
                };
                ui.separator();
                if ui.button("Add unsorted row at the bottom").clicked() {
                    let new_row = TableRow {
                        field_1: self.row_num,
                        field_2: self.row_num * 3,
                        field_3: self.row_num * 8,
                        field_4: self.row_num * 11,
                        field_5: self.row_num * 17,
                        field_6: self.row_num * 21,
                        create_count: 0,
                    };
                    self.row_num += 1;
                    self.table.add_unsorted_row(new_row);
                }
            });
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("Auto scrolling speed:");
                if ui
                    .add(Slider::new(&mut self.scroll_speed, 10.0..=100.0))
                    .changed()
                {
                    let scroll = AutoScroll::new(self.auto_scrolling).max_speed(self.scroll_speed);
                    self.table.update_auto_scroll(scroll);
                };
                ui.separator();
                if ui
                    .checkbox(&mut self.auto_scrolling, "Enable Auto Scrolling on drag?")
                    .changed()
                {
                    let scroll = AutoScroll::new(self.auto_scrolling);
                    self.table.update_auto_scroll(scroll);
                }
            });
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("Row Recreation Counter:");
                ui.add(Slider::new(&mut self.reload_counter, 10000..=1000000));
                ui.label("Higher value = Less often the UI is refreshed")
            });
            ui.separator();

            if self.row_count * 10 / 100 > self.reload_counter as u64 {
                ui.horizontal(|ui| {
                    ui.label(
                        "⚠  Row count too high. Increase recreation counter to prevent ui freeze",
                    );
                });
                ui.separator();
            }

            ui.horizontal(|ui| {
                let text_edit =
                    TextEdit::singleline(&mut self.search_text).hint_text("Search for rows");

                ui.add(text_edit);

                if ui.button("Search").clicked() {
                    let column_list: Vec<TableColumns> =
                        self.search_column_list.clone().into_iter().collect();

                    self.table
                        .search_and_show(&column_list, &self.search_text, None, None);
                };

                if ui.button("Clear").clicked() {
                    self.search_text.clear();
                    self.table.recreate_rows();
                };

                ui.separator();

                let all_columns: Vec<TableColumns> = TableColumns::iter().collect();
                for col in all_columns.into_iter() {
                    if ui
                        .selectable_label(self.search_column_list.contains(&col), col.to_string())
                        .clicked()
                    {
                        if self.search_column_list.contains(&col) {
                            self.search_column_list.remove(&col);
                        } else {
                            self.search_column_list.insert(col);
                        }
                    };
                }
            });

            ui.separator();

            self.table.show_ui(ui, |table| {
                let mut table = table
                    .drag_to_scroll(false)
                    .striped(true)
                    .resizable(true)
                    .cell_layout(Layout::left_to_right(Align::Center))
                    .drag_to_scroll(false)
                    .auto_shrink([false; 2])
                    .min_scrolled_height(0.0);

                for _col in TableColumns::iter() {
                    table = table.column(Column::initial(150.0))
                }
                table
            });
            self.table.set_config(self.conf);

            if self.add_rows {
                for _num in 0..10000 {
                    self.table.add_modify_row(|_| {
                        let new_row = TableRow {
                            field_1: self.row_num,
                            field_2: self.row_num * 3,
                            field_3: self.row_num * 8,
                            field_4: self.row_num * 11,
                            field_5: self.row_num * 17,
                            field_6: self.row_num * 21,
                            create_count: 0,
                        };
                        Some(new_row)
                    });
                    self.row_num += 1;
                    if self.row_num > self.row_to_add {
                        self.add_rows = false;
                        self.row_to_add = 0;
                        self.row_num = 0;
                        // forcefully reload the table as there are no more rows coming
                        self.table.recreate_rows();
                        self.conf.counting_ongoing = false;
                        self.table.set_auto_reload(None);

                        break;
                    }
                }
                // Ensure it does not wait for an event on the app to load the new rows
                ctx.request_repaint();
            }
        });
    }
}

#[derive(Clone, Default)]
struct TableRow {
    field_1: u64,
    field_2: u64,
    field_3: u64,
    field_4: u64,
    field_5: u64,
    field_6: u64,
    create_count: u64,
}

#[derive(Eq, PartialEq, Debug, Ord, PartialOrd, Clone, Copy, Hash, Default, EnumIter, Display)]
enum TableColumns {
    #[default]
    #[strum(to_string = "Column 1")]
    Field1,
    #[strum(to_string = "Column 2")]
    Field2,
    #[strum(to_string = "Column 3")]
    Field3,
    #[strum(to_string = "Column 4")]
    Field4,
    #[strum(to_string = "Column 5")]
    Field5,
    #[strum(to_string = "Column 6")]
    Field6,
    #[strum(to_string = "Render Counter")]
    Field7,
}

impl ColumnOperations<TableRow, TableColumns, Config> for TableColumns {
    fn column_text(&self, row: &TableRow) -> String {
        match self {
            TableColumns::Field1 => row.field_1.to_string(),
            TableColumns::Field2 => row.field_2.to_string(),
            TableColumns::Field3 => row.field_3.to_string(),
            TableColumns::Field4 => row.field_4.to_string(),
            TableColumns::Field5 => row.field_5.to_string(),
            TableColumns::Field6 => row.field_6.to_string(),
            TableColumns::Field7 => row.create_count.to_string(),
        }
    }
    fn create_header(
        &self,
        ui: &mut Ui,
        sort_order: Option<SortOrder>,
        _table: &mut SelectableTable<TableRow, TableColumns, Config>,
    ) -> Option<egui::Response> {
        let mut text = self.to_string();

        if let Some(sort) = sort_order {
            match sort {
                SortOrder::Ascending => text += "🔽",
                SortOrder::Descending => text += "🔼",
            }
        }
        let selected = sort_order.is_some();
        let resp = ui.add_sized(ui.available_size(), Button::selectable(selected, text));
        Some(resp)
    }
    fn create_table_row(
        &self,
        ui: &mut Ui,
        row: &SelectableRow<TableRow, TableColumns>,
        cell_selected: bool,
        table: &mut SelectableTable<TableRow, TableColumns, Config>,
    ) -> egui::Response {
        let row_id = row.id;
        let row_data = &row.row_data;
        let config = table.config;

        let text = match self {
            TableColumns::Field1 => row_data.field_1.to_string(),
            TableColumns::Field2 => row_data.field_2.to_string(),
            TableColumns::Field3 => row_data.field_3.to_string(),
            TableColumns::Field4 => row_data.field_4.to_string(),
            TableColumns::Field5 => row_data.field_5.to_string(),
            TableColumns::Field6 => row_data.field_6.to_string(),
            TableColumns::Field7 => row_data.create_count.to_string(),
        };

        // Persist the creation count, while row creation is ongoing, this will get auto
        // reloaded. After there is no more row creation, auto reload is turned off and won't
        // reload until next manual intervention. While no more rows are being created, we are
        // modifying the rows directly that are being shown in the UI which is much less
        // expensive and gets shown to the UI immediately.
        // Continue to update the persistent row data to ensure once reload happens, the
        // previous count data is not lost
        table.add_modify_row(|table| {
            let target_row = table.get_mut(&row_id).unwrap();
            target_row.row_data.create_count += 1;
            None
        });
        if !config.counting_ongoing {
            table.modify_shown_row(|t, index| {
                let target_index = index.get(&row_id).unwrap();
                let target_row = t.get_mut(*target_index).unwrap();
                target_row.row_data.create_count += 1;
            });
        }

        // The same approach works for both cell based selection and for entire row selection on
        // drag.
        let resp = ui.add_sized(ui.available_size(), Button::selectable(cell_selected, text));

        resp.context_menu(|ui| {
            if ui.button("Select All Rows").clicked() {
                table.select_all();
                ui.close();
            }
            if ui.button("Unselect All Rows").clicked() {
                table.unselect_all();
                ui.close();
            }
            if ui.button("Copy Selected Cells").clicked() {
                table.copy_selected_cells(ui);
                ui.close();
            }
            if ui.button("Mark row as selected").clicked() {
                ui.close();
                table.mark_row_as_selected(row_id, None);
            }
        });
        resp
    }
}

impl ColumnOrdering<TableRow> for TableColumns {
    fn order_by(&self, row_1: &TableRow, row_2: &TableRow) -> std::cmp::Ordering {
        match self {
            TableColumns::Field1 => row_1.field_1.cmp(&row_2.field_1),
            TableColumns::Field2 => row_1.field_2.cmp(&row_2.field_2),
            TableColumns::Field3 => row_1.field_3.cmp(&row_2.field_3),
            TableColumns::Field4 => row_1.field_4.cmp(&row_2.field_4),
            TableColumns::Field5 => row_1.field_5.cmp(&row_2.field_5),
            TableColumns::Field6 => row_1.field_6.cmp(&row_2.field_6),
            TableColumns::Field7 => row_1.create_count.cmp(&row_2.create_count),
        }
    }
}
