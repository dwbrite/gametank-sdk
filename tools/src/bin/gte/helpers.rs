
pub fn get_now_ms() -> f64 {
    #[cfg(target_arch = "wasm32")]
    {
        let window = web_sys::window().expect("should have a window in this context");
        let performance = window
            .performance()
            .expect("performance should be available");

        return performance.now();
    }

    #[cfg(not(target_arch = "wasm32"))]
    unsafe {
        use std::time::Instant;
        static mut START_INSTANT: Option<Instant> = None;

        if START_INSTANT.is_none() {
            START_INSTANT = Some(Instant::now());
        }
        return START_INSTANT.unwrap().elapsed().as_secs_f64() * 1000.0;
    }
}
