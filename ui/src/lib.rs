pub mod context;
pub mod draw;
pub mod event;
pub mod input;
pub mod layout;
pub mod r#macro;
pub mod node;
pub mod output;
pub mod provider;
pub mod tween;

pub use crate::context::UiContext;
pub use crate::draw::*;
pub use crate::event::*;
pub use crate::input::*;
pub use crate::layout::*;
pub use crate::node::*;
pub use crate::output::UIOutputEvent;
pub use crate::provider::*;
pub use crate::tween::*;

pub use utils::arena::{Arena, Id};
pub type NodeId = Id<UiNode>;
