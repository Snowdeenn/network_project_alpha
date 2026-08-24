use serde::Deserialize;
use std::collections::HashMap;
use utils::map::cell::CellKind;

use crate::app::resources::Resources;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TileKind {
    MurPlatAngleBasGauche,
    MurPlatAngleBasDroit,
    MurPlatAngleDroit,
    MurPlatAngleGauche,
    MurPlat,
    MurPlatDroit,
    MurPlatGauche,
    MurAngleBasDroit,
    MurAngleBasGauche,
    MurAngleHautDroit,
    MurAngleHautGauche,
    MurSimple,
    MurSimple2,
    Tree,
    WallBush,
    MoyenCailloux,
    GrosCailloux,
    Flower,
    Tile,
    Grass,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Bounds {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

impl Bounds {
    pub fn to_uv(&self, tex_width: f32, tex_height: f32) -> prism::UvRect {
        prism::UvRect {
            x: self.x as f32 / tex_width,
            y: self.y as f32 / tex_height,
            w: self.w as f32 / tex_width,  // largeur normalisée
            h: self.h as f32 / tex_height, // hauteur normalisée
        }
    }
}

pub struct TextureAtlasInfo {
    pub sprites: HashMap<TileKind, Bounds>,
}

impl TextureAtlasInfo {
    pub fn get(&self, kind: TileKind) -> &Bounds {
        &self.sprites[&kind]
    }
}
impl<'de> Deserialize<'de> for TextureAtlasInfo {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let root = serde_json::Value::deserialize(deserializer)?;

        let slices = root
            .pointer("/meta/slices")
            .and_then(|v| v.as_array())
            .ok_or_else(|| serde::de::Error::custom("Champ '/meta/slices' introuvable"))?;

        let mut map = HashMap::with_capacity(slices.len());

        for slice in slices {
            // Convertit le champ "name" du JSON directement en TileKind
            let name_val = slice
                .get("name")
                .ok_or_else(|| serde::de::Error::custom("Slice sans champ 'name'"))?;
            let kind: TileKind =
                serde_json::from_value(name_val.clone()).map_err(serde::de::Error::custom)?;

            // Extrait les bounds de keys[0]
            let bounds_val = slice
                .pointer("/keys/0/bounds")
                .ok_or_else(|| serde::de::Error::custom("Bounds introuvables dans keys[0]"))?;
            let bounds: Bounds =
                serde_json::from_value(bounds_val.clone()).map_err(serde::de::Error::custom)?;

            map.insert(kind, bounds);
        }

        Ok(TextureAtlasInfo { sprites: map })
    }
}

pub struct TileMap {
    texture_atlas: utils::ids::TextureId,
    atlas_info: TextureAtlasInfo,
    tile_size: u32,
}

impl TileMap {
    pub fn new(
        gpu_resource: &mut prism::GpuResources,
        ctx: &prism::GpuContext,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let atlas_info_str = include_str!("../../../assets/map_texture/map_sprite_sheet.json");
        let atlas_info: TextureAtlasInfo = serde_json::from_str(atlas_info_str)?;
        let texture_atlas =
            gpu_resource.load_texture(ctx, "assets/map_texture/map_sprite_sheet.png")?;

        Ok(Self {
            texture_atlas,
            atlas_info,
            tile_size: 64,
        })
    }
    pub fn draw(
        &self,
        app_resources: &Resources,
        gpu_resources: &prism::GpuResources,
        frame: &mut prism::Frame,
        screen_w: f32,
        screen_h: f32,
    ) {
        let grid = app_resources.read_resource::<utils::map::grid::Grid>();
        let texture_atlas = gpu_resources.get_texture(self.texture_atlas).unwrap();
        let (atlas_w, atlas_h) = texture_atlas.size;
        let tile = self.tile_size as f32;

        // Cellules visibles à l'écran
        let half_w = screen_w * 0.5;
        let half_h = screen_h * 0.5;

        // Coin haut-gauche de la caméra en coordonnées monde
        let world_min_x = (frame.camera_pos.x - half_w).max(0.0);
        let world_min_y = (frame.camera_pos.y - half_h).max(0.0);
        let world_max_x = (frame.camera_pos.x + half_w).min(grid.width() as f32 * tile);
        let world_max_y = (frame.camera_pos.y + half_h).min(grid.height() as f32 * tile);

        // Convertir en indices de cellules
        let cell_min_x = (world_min_x / tile) as u32;
        let cell_min_y = (world_min_y / tile) as u32;
        let cell_max_x = ((world_max_x / tile) as u32 + 1).min(grid.width());
        let cell_max_y = ((world_max_y / tile) as u32 + 1).min(grid.height());

        for x in cell_min_x..cell_max_x {
            for y in cell_min_y..cell_max_y {
                let cell = grid.get(x, y).unwrap();
                let world_x = x as f32 * tile;
                let world_y = y as f32 * tile;

                let tile_kind = match cell.kind {
                    CellKind::Wall => TileKind::MurSimple,
                    CellKind::Floor => TileKind::Grass,
                    _ => continue,
                };

                let bounds = self.atlas_info.get(tile_kind);
                frame.push_world(prism::DrawCommand::Texture {
                    id: self.texture_atlas,
                    pos: [world_x, world_y],
                    size: [tile, tile],
                    rotation: 0.0,
                    uv: Some(bounds.to_uv(atlas_w as f32, atlas_h as f32)),
                    tint: [1.0, 1.0, 1.0, 1.0],
                    blend: prism::BlendMode::Alpha,
                    layer: 0,
                });
            }
        }
    }
}
