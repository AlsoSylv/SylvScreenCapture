#[cfg(target_os = "windows")]
mod windows;

pub use windows::ShMem;

mod common {
    use std::sync::atomic::AtomicU8;

    pub struct View<T> {
        ref_count: AtomicU8,
        inner: T,
    }

    impl<T> View<T> {
        pub fn as_ref(&self) -> &T {
            &self.inner
        }

        pub fn as_mut(&mut self) -> &mut T {
            &mut self.inner
        }

        pub fn ref_count(&self) -> &AtomicU8 {
            &self.ref_count
        }

        pub fn dec_ref_count(&self) -> u8 {
            self.ref_count
                .fetch_sub(1, std::sync::atomic::Ordering::SeqCst)
        }
    }
}
