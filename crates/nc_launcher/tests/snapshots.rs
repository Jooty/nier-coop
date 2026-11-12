use egui_kittest::Harness;

#[test]
fn trivial_ui_renders_off_screen() {
    let mut harness = Harness::builder()
        .with_size(eframe::egui::vec2(300.0, 100.0))
        .renderer(egui_kittest::wgpu::WgpuTestRenderer::new())
        .build_ui(|ui| {
            ui.label("hello");
        });
    harness.run();
    let image = harness.render().expect("render");
    std::fs::create_dir_all("../../target/launcher-snapshots").unwrap();
    image
        .save("../../target/launcher-snapshots/trivial.png")
        .unwrap();
}
