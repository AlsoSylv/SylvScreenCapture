#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "windows")]
pub use windows::{Mutex, ShMem};

mod common {
    use core::{
        ops::{Deref, DerefMut},
        sync::atomic::AtomicU8,
    };

    #[repr(C)]
    pub struct View<T> {
        programs_holding: AtomicU8,
        inner: T,
    }

    impl<T> View<T>
    where
        T: Default,
    {
        pub fn new() -> Self {
            Self {
                programs_holding: AtomicU8::new(0),
                inner: T::default(),
            }
        }

        pub fn inc_ref_count(&self) -> u8 {
            self.programs_holding
                .fetch_add(1, core::sync::atomic::Ordering::SeqCst)
        }

        #[allow(unused)]
        pub fn ref_count(&self) -> &AtomicU8 {
            &self.programs_holding
        }

        pub fn dec_ref_count(&self) -> u8 {
            self.programs_holding
                .fetch_sub(1, core::sync::atomic::Ordering::SeqCst)
        }
    }

    impl<T> Deref for View<T> {
        type Target = T;

        fn deref(&self) -> &Self::Target {
            &self.inner
        }
    }

    impl<T> DerefMut for View<T> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.inner
        }
    }

    // pub enum WaitEvent {
    //     Success,
    //     Failed,
    //     Timeout,
    // }
}
