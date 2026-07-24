//! Wall-clock shim: `std::time::Instant::now()` PANICS on
//! `wasm32-unknown-unknown` (std's unsupported time backend), and every
//! engine timing read is measurement/reporting — never behavior. On wasm
//! the clock freezes at zero instead of panicking, which is exactly what
//! a browser think should report anyway (the in-page demo found this the
//! hard way: any solve reaching the best-first fallback died at
//! `search_from`'s phase-attribution timer).

/// A monotonic timestamp that is a no-op on wasm.
#[derive(Clone, Copy)]
pub struct Clock {
    #[cfg(not(target_arch = "wasm32"))]
    t0: std::time::Instant,
}

impl Clock {
    #[inline]
    pub fn now() -> Self {
        Clock {
            #[cfg(not(target_arch = "wasm32"))]
            t0: std::time::Instant::now(),
        }
    }

    #[inline]
    pub fn elapsed_ms(&self) -> u128 {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.t0.elapsed().as_millis()
        }
        #[cfg(target_arch = "wasm32")]
        {
            0
        }
    }

    #[inline]
    pub fn elapsed_us(&self) -> u128 {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.t0.elapsed().as_micros()
        }
        #[cfg(target_arch = "wasm32")]
        {
            0
        }
    }

    #[inline]
    pub fn elapsed_secs(&self) -> f64 {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.t0.elapsed().as_secs_f64()
        }
        #[cfg(target_arch = "wasm32")]
        {
            0.0
        }
    }
}
