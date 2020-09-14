use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

pub struct ThreadStatus {
    is_rt: AtomicBool,
    atomic_action: AtomicU8,
}

impl ThreadStatus {
    pub const fn new() -> Self {
        Self {
            is_rt: AtomicBool::new(false),
            atomic_action: AtomicU8::new(0),
        }
    }

    pub fn set_rt(&self) {
        self.is_rt.store(true, Ordering::Release);
    }
    pub fn unset_rt(&self) {
        self.is_rt.store(false, Ordering::Release);
    }
    pub fn is_rt(&self) -> bool {
        self.is_rt.load(Ordering::Acquire)
    }

    pub fn set_action(&self, action: FailAction) {
        let action_byte = action as u8;
        self.atomic_action.store(action_byte, Ordering::Release)
    }

    pub fn action(&self) -> FailAction {
        let action_byte = self.atomic_action.load(Ordering::Acquire);
        unsafe { std::mem::transmute(action_byte) }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[repr(u8)]
pub enum FailAction {
    Panic,
    Nothing,
}

thread_local! {static RT_FLAG : ThreadStatus = ThreadStatus::new();}

pub struct DebugRtAllocator {}

impl DebugRtAllocator {
    const fn new() -> Self {
        Self {}
    }

    pub fn set_rt(&self) {
        RT_FLAG.with(|flag| {
            flag.set_rt();
        });
    }

    pub fn unset_rt(&self) {
        RT_FLAG.with(|flag| {
            flag.unset_rt();
        });
    }

    pub fn is_rt(&self) -> bool {
        RT_FLAG.with(|flag| flag.is_rt())
    }

    pub fn set_action(&self, action : FailAction) {
        RT_FLAG.with(|flag| flag.set_action(action));
    }

    pub fn action(&self) -> FailAction {
        RT_FLAG.with(|flag| flag.action())
    }

    #[track_caller]
    fn assert_not_rt(&self, _layout: Layout) {
        if self.is_rt() && self.action() == FailAction::Panic {
            self.unset_rt();
            panic!("Tried to allocate in RT-thread.");
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
    #[test]
    fn test_nopanic_alloc() {
        let (snd, recv) = mpsc::sync_channel(4);
        let cb = move || {
            let heapa = Box::new([0u8; 16]);
            let heapb = Box::new([1u8; 16]);
            snd.send(heapa.as_ptr() as usize).unwrap();
            snd.send(heapb.as_ptr() as usize).unwrap();
            MYALLOC.set_rt();
            MYALLOC.set_action(FailAction::Nothing);
            let heapc = Box::new([2u8; 44]);
            snd.send(heapc.as_ptr() as usize).unwrap();
            MYALLOC.unset_rt();
        };
        let panicer = thread::spawn(cb);
        let mut vals = Vec::with_capacity(2);
        let tm = std::time::Duration::from_millis(500);
        vals.push(recv.recv_timeout(tm).unwrap());
        vals.push(recv.recv_timeout(tm).unwrap());
        vals.push(recv.recv_timeout(tm).unwrap());
        let joined = panicer.join();
        assert!(joined.is_ok());
        assert_eq!(3, vals.len());
    }
}
