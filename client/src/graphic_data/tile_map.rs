pub struct TileMap {
    texture_atlas: utils::ids::TextureId,
    tile_size: u32,
}

impl TileMap {
    pub fn new(gpu_resource: &mut prism::GpuResources, ctx: &prism::GpuContext) -> Self {}
    pub fn draw(gpu_resources: &prism::GpuResources, frame: &mut prism::Frame) {}
}
