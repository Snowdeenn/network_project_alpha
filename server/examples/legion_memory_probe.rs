//! Isolated Legion probe: no server, network, game resources or logging.
use legion::{
    EntityStore, Resources, Schedule, World,
    systems::{CommandBuffer, SystemBuilder},
};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    io::{self, Write},
    sync::atomic::{AtomicUsize, Ordering},
};

struct Counted;
static LIVE: AtomicUsize = AtomicUsize::new(0);
static ALLOCATED: AtomicUsize = AtomicUsize::new(0);
unsafe impl GlobalAlloc for Counted {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            LIVE.fetch_add(layout.size(), Ordering::Relaxed);
            ALLOCATED.fetch_add(layout.size(), Ordering::Relaxed);
        }
        ptr
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
        unsafe { System.dealloc(ptr, layout) };
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        let result = unsafe { System.realloc(ptr, layout, size) };
        if !result.is_null() {
            if size >= layout.size() {
                LIVE.fetch_add(size - layout.size(), Ordering::Relaxed);
            } else {
                LIVE.fetch_sub(layout.size() - size, Ordering::Relaxed);
            }
            ALLOCATED.fetch_add(size, Ordering::Relaxed);
        }
        result
    }
}
#[global_allocator]
static ALLOCATOR: Counted = Counted;

#[derive(Clone, Copy)]
struct Position(f64, f64);
struct Velocity(f64, f64);
struct Active(bool);
#[allow(dead_code)]
struct Lifetime(f32);

fn checkpoint(mode: &str, phase: &str, operations: usize, entities: usize) {
    // Capture before formatting; Python samples this process while it waits for ACK.
    let live = LIVE.load(Ordering::Relaxed);
    let allocated = ALLOCATED.load(Ordering::Relaxed);
    println!(
        "{}",
        serde_json::json!({"mode":mode,"phase":phase,"operations":operations,"entities":entities,"live_bytes":live,"cumulative_allocated_bytes":allocated})
    );
    io::stdout().flush().unwrap();
    let mut ack = String::new();
    assert!(
        io::stdin().read_line(&mut ack).unwrap() > 0,
        "missing monitor ACK"
    );
}

