mod loader;
mod parser;

enum VfxFamily {
    Particle,
    Mesh { logical: MeshLogicalDescriptor },
    Trail,
    Light,
    PostProcess,
}

enum VfxEmissionMode {
    Burst,
    Continuous,
}

struct VfxEmission {
    pub mode: VfxEmissionMode,
    pub count: u32,
    pub delay_seconds: f32,
}

enum VfxSpace {
    World,
    Local,
}

enum VfxShape {
    Disk { radius: f32 },
    Point { coord: [f32; 2] },
}

struct LogicalDeath<D> {
    pub when: Vec<String>,
    pub on_death: D,
}

// ============ Mesh Description ======================================================================
struct MeshInit {
    pub space: VfxSpace,
    pub shape: VfxShape,
    pub life_time_seconds: [f32; 2],
}

struct MeshLogicalDeath {
    pub stop_simulation: bool,
}

struct MeshLogicalDescriptor {
    pub emission: VfxEmission,
    pub init: MeshInit,
    pub logical_death: LogicalDeath<MeshLogicalDeath>,
}

// ====================================================================================================

// ============ Particle Description ==================================================================

struct ParticleInit {
    pub space: VfxSpace,
    pub shape: VfxShape,
    pub velocity: [f32; 2],
    pub life_time_seconds: [f32; 2]
}

struct ParticleEvolution {
    pub accel: [f32; 2],
    pub drag: f32
}

struct ParticleReaction {
    on: ReactionEvent,
    actions: Vec<ReactionAction>,
}

enum ReactionEvent {
    Collision,
    LogicalDeath,
}

enum ReactionAction {
    SetVelocity { value: [f32; 2] },
    RequestDeath { reason: String },
}

struct ParticleLogicalDeath {
    pub stop_simulation: bool,
    pub disable_collision: bool,
}

struct ParticleLogicalDescriptor {
    pub emission: VfxEmission,
    pub init: ParticleInit,
    pub evolution: ParticleEvolution,
    pub reaction: ParticleReaction,
    pub logical_death: LogicalDeath<ParticleLogicalDeath>
}

// ====================================================================================================

struct VfxEffectDescriptor {
    pub version: u32,
    pub name: String,
    pub family: VfxFamily,
}
