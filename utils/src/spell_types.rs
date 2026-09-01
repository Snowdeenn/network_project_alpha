use std::ops::Deref;

use crate::Id;

use crate::ids::SpellTag;

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash)]
pub struct SpellId(Id<SpellTag>);
impl SpellId {
    pub fn get(&self) -> Id<SpellTag> {
        self.0
    }
}
impl Deref for SpellId {
    type Target = Id<SpellTag>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl From<Id<SpellTag>> for SpellId {
    fn from(val: Id<SpellTag>) -> Self {
        Self(val)
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, bincode::Encode, bincode::Decode)]
pub struct SpellCost {
    pub cooldown: f32,
    pub gold: u32,
    pub charges: u32,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, bincode::Encode, bincode::Decode)]
pub struct SpellTargetingConfig {
    pub kind: SpellTargetingKind,
    pub range: f32,
    pub projectile_radius: f32,
    pub speed: f32,
    pub aoe: Option<AoeSpellShape>,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, bincode::Encode, bincode::Decode)]
#[serde(tag = "shape")]
pub enum AoeSpellShape {
    Circle {
        #[serde(default = "crate::math::Vec2::zero")]
        offset: crate::math::Vec2,
        radius: f32,
    },
    Box {
        #[serde(default = "crate::math::Vec2::zero")]
        offset: crate::math::Vec2,
        size: crate::math::Vec2,
        rotation: f32,
    },
    Cone {
        #[serde(default = "crate::math::Vec2::zero")]
        offset: crate::math::Vec2,
        direction: crate::math::Vec2,
        angle: f32,
        range: f32,
    },
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, bincode::Encode, bincode::Decode)]
pub enum SpellTargetingKind {
    Directional,
    OnSelf,
    SingleTarget,
    //...
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, bincode::Encode, bincode::Decode)]
#[serde(tag = "kind")]
pub enum SpellEffectKind {
    Damage {
        amount: f32,
        element: Element,
    },
    Knockback {
        force: f32,
    },
    ApplyStatus {
        status: AppliedStatus,
        duration: f32,
        tick_interval: f32,
        damage_per_tick: f32,
    },
    Heal {
        amount: u32,
    },
    
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, bincode::Encode, bincode::Decode)]
pub enum AppliedStatus {
    Burn,
    Blind,
    Slowed,
    // ...
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, bincode::Encode, bincode::Decode)]
pub enum Element {
    Fire,
    Water,
    Wind,
    Earth,
    // ...
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct RawSpell {
    pub id: String,
    pub name: String,
    pub description: String,
    pub costs: SpellCost,
    pub targeting: SpellTargetingConfig,
    pub effects: Vec<SpellEffectKind>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize, Debug, bincode::Encode, bincode::Decode)]
pub struct Spell {
    pub name: String,
    pub description: String,
    pub costs: SpellCost,
    pub targeting: SpellTargetingConfig,
    pub effects: Vec<SpellEffectKind>,
}

impl RawSpell {
    pub fn into_spell(self) -> (String, Spell) {
        (
            self.id,
            Spell {
                name: self.name,
                description: self.description,
                costs: self.costs,
                targeting: self.targeting,
                effects: self.effects,
            },
        )
    }
}
