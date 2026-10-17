use eframe::egui;

struct Spike;

impl eframe::App for Spike {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.heading("tsumiba spike こんにちは");
        ui.label("hello world");
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([640.0, 240.0]),
        ..Default::default()
    };
    eframe::run_native("tsumiba", options, Box::new(|_cc| Ok(Box::new(Spike))))
}
