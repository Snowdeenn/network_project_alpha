use utils::{
    arena::Arena,
    ids::{BufferId, BufferTag},
};

use crate::context::GpuContext;

pub struct GpuBuffer {
    pub buffer: wgpu::Buffer,
    pub size: u64,
    pub usage: wgpu::BufferUsages,
}

pub struct GpuBufferManager {
    buffers: Arena<GpuBuffer, BufferTag>,
}

impl GpuBufferManager {
    pub fn new() -> Self {
        Self {
            buffers: Arena::new(),
        }
    }

    pub fn create_buffer(
        &mut self,
        ctx: &GpuContext,
        size: u64,
        usage: wgpu::BufferUsages,
    ) -> BufferId {
        let buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size,
            usage,
            mapped_at_creation: false,
        });
        self.buffers.insert(GpuBuffer {
            buffer,
            size,
            usage,
        })
    }

    pub fn write_buffer(&self, ctx: &GpuContext, id: BufferId, data: &[u8]) {
        let buffer = self
            .buffers
            .get(id)
            .expect("[GpuBufferManager] GpuBuffer invalide check l'id");
        ctx.queue.write_buffer(&buffer.buffer, 0, data);
    }

    pub fn get(&self, id: BufferId) -> Option<&GpuBuffer> {
        self.buffers.get(id)
    }

    pub fn get_mut(&mut self, id: BufferId) -> Option<&mut GpuBuffer> {
        self.buffers.get_mut(id)
    }

    pub fn remove(&mut self, id: BufferId) -> Option<GpuBuffer> {
        self.buffers.remove(id)
    }
}
