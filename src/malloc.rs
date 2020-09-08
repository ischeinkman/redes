use std::alloc::handle_alloc_error;
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, Ordering};

thread_local! {static RT_FLAG : AtomicBool = AtomicBool::new(false);}

pub struct DebugRtAllocator {}

impl DebugRtAllocator {
    const fn new() -> Self {
        Self {}
    }

    pub fn set_rt(&self) {
        RT_FLAG.with(|flag| {
            flag.store(true, Ordering::Release);
        });
    }

    pub fn unset_rt(&self) {
        RT_FLAG.with(|flag| {
            flag.store(false, Ordering::Release);
        });
    }

    pub fn is_rt(&self) -> bool {
        RT_FLAG.with(|flag| flag.load(Ordering::Acquire))
    }

    fn assert_not_rt(&self, layout: Layout) {
        if self.is_rt() {
            unsafe {
                self.unset_rt();
                panic!("Tried to allocate in RT-thread.");
            }
        }
    }
}

static DEF: System = System;

unsafe impl GlobalAlloc for DebugRtAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.assert_not_rt(layout);
        DEF.alloc(layout)
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.assert_not_rt(layout);
        DEF.dealloc(ptr, layout)
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        self.assert_not_rt(layout);
        DEF.realloc(ptr, layout, new_size)
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        self.assert_not_rt(layout);
        DEF.alloc_zeroed(layout)
    }
}

#[cfg_attr(feature = "rt-alloc-panic", global_allocator)]
pub static MYALLOC: DebugRtAllocator = DebugRtAllocator::new();

#[cfg(all(test, feature = "rt-alloc-panic"))]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::thread;

    #[test]
    fn test_panic_alloc() {
        let (snd, recv) = mpsc::sync_channel(4);
        let cb = move || {
            let heapa = Box::new([0u8; 16]);
            let heapb = Box::new([1u8; 16]);
            snd.send(heapa.as_ptr() as usize).unwrap();
            snd.send(heapb.as_ptr() as usize).unwrap();
            MYALLOC.set_rt();
            let heapc = Box::new([2u8; 44]);
            snd.send(heapc.as_ptr() as usize).unwrap();
            MYALLOC.unset_rt();
        };
        let panicer = thread::spawn(cb);
        let mut vals = Vec::with_capacity(2);
        let tm = std::time::Duration::from_millis(500);
        vals.push(recv.recv_timeout(tm).unwrap());
        vals.push(recv.recv_timeout(tm).unwrap());
        assert!(recv.recv_timeout(tm).is_err());
        let joined = panicer.join();
        assert!(joined.is_err());
    }
}
