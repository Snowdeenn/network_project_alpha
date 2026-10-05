//! Allocation accounting without allocating inside the allocator itself.
use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
static ALLOCATED: AtomicU64 = AtomicU64::new(0);
static FREED: AtomicU64 = AtomicU64::new(0);
static CALLS: AtomicU64 = AtomicU64::new(0);
static LIVE: AtomicU64 = AtomicU64::new(0);

pub struct CountingAllocator;
unsafe impl std::alloc::GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        let p = unsafe { std::alloc::GlobalAlloc::alloc(&std::alloc::System, layout) };
        if !p.is_null() {
            allocated(layout.size());
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, layout: std::alloc::Layout) {
        freed(layout.size());
        unsafe { std::alloc::GlobalAlloc::dealloc(&std::alloc::System, p, layout) };
    }
    unsafe fn realloc(&self, p: *mut u8, layout: std::alloc::Layout, size: usize) -> *mut u8 {
        let result =
            unsafe { std::alloc::GlobalAlloc::realloc(&std::alloc::System, p, layout, size) };
        if !result.is_null() {
            record_system(size as u64, layout.size() as u64, 1);
            if size >= layout.size() {
                LIVE.fetch_add((size - layout.size()) as u64, Ordering::Relaxed);
            } else {
                LIVE.fetch_sub((layout.size() - size) as u64, Ordering::Relaxed);
            }
            FREED.fetch_add(layout.size() as u64, Ordering::Relaxed);
            ALLOCATED.fetch_add(size as u64, Ordering::Relaxed);
            CALLS.fetch_add(1, Ordering::Relaxed);
        }
        result
    }
}
fn allocated(size: usize) {
    record_system(size as u64, 0, 1);
    LIVE.fetch_add(size as u64, Ordering::Relaxed);
    ALLOCATED.fetch_add(size as u64, Ordering::Relaxed);
    CALLS.fetch_add(1, Ordering::Relaxed);
}
fn freed(size: usize) {
    record_system(0, size as u64, 0);
    LIVE.fetch_sub(size as u64, Ordering::Relaxed);
    FREED.fetch_add(size as u64, Ordering::Relaxed);
}
#[derive(Clone, Copy, Default, Serialize)]
pub struct Counters {
    pub live_bytes: u64,
    pub allocated_bytes: u64,
    pub freed_bytes: u64,
    pub allocation_calls: u64,
}
pub fn snapshot() -> Counters {
    Counters {
        live_bytes: LIVE.load(Ordering::Relaxed),
        allocated_bytes: ALLOCATED.load(Ordering::Relaxed),
        freed_bytes: FREED.load(Ordering::Relaxed),
        allocation_calls: CALLS.load(Ordering::Relaxed),
    }
}
#[derive(Default, Serialize)]
pub struct Delta {
    pub allocated_bytes: u64,
    pub freed_bytes: u64,
    pub allocation_calls: u64,
    pub net_bytes: i64,
}
pub fn checkpoint(previous: &mut Counters) -> Delta {
    let next = snapshot();
    let result = Delta {
        allocated_bytes: next.allocated_bytes - previous.allocated_bytes,
        freed_bytes: next.freed_bytes - previous.freed_bytes,
        allocation_calls: next.allocation_calls - previous.allocation_calls,
        net_bytes: next.live_bytes as i64 - previous.live_bytes as i64,
    };
    *previous = next;
    result
}
#[derive(Default, Serialize)]
pub struct TickAllocations {
    pub systems: Vec<SystemSample>,
    pub enabled: bool,
    pub totals: Counters,
    pub network_receive: Delta,
    pub commands: Delta,
    pub simulation: Delta,
    pub events: Delta,
    pub snapshots: Delta,
    pub network_send: Delta,
    pub cleanup: Delta,
}

