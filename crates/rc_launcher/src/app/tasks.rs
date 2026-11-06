use std::sync::mpsc;

pub struct Task<T> {
    receiver: mpsc::Receiver<T>,
}

impl<T: Send + 'static> Task<T> {
    pub fn spawn(
        work: impl FnOnce() -> T + Send + 'static,
        repaint: eframe::egui::Context,
    ) -> Self {
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = sender.send(work());
            repaint.request_repaint();
        });
        Self { receiver }
    }

    pub fn try_take(&self) -> Option<T> {
        self.receiver.try_recv().ok()
    }
}
