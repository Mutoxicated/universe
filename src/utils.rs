pub struct MutPtr<T: ?Sized> {
    inner: *mut T,
}

impl<T: ?Sized> MutPtr<T> {
    pub fn inner(&mut self) -> *mut T {
        self.inner
    }
}

unsafe impl<T: ?Sized + Sync + Send> Send for MutPtr<T> {}
unsafe impl<T: ?Sized + Sync + Send> Sync for MutPtr<T> {}