fn main() {
    let mode = std::env::args()
        .nth(1)
        .expect("direct|commands|schedule|pool|take|recycle");
    if mode == "flowfield_system" {
        use server::{
            navigation::FlowFieldManager,
            simulation::resources::components::{Player, Position},
        };
        checkpoint(&mode, "before_world", 0, 0);
        let mut world = World::default();
        world.push((Player, Position { x: 0.0, y: 0.0 }));
        let mut resources = Resources::default();
        resources.insert(utils::map::grid::Grid::new(150, 100, 64.0));
        resources.insert(FlowFieldManager::default());
        let mut schedule = Schedule::builder()
            .add_system(server::simulation::systems::flow_field::update_flow_fields_system())
            .build();
        schedule.execute(&mut world, &mut resources);
        checkpoint(&mode, "baseline", 0, world.len());
        let mut count = 0;
        for target in [1, 10, 100] {
            while count < target {
                let transient = world.push((Position { x: 0.0, y: 0.0 },));
                schedule.execute(&mut world, &mut resources);
                world.remove(transient);
                count += 1;
            }
            // Reproduces the defect: fields persist for non-player entities.
            assert_eq!(
                resources.get::<FlowFieldManager>().unwrap().fields.len(),
                count + 1
            );
            checkpoint(&mode, "checkpoint", count, world.len());
        }
        drop(schedule);
        drop(resources);
        drop(world);
        checkpoint(&mode, "after_drop", count, 0);
        return;
    }
    if mode == "flowfield" {
        checkpoint(&mode, "before_world", 0, 0);
        let grid = utils::map::grid::Grid::new(150, 100, 64.0);
        let mut field = utils::map::flow_field::FlowField::new(150, 100);
        checkpoint(&mode, "baseline", 0, 1);
        let mut count = 0;
        for target in [1, 10, 100, 1000] {
            while count < target {
                field.compute(
                    &grid,
                    utils::math::Vec2::new((count % 100) as f32 * 64.0, (count % 90) as f32 * 64.0),
                );
                count += 1;
            }
            checkpoint(&mode, "checkpoint", count, 1);
        }
        drop(field);
        drop(grid);
        checkpoint(&mode, "after_drop", count, 0);
        return;
    }
    if mode == "take" || mode == "recycle" {
        checkpoint(&mode, "before_world", 0, 0);
        let mut manager = utils::buffer::BufferManager::with_capacity(1);
        checkpoint(&mode, "baseline", 0, 0);
        let mut operations = 0;
        for target in [1_024, 7_040, 10_048, 100_032] {
            while operations < target {
                let (id, buffer) = manager.acquire::<Vec<u8>>().unwrap();
                buffer.resize(4096, 1);
                let owned = std::mem::take(buffer);
                std::hint::black_box(&owned);
                if mode == "recycle" {
                    *manager.get_mut::<Vec<u8>>(id).unwrap() = owned;
                } else {
                    drop(owned);
                }
                manager.release(id);
                operations += 1;
            }
            checkpoint(&mode, "checkpoint", operations, 0);
        }
        drop(manager);
        checkpoint(&mode, "after_drop", operations, 0);
        return;
    }
    assert!(["direct", "commands", "schedule", "pool"].contains(&mode.as_str()));
    checkpoint(&mode, "before_world", 0, 0);
    let mut world = World::default();
    let mut resources = Resources::default();
    for _ in 0..150 {
        world.push((Position(0.0, 0.0), Active(true)));
    }
    let mut command = CommandBuffer::new(&world);
    let mut schedule = Schedule::builder()
        .add_system(SystemBuilder::new("isolated_churn").build(|cmd, _, _, _| {
            for _ in 0..64 {
                let e = cmd.push((Position(1.0, 2.0), Velocity(3.0, 4.0)));
                cmd.add_component(e, Active(true));
                cmd.add_component(e, Lifetime(1.0));
                cmd.remove_component::<Lifetime>(e);
                cmd.remove(e);
            }
        }))
        .build();
    let pool: Vec<_> = if mode == "pool" {
        (0..64)
            .map(|_| world.push((Position(1.0, 2.0), Velocity(3.0, 4.0), Active(false))))
            .collect()
    } else {
        vec![]
    };
    checkpoint(&mode, "baseline", 0, world.len());
    let mut operations = 0;
    for target in [1_024, 7_040, 10_048, 100_032] {
        while operations < target {
            match mode.as_str() {
                "direct" => {
                    for _ in 0..64 {
                        let e = world.push((Position(1.0, 2.0), Velocity(3.0, 4.0)));
                        let mut entry = world.entry(e).unwrap();
                        entry.add_component(Active(true));
                        entry.add_component(Lifetime(1.0));
                        entry.remove_component::<Lifetime>();
                        world.remove(e);
                    }
                }
                "commands" => {
                    for _ in 0..64 {
                        let e = command.push((Position(1.0, 2.0), Velocity(3.0, 4.0)));
                        command.add_component(e, Active(true));
                        command.add_component(e, Lifetime(1.0));
                        command.remove_component::<Lifetime>(e);
                        command.remove(e);
                    }
                    command.flush(&mut world, &mut resources);
                }
                "schedule" => schedule.execute(&mut world, &mut resources),
                "pool" => {
                    for &e in &pool {
                        let mut entry = world.entry_mut(e).unwrap();
                        let pos = entry.get_component_mut::<Position>().unwrap();
                        pos.0 += 1.0;
                        pos.1 += 1.0;
                        entry.get_component_mut::<Active>().unwrap().0 = true;
                        let vel = entry.get_component_mut::<Velocity>().unwrap();
                        vel.0 += 1.0;
                        vel.1 += 1.0;
                        entry.get_component_mut::<Active>().unwrap().0 = false;
                    }
                }
                _ => unreachable!(),
            }
            operations += 64;
        }
        checkpoint(&mode, "checkpoint", operations, world.len());
    }
    drop(pool);
    drop(schedule);
    drop(command);
    drop(resources);
    drop(world);
    checkpoint(&mode, "after_drop", operations, 0);
}
