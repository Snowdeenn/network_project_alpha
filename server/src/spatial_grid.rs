use crate::simulation::components::*;
use legion::Entity;

pub struct SpatialGrid {
    cells: Vec<Entity>,
    offsets: Vec<usize>,
    counts: Vec<usize>,
    pending: Vec<(Entity, usize)>,
    cell_size: f64,
    cols: usize,
    rows: usize,
}

impl SpatialGrid {
    pub fn new(cell_size: f64, arena_w: f64, arena_h: f64) -> Self {
        let cols = (arena_w / cell_size).ceil() as usize;
        let rows = (arena_h / cell_size).ceil() as usize;
        Self {
            cells: Vec::with_capacity(cols * rows * 4),
            offsets: vec![0; cols * rows],
            counts: vec![0; cols * rows],
            pending: vec![],
            cell_size,
            cols,
            rows,
        }
    }

    #[inline]
    pub fn clear(&mut self) {
        self.counts.fill(0);
        self.cells.clear();
    }

    pub fn cell_index(&self, x: f64, y: f64) -> usize {
        (y / self.cell_size) as usize * self.cols + (x / self.cell_size) as usize
    }

    pub fn insert(&mut self, entity: Entity, pos: &Position, collider: &Collider) {
        let (min_col, max_col, min_row, max_row) = self.bounds(pos, collider);

        for row in min_row..=max_row {
            for cols in min_col..=max_col {
                let cell_index = row * self.cols as usize + cols;
                self.counts[cell_index] += 1;
                self.pending.push((entity, cell_index));
            }
        }
    }

    pub fn build(&mut self) {
        self.offsets[0] = 0;
        for i in 1..self.counts.len() {
            self.offsets[i] = self.offsets[i - 1] + self.counts[i - 1];
        }
        let total: usize = self.counts.iter().sum();
        self.cells.resize(total, self.pending[0].0); // entity random -> self.pending[0].0 valeur arbitraire parce qu'on s'en fou de la valeur dans le resize

        let mut write_pos = self.offsets.clone();
        self.pending.iter().for_each(|(entity, cell_index)| {
            self.cells[write_pos[*cell_index]] = *entity;
            write_pos[*cell_index] += 1
        });
    }

    pub fn query(&self, pos: &Position, collider: &Collider, out: &mut Vec<Entity>) {
        let (min_col, max_col, min_row, max_row) = self.bounds(pos, collider);
        for row in min_row..=max_row {
            for cols in min_col..=max_col {
                let cell_index = row * self.cols as usize + cols;
                let start = self.offsets[cell_index];
                let end = start + self.counts[cell_index];
                out.extend_from_slice(&self.cells[start..end]);
            }
        }
    }

    fn bounds(&self, pos: &Position, collider: &Collider) -> (usize, usize, usize, usize) {
        let min_x = (pos.x / self.cell_size).floor().max(0.0) as usize;
        let max_x = (((pos.x + collider.w) / self.cell_size).floor() as usize)
            .min(self.cols.saturating_sub(1));
        let min_y = (pos.y / self.cell_size).floor().max(0.0) as usize;
        let max_y = (((pos.y + collider.h) / self.cell_size).floor() as usize)
            .min(self.rows.saturating_sub(1));

        (min_x, max_x, min_y, max_y)
    }
}
