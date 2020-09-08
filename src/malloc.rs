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
                dprintf(1, b"Tried to allocate in RT-thread.\0".as_ptr() as *const _);
                handle_alloc_error(layout);
            }
        }
    }
}

extern "C" {
    fn dprintf(fd: u32, format: *const u8);
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

#[cfg_attr(feature="rt-alloc-panic", global_allocator)]
pub static MYALLOC: DebugRtAllocator = DebugRtAllocator::new();
