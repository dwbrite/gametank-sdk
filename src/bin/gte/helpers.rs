pub fn get_now_ms() -> f64 {
    #[cfg(target_arch = "wasm32")]
    {
        let window = web_sys::window().expect("should have a window in this context");
        let performance = window
            .performance()
            .expect("performance should be available");

        performance.now()
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::sync::OnceLock;
        use std::time::Instant;

        static START_INSTANT: OnceLock<Instant> = OnceLock::new();

        START_INSTANT.get_or_init(Instant::now).elapsed().as_secs_f64() * 1000.0
    }
}
