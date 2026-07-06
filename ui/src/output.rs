use crate::arena::NodeId;

pub enum UIOutputEvent {
    Clicked  { id: NodeId },
    Hovered  { id: NodeId },
    Released { id: NodeId },
}