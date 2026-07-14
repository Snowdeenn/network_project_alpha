pub mod node;
pub mod context;
pub mod layout;
pub mod draw;
pub mod event;
pub mod tween;
pub mod r#macro;
pub mod input;
pub mod output;
pub mod provider;

pub mod prelude {
    pub use crate::node::*;
    pub use crate::context::UiContext;
    pub use crate::layout::*;
    pub use crate::draw::*;
    pub use crate::event::*;
    pub use crate::tween::*;
    pub use crate::input::*;
    pub use crate::output::UIOutputEvent;
    pub use crate::provider::*;
}

pub use shared::arena::{Arena, Id};
use crate::node::UiNode;
pub type NodeId = Id<UiNode>;