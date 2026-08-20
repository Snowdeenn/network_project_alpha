#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CellKind {
    #[default]
    Floor,
    Wall,
    Spawn,
    Water,
    Debris
}

#[derive(Default, Clone, Copy)]
pub struct Cell {
    pub kind: CellKind,
    pub cost: u8,   // 1=normal, 128=lent, 255=bloqué
}