use crate::map::cell::*;
pub struct Grid {
    width: u32,
    height: u32,
    cell_size: f32,
    cells: Vec<Cell>,
}

impl Grid {
    pub fn new(width: u32, height: u32, cell_size: f32) -> Self {
        Self {
            width,
            height,
            cell_size,
            cells: [Cell::default()].repeat((width * height) as usize),
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn get<'a>(&'a self, x: u32, y: u32) -> Option<&'a Cell> {
        let idx = self.cell_index(x, y);
        if idx < self.cells.len() {
            Some(&self.cells[idx])
        } else {
            None
        }
    }

    pub fn get_mut<'a>(&'a mut self, x: u32, y: u32) -> Option<&'a mut Cell> {
        let idx = self.cell_index(x, y);
        if idx < self.cells.len() {
            Some(&mut self.cells[idx])
        } else {
            None
        }
    }

    pub fn cell_index(&self, x: u32, y: u32) -> usize {
        assert!(
            x < self.width,
            "X doit être inférieur ou égal à la largeur de la grid"
        );
        assert!(
            y < self.height,
            "Y doit être inférieur ou égal à la hauteur de la gird"
        );
        (y * self.width + x) as usize
    }

    pub fn cell_pos(&self, index: usize) -> (u32, u32) {
        (index as u32 % self.width, index as u32 / self.width)
    }

    pub fn set_cell(&mut self, x: u32, y: u32, cell: Cell) {
        let idx = self.cell_index(x, y);
        self.cells[idx] = cell;
    }

    pub fn world_to_grid(&self, pos: crate::math::Vec2) -> (u32, u32) {
        (
            (pos.x / self.cell_size).floor() as u32,
            (pos.y / self.cell_size).floor() as u32,
        )
    }

    pub fn grid_to_world(&self, x: u32, y: u32) -> crate::math::Vec2 {
        // Le centre de la cell est à l'index + 0.5
        crate::math::Vec2::new(
            (x as f32 + 0.5) * self.cell_size,
            (y as f32 + 0.5) * self.cell_size,
        )
    }

    pub fn is_walkable(&self, x: u32, y: u32) -> bool {
        let cell = &self.cells[self.cell_index(x, y)];
        if matches!(cell.kind, CellKind::Wall) {
            return false;
        }
        true
    }

    pub fn neighbors(&self, x: u32, y: u32) -> Vec<(u32, u32)> {
        let mut neighbors = Vec::with_capacity(8);
        self.each_neighbor_pos(x, y, |nx, ny| {
            neighbors.push((nx, ny));
        });
        neighbors
    }

    fn each_neighbor_pos(&self, x: u32, y: u32, mut f: impl FnMut(u32, u32)) {
        let can_left = x > 0;
        let can_right = x + 1 < self.width;
        let can_up = y > 0;
        let can_down = y + 1 < self.height;

        if can_up {
            if can_left {
                f(x - 1, y - 1);
            }
            f(x, y - 1);
            if can_right {
                f(x + 1, y - 1);
            }
        }
        if can_left {
            f(x - 1, y);
        }
        if can_right {
            f(x + 1, y);
        }
        if can_down {
            if can_left {
                f(x - 1, y + 1);
            }
            f(x, y + 1);
            if can_right {
                f(x + 1, y + 1);
            }
        }
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Cell> {
        self.cells.iter()
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, Cell> {
        self.cells.iter_mut()
    }

    pub fn iter_mut_with_pos(&mut self) -> impl Iterator<Item = (u32, u32, &mut Cell)> {
        let width = self.width;
        self.cells.iter_mut().enumerate().map(move |(idx, cell)| {
            let x = (idx as u32) % width;
            let y = (idx as u32) / width;
            (x, y, cell)
        })
    }

    pub fn iter_with_pos(&self) -> impl Iterator<Item = (u32, u32, &Cell)> {
        let width = self.width;
        self.cells.iter().enumerate().map(move |(idx, cell)| {
            let x = (idx as u32) % width;
            let y = (idx as u32) / width;
            (x, y, cell)
        })
    }

    pub fn len(&self) -> usize {
        self.cells.len()
    }
}

impl<'a> IntoIterator for &'a Grid {
    type Item = &'a Cell;
    type IntoIter = std::slice::Iter<'a, Cell>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a> IntoIterator for &'a mut Grid {
    type Item = &'a mut Cell;
    type IntoIter = std::slice::IterMut<'a, Cell>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Fonction utilitaire pour créer une grille valide et remplie
    fn create_dummy_grid(width: u32, height: u32, cell_size: f32) -> Grid {
        let mut grid = Grid::new(width, height, cell_size);
        // Remplit le vecteur pour éviter d'avoir une longueur de 0
        grid.cells = vec![Cell::default(); (width * height) as usize];
        grid
    }

    #[test]
    fn test_world_to_grid_conversion() {
        let grid = create_dummy_grid(10, 10, 16.0);

        // Position (24.0, 40.0) avec taille 16.0 -> (1.5, 2.5) -> case (1, 2)
        let world_pos = crate::math::Vec2::new(24.0, 40.0);
        let grid_pos = grid.world_to_grid(world_pos);

        assert_eq!(grid_pos, (1, 2));
    }

    #[test]
    fn test_grid_to_world_conversion() {
        let grid = create_dummy_grid(10, 10, 16.0);

        // Case (1, 2) doit donner le centre : (1.5 * 16.0, 2.5 * 16.0) = (24.0, 40.0)
        let world_pos = grid.grid_to_world(1, 2);

        assert_eq!(world_pos.x, 24.0);
        assert_eq!(world_pos.y, 40.0);
    }

    #[test]
    fn test_cell_index_and_pos_reversibility() {
        let grid = create_dummy_grid(5, 5, 10.0);
        let (x, y) = (3, 2);

        let index = grid.cell_index(x, y);
        let (res_x, res_y) = grid.cell_pos(index);

        assert_eq!((x, y), (res_x, res_y));
    }

    #[test]
    fn test_neighbors_counts() {
        let grid = create_dummy_grid(5, 5, 10.0);

        // Coin haut-gauche : seulement 2 voisins
        let corner_neighbors = grid.neighbors(0, 0);
        assert_eq!(corner_neighbors.len(), 3);
        assert!(corner_neighbors.contains(&(1, 0)));
        assert!(corner_neighbors.contains(&(0, 1)));
        assert!(corner_neighbors.contains(&(1, 1)));

        // Centre : 8 voisins
        let center_neighbors = grid.neighbors(2, 2);
        assert_eq!(center_neighbors.len(), 8);

        // Bord droit : 5 voisins
        let edge_neighbors = grid.neighbors(4, 2);
        assert_eq!(edge_neighbors.len(), 5);
    }

    #[test]
    #[should_panic(expected = "X doit être inférieur")]
    fn test_cell_index_out_of_bounds_x() {
        let grid = create_dummy_grid(5, 5, 10.0);
        grid.cell_index(5, 2); // Doit paniquer car x == width
    }

    #[test]
    #[should_panic(expected = "Y doit être inférieur")]
    fn test_cell_index_out_of_bounds_y() {
        let grid = create_dummy_grid(5, 5, 10.0);
        grid.cell_index(2, 5); // Doit paniquer car y == height
    }
}
