use std::{
    cell::UnsafeCell,
    ffi::CStr,
    ops::{Deref, DerefMut},
};

mod os;

pub struct Shmem<T>
where
    T: Default,
{
    inner: os::ShMem<T>,
}

// In theory, as long as the inner type would be safe across multiple threads, the shared memory is
unsafe impl<T> Send for Shmem<T> where T: Send + Default {}
unsafe impl<T> Sync for Shmem<T> where T: Sync + Default {}

impl<T> Shmem<T>
where
    T: Default,
{
    pub fn new(name: &CStr) -> Self {
        Shmem {
            inner: os::ShMem::new(name).unwrap(),
        }
    }

    pub fn open(name: &CStr) -> Self {
        Shmem {
            inner: os::ShMem::open(name).unwrap(),
        }
    }

    pub fn ref_count(&self) -> u8 {
        self.inner
            .view()
            .ref_count()
            .load(std::sync::atomic::Ordering::SeqCst)
    }

    /// # Safety
    /// Calling this can trigger the deconstructor, and should only be called if this is the intended effect
    pub unsafe fn dec_ref_count(&mut self) {
        unsafe {
            self.inner.dec_ref_count();
        }
    }
}

impl<T> Deref for Shmem<T>
where
    T: Default,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

pub struct LockedSharedMem<T>
where
    T: Default,
{
    mem: UnsafeCell<os::ShMem<T>>,
    mutex: os::Mutex,
}

impl<T> LockedSharedMem<T>
where
    T: Default,
{
    /// SAFETY: Operations on the underlying shared memory are only safe if the allocation and mutex are opened with the same name every time they're used.
    /// Since the shared memory is reference counted, the mutex has to be locked BEFORE the allocation can be opened.
    pub unsafe fn new(mem_name: &CStr, mutex_name: &CStr) -> Self {
        let mutex = os::Mutex::new(mutex_name).unwrap();
        mutex.lock();

        let mem = os::ShMem::new(mem_name).unwrap();

        mutex.release().unwrap();

        LockedSharedMem {
            mem: UnsafeCell::new(mem),
            mutex,
        }
    }

    /// SAFETY: Operations on the underlying shared memory are only safe if the allocation and mutex are opened with the same name every time they're used.
    /// Since the shared memory is reference counted, the mutex has to be locked BEFORE the allocation can be opened.
    pub unsafe fn open(mem_name: &CStr, mutex_name: &CStr) -> Self {
        let mutex = os::Mutex::open(mutex_name).unwrap();

        mutex.lock();

        let mem = os::ShMem::open(mem_name).unwrap();

        mutex.release().unwrap();

        LockedSharedMem {
            mem: UnsafeCell::new(mem),
            mutex,
        }
    }

    pub fn lock(&self) -> MutexGuard<T> {
        self.mutex.lock();

        MutexGuard {
            mutex: &self.mutex,
            mem: &mut *unsafe { self.mem.get().as_mut().unwrap() },
        }
    }
}

pub struct MutexGuard<'a, T> {
    mutex: &'a os::Mutex,
    mem: &'a mut T,
}

impl<T> Drop for MutexGuard<'_, T> {
    fn drop(&mut self) {
        self.mutex.release().unwrap();
    }
}

impl<T> Deref for MutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.mem
    }
}

impl<T> DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.mem
    }
}
