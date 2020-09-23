use std::num::NonZeroU16;

#[macro_export]
macro_rules! const_try {
    ($e:expr) => {{
        let raw = $e;
        match raw {
            Ok(val) => val,
            Err(e) => {
                return Err(e);
            }
        }
    }};
}

#[macro_export]
macro_rules! const_min {
    ($a:expr, $b:expr) => {{
        let ra = $a;
        let rb = $b;
        if ra > rb {
            ra
        } else {
            rb
        }
    }};
}

pub const ONE_NZU16: NonZeroU16 = unsafe { NonZeroU16::new_unchecked(1) };

pub struct TuplerIter<K, V, F: Fn(&K) -> V, I: Iterator<Item = K>> {
    iter: I,
    cb: F,
}

impl<K, V, F: Fn(&K) -> V, I: Iterator<Item = K>> TuplerIter<K, V, F, I> {
    pub fn new(iter : I, cb : F) -> Self {
        Self {iter, cb}
    }
}
impl<K, V, F: Fn(&K) -> V, I: Iterator<Item = K>> Iterator for TuplerIter<K, V, F, I> {
    type Item = (K, V);
    fn next(&mut self) -> Option<Self::Item> {
        let next_k = self.iter.next()?;
        let next_v = (self.cb)(&next_k);
        Some((next_k, next_v))
    }
}

