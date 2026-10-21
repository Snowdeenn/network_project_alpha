use std::ops::{Deref, DerefMut};

use utils::arena::*;

use crate::vfx::parser::r#struct::VfxEffectDescriptor;


pub struct AssetRegister(Arena<VfxEffectDescriptor>);

impl AssetRegister {
   pub fn new() -> Self {
       AssetRegister(Arena::new())
   }
}

impl DerefMut for AssetRegister {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Deref for AssetRegister {
    type Target = Arena<VfxEffectDescriptor>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
