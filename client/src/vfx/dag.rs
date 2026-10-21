use utils::arena::Arena;

use crate::vfx::instance::{VfxInstance, VfxInstanceId, VfxInstanceState};

/// Indice local au DAG qui contient le nœud.
pub type VfxNodeId = usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VfxEvent {
    Started,
    Finishing,
    Finished,
}

pub enum VfxNodeEffect {
    Effect(VfxInstanceId),
    Dag(VfxGraph),
}

pub struct VfxNode {
    pub effect: VfxNodeEffect,
    state: VfxInstanceState,
    started_at: Option<f32>,
    finishing_at: Option<f32>,
    finished_at: Option<f32>,
    start_delay: f32,
}

impl VfxNode {
    pub fn state(&self) -> VfxInstanceState {
        self.state
    }

    fn event_time(&self, event: VfxEvent) -> Option<f32> {
        match event {
            VfxEvent::Started => self.started_at,
            VfxEvent::Finishing => self.finishing_at,
            VfxEvent::Finished => self.finished_at,
        }
    }
}

pub struct VfxEdge {
    pub source: VfxNodeId,
    pub target: VfxNodeId,
    pub event: VfxEvent,
    /// Délai en secondes après l'événement source.
    pub delay_seconds: f32,
}

/// Plan d'exécution composé d'instances élémentaires et de sous-DAG.
pub struct VfxGraph {
    pub name: String,
    nodes: Vec<VfxNode>,
    edges: Vec<VfxEdge>,
    state: VfxInstanceState,
    age: f32,
    finished_effect: u32,
}

impl VfxGraph {
    pub fn new(name: String) -> Self {
        Self {
            name,
            nodes: Vec::new(),
            edges: Vec::new(),
            state: VfxInstanceState::Waiting,
            age: 0.0,
            finished_effect: 0,
        }
    }

    pub fn nodes(&self) -> &[VfxNode] {
        &self.nodes
    }

    pub fn edges(&self) -> &[VfxEdge] {
        &self.edges
    }

    pub fn state(&self) -> VfxInstanceState {
        self.state
    }

    pub fn add_node(&mut self, effect: VfxNodeEffect) -> VfxNodeId {
        self.add_delayed_node(effect, 0.0)
    }

    /// Délai depuis l'activation du graphe, y compris pour une racine.
    pub fn add_delayed_node(&mut self, effect: VfxNodeEffect, delay_seconds: f32) -> VfxNodeId {
        assert_eq!(self.state, VfxInstanceState::Waiting);
        assert!(delay_seconds.is_finite() && delay_seconds >= 0.0);
        let id = self.nodes.len();
        self.nodes.push(VfxNode {
            effect,
            state: VfxInstanceState::Waiting,
            started_at: None,
            finishing_at: None,
            finished_at: None,
            start_delay: delay_seconds,
        });
        self.finished_effect += 1;
        id
    }

    pub fn is_finish(&self) -> bool {
        self.finished_effect == 0
    }

    pub fn add_edge(&mut self, edge: VfxEdge) -> Result<(), &'static str> {
        if self.state != VfxInstanceState::Waiting {
            return Err("cannot modify a running DAG");
        }
        if edge.source >= self.nodes.len() || edge.target >= self.nodes.len() {
            return Err("unknown VFX node");
        }
        if !edge.delay_seconds.is_finite() || edge.delay_seconds < 0.0 {
            return Err("VFX delay must be finite and non-negative");
        }
        // Une arête cible -> ... -> source créerait un cycle.
        let mut pending = vec![edge.target];
        let mut visited = vec![false; self.nodes.len()];
        while let Some(node) = pending.pop() {
            if node == edge.source {
                return Err("VFX edge would create a cycle");
            }
            if !visited[node] {
                visited[node] = true;
                for edge in &self.edges {
                    if edge.source == node {
                        pending.push(edge.target);
                    }
                }
            }
        }
        self.edges.push(edge);
        Ok(())
    }

    /// Active les racines au premier appel, puis synchronise les instances.
    /// La simulation des effets élémentaires reste à la charge de l'appelant.
    /// Les événements observés sont datés avec l'âge local du graphe.
    pub(super) fn update(&mut self, dt: f32, instances: &mut Arena<VfxInstance>) {
        assert!(dt.is_finite() && dt >= 0.0);
        if self.state == VfxInstanceState::Finished {
            return;
        }
        let starting = self.state == VfxInstanceState::Waiting;
        self.state = VfxInstanceState::Active;
        if !starting {
            self.age += dt;
        }
        let mut outgoing: Vec<Vec<usize>> = vec![Vec::new(); self.nodes.len()];
        let mut incoming = vec![0_usize; self.nodes.len()];
        let mut ready_at: Vec<_> = self
            .nodes
            .iter()
            .map(|node| Some(node.start_delay))
            .collect();

        for (edge_index, edge) in self.edges.iter().enumerate() {
            outgoing[edge.source].push(edge_index);
            incoming[edge.target] += 1;
        }

        // Parcours topologique : chaque source est traitée avant ses cibles.
        // Chaque nœud et chaque arête ne sont visités qu'une fois par update.
        let mut pending: std::collections::VecDeque<_> = incoming
            .iter()
            .enumerate()
            .filter_map(|(id, count)| (*count == 0).then_some(id))
            .collect();

        while let Some(id) = pending.pop_front() {
            let node = &mut self.nodes[id];
            let activated = node.state == VfxInstanceState::Waiting
                && ready_at[id].is_some_and(|time| time <= self.age);

            if activated {
                node.state = VfxInstanceState::Active;
                node.started_at = Some(self.age);
                if let VfxNodeEffect::Effect(instance_id) = &node.effect {
                    let instance = instances
                        .get_mut(*instance_id)
                        .expect("VFX graph references a missing instance");
                    instance.state = VfxInstanceState::Active;
                    instance.age = 0.0;
                }
            }

            if matches!(
                node.state,
                VfxInstanceState::Active | VfxInstanceState::Finishing
            ) {
                let state = match &mut node.effect {
                    VfxNodeEffect::Effect(instance_id) => {
                        instances
                            .get(*instance_id)
                            .expect("VFX graph references a missing instance")
                            .state
                    }
                    VfxNodeEffect::Dag(graph) => {
                        graph.update(if activated { 0.0 } else { dt }, instances);
                        graph.state()
                    }
                };
                if matches!(
                    state,
                    VfxInstanceState::Finishing | VfxInstanceState::Finished
                ) && node.finishing_at.is_none()
                {
                    // Une fin immédiate implique aussi la fin de la phase active.
                    node.finishing_at = Some(self.age);
                }
                if state == VfxInstanceState::Finished && node.finished_at.is_none() {
                    node.finished_at = Some(self.age);
                    self.finished_effect -= 1;
                }
                node.state = state;
            }

            for &edge_index in &outgoing[id] {
                let edge = &self.edges[edge_index];
                ready_at[edge.target] = match (ready_at[edge.target], node.event_time(edge.event)) {
                    (Some(current), Some(time)) => Some(current.max(time + edge.delay_seconds)),
                    _ => None,
                };
                incoming[edge.target] -= 1;
                if incoming[edge.target] == 0 {
                    pending.push_back(edge.target);
                }
            }
        }

        if self.finished_effect == 0 {
            self.state = VfxInstanceState::Finished;
        } else if self.nodes.iter().all(|node| {
            matches!(
                node.state,
                VfxInstanceState::Finishing | VfxInstanceState::Finished
            )
        }) {
            self.state = VfxInstanceState::Finishing;
        }
    }
}
