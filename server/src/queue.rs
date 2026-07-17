use std::ops::{Deref, DerefMut};

pub struct Queue<T> {
    pub data: Vec<T>,
}

impl<T> Deref for Queue<T> {
    type Target = Vec<T>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T> DerefMut for Queue<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}