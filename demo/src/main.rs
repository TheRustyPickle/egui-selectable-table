mod app;

use app::MainWindow;

#[cfg(target_arch = "wasm32")]
use eframe::{WebOptions, WebRunner};

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    use egui::{ViewportBuilder, vec2};

    let options = eframe::NativeOptions {
        centered: true,
        persist_window: false,
        viewport: ViewportBuilder {
            inner_size: Some(vec2(1200.0, 700.0)),
            ..Default::default()
        },
        ..Default::default()
    };

    eframe::run_native(
        "Selectable Table Demo",
        options,
        Box::new(|cc| Ok(Box::new(MainWindow::new(cc)))),
    )
}

#[cfg(target_arch = "wasm32")]
fn main() {
    use eframe::wasm_bindgen::JsCast as _;
    let web_options = WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window()
            .expect("No window")
            .document()
            .expect("No document");

        let canvas = document
            .get_element_by_id("the_canvas_id")
            .expect("Failed to find the_canvas_id")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("the_canvas_id was not a HtmlCanvasElement");

        let start_result = WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cc| Ok(Box::new(MainWindow::new(cc)))),
            )
            .await;

        // Remove the loading text and spinner:
        if let Some(loading_text) = document.get_element_by_id("loading_text") {
            match start_result {
                Ok(_) => {
                    loading_text.remove();
                }
                Err(e) => {
                    loading_text.set_inner_html(
                        "<p> The app has crashed. See the developer console for details. </p>",
                    );
                    panic!("Failed to start eframe: {e:?}");
                }
            }
        }
    });
}