thread_local! { static CURRENT_SYSTEM: std::cell::Cell<usize> = const { std::cell::Cell::new(0) }; }
struct SystemCounters {
    name: std::sync::OnceLock<&'static str>,
    allocated: AtomicU64,
    freed: AtomicU64,
    calls: AtomicU64,
}
static SYSTEMS: [SystemCounters; 64] = [const {
    SystemCounters {
        name: std::sync::OnceLock::new(),
        allocated: AtomicU64::new(0),
        freed: AtomicU64::new(0),
        calls: AtomicU64::new(0),
    }
}; 64];
static NEXT_SYSTEM: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(1);
fn record_system(allocated: u64, freed: u64, calls: u64) {
    let _ = CURRENT_SYSTEM.try_with(|current| {
        let counters = &SYSTEMS[current.get()];
        counters.allocated.fetch_add(allocated, Ordering::Relaxed);
        counters.freed.fetch_add(freed, Ordering::Relaxed);
        counters.calls.fetch_add(calls, Ordering::Relaxed);
    });
}
struct Scope(usize);
impl Scope {
    fn enter(id: usize) -> Self {
        Self(CURRENT_SYSTEM.with(|value| value.replace(id)))
    }
}
impl Drop for Scope {
    fn drop(&mut self) {
        CURRENT_SYSTEM.with(|value| value.set(self.0));
    }
}
#[derive(Serialize)]
pub struct SystemSample {
    pub name: &'static str,
    pub allocated_bytes: u64,
    pub freed_bytes: u64,
    pub allocation_calls: u64,
    pub net_bytes: i64,
}
pub fn system_samples() -> Vec<SystemSample> {
    if !cfg!(feature = "allocation-metrics") {
        return vec![];
    }
    SYSTEMS
        .iter()
        .enumerate()
        .filter_map(|(id, counts)| {
            let name = if id == 0 {
                "outside_systems"
            } else {
                *counts.name.get()?
            };
            let allocated = counts.allocated.load(Ordering::Relaxed);
            let freed = counts.freed.load(Ordering::Relaxed);
            Some(SystemSample {
                name,
                allocated_bytes: allocated,
                freed_bytes: freed,
                allocation_calls: counts.calls.load(Ordering::Relaxed),
                net_bytes: allocated as i64 - freed as i64,
            })
        })
        .collect()
}
pub struct Instrumented<S> {
    inner: S,
    id: usize,
}
pub fn instrument<S>(name: &'static str, inner: S) -> Instrumented<S> {
    if !cfg!(feature = "allocation-metrics") {
        return Instrumented { inner, id: 0 };
    }
    if let Some(id) = SYSTEMS
        .iter()
        .position(|s| s.name.get().is_some_and(|existing| *existing == name))
    {
        return Instrumented { inner, id };
    }
    let id = NEXT_SYSTEM.fetch_add(1, Ordering::Relaxed);
    assert!(id < SYSTEMS.len());
    SYSTEMS[id].name.set(name).unwrap();
    Instrumented { inner, id }
}
impl<S: legion::systems::Runnable> legion::systems::Runnable for Instrumented<S> {
    fn name(&self) -> Option<&legion::systems::SystemId> {
        self.inner.name()
    }
    fn reads(
        &self,
    ) -> (
        &[legion::systems::ResourceTypeId],
        &[legion::storage::ComponentTypeId],
    ) {
        self.inner.reads()
    }
    fn writes(
        &self,
    ) -> (
        &[legion::systems::ResourceTypeId],
        &[legion::storage::ComponentTypeId],
    ) {
        self.inner.writes()
    }
    fn accesses_archetypes(&self) -> &legion::world::ArchetypeAccess {
        self.inner.accesses_archetypes()
    }
    fn prepare(&mut self, world: &legion::World) {
        let _scope = Scope::enter(self.id);
        self.inner.prepare(world);
    }
    unsafe fn run_unsafe(
        &mut self,
        world: &legion::World,
        resources: &legion::systems::UnsafeResources,
    ) {
        let _scope = Scope::enter(self.id);
        unsafe { self.inner.run_unsafe(world, resources) };
    }
    fn command_buffer_mut(
        &mut self,
        world: legion::world::WorldId,
    ) -> Option<&mut legion::systems::CommandBuffer> {
        self.inner.command_buffer_mut(world)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::alloc::{GlobalAlloc, Layout};
    #[test]
    fn counts_allocation_resize_and_release_without_changing_contents() {
        let baseline = snapshot();
        let allocator = CountingAllocator;
        let small = Layout::from_size_align(128, 64).unwrap();
        unsafe {
            let pointer = allocator.alloc_zeroed(small);
            assert!(!pointer.is_null());
            assert_eq!(*pointer, 0);
            assert_eq!(pointer as usize % 64, 0);
            *pointer = 42;
            assert_eq!(snapshot().live_bytes - baseline.live_bytes, 128);
            let pointer = allocator.realloc(pointer, small, 512);
            assert!(!pointer.is_null());
            assert_eq!(*pointer, 42);
            assert_eq!(pointer as usize % 64, 0);
            assert_eq!(snapshot().live_bytes - baseline.live_bytes, 512);
            allocator.dealloc(pointer, Layout::from_size_align(512, 64).unwrap());
        }
        let final_counts = snapshot();
        assert_eq!(final_counts.live_bytes, baseline.live_bytes);
        assert_eq!(final_counts.allocated_bytes - baseline.allocated_bytes, 640);
        assert_eq!(final_counts.freed_bytes - baseline.freed_bytes, 640);
        assert_eq!(final_counts.allocation_calls - baseline.allocation_calls, 2);
    }
}
