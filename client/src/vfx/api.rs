use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;

use super::VfxSpawnParams;
use super::request::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueueFull;

impl std::fmt::Display for QueueFull {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("VFX request queue or unread results are full")
    }
}

impl std::error::Error for QueueFull {}

pub(super) struct Requests {
    pub queue: VecDeque<VfxRequest>,
    // Les résultats non consommés occupent aussi une place : mémoire bornée,
    // sans perdre silencieusement un résultat promis à l'appelant.
    pub results: HashMap<SpawnTicket, Option<SpawnResult>>,
    pub command_results: HashMap<CommandTicket, Option<Result<(), VfxCommandError>>>,
    pub capacity: usize,
    pub next_ticket: u64,
    pub rejected: u64,
}

/// Façade clonable pour les systèmes locaux, le réseau et les animations.
/// Elle partage uniquement la file ; seul VfxSystem possède la simulation.
#[derive(Clone)]
pub struct VfxApi {
    pub(super) requests: Rc<RefCell<Requests>>,
}

impl VfxApi {
    pub(super) fn new(capacity: usize) -> Self {
        Self {
            requests: Rc::new(RefCell::new(Requests {
                queue: VecDeque::new(),
                results: HashMap::new(),
                command_results: HashMap::new(),
                capacity,
                next_ticket: 0,
                rejected: 0,
            })),
        }
    }

    pub fn spawn(
        &self,
        effect: impl Into<String>,
        params: VfxSpawnParams,
    ) -> Result<SpawnTicket, QueueFull> {
        self.spawn_with(effect, params, SpawnOptions::default())
    }

    pub fn spawn_with(
        &self,
        effect: impl Into<String>,
        params: VfxSpawnParams,
        options: SpawnOptions,
    ) -> Result<SpawnTicket, QueueFull> {
        let mut requests = self.requests.borrow_mut();
        if requests.queue.len() >= requests.capacity
            || requests.results.len() + requests.command_results.len() >= requests.capacity
        {
            requests.rejected += 1;
            return Err(QueueFull);
        }
        let ticket = SpawnTicket(requests.next_ticket);
        requests.next_ticket = requests
            .next_ticket
            .checked_add(1)
            .expect("VFX tickets exhausted");
        requests.results.insert(ticket, None);
        requests.queue.push_back(VfxRequest::Spawn {
            ticket: Some(ticket),
            effect: effect.into(),
            params,
            options,
        });
        Ok(ticket)
    }

    /// Consomme le résultat. Un ticket en attente reste consultable plus tard.
    pub fn take_result(&self, ticket: SpawnTicket) -> Option<SpawnResult> {
        let mut requests = self.requests.borrow_mut();
        let result = requests.results.get(&ticket).copied().flatten()?;
        requests.results.remove(&ticket);
        Some(result)
    }

    /// Submit a visual event without retaining an unread result ticket.
    pub fn spawn_detached(
        &self,
        effect: impl Into<String>,
        params: VfxSpawnParams,
        options: SpawnOptions,
    ) -> Result<(), QueueFull> {
        self.enqueue(VfxRequest::Spawn {
            ticket: None,
            effect: effect.into(),
            params,
            options,
        })
    }

    pub fn stop_named(&self, effect: impl Into<String>) -> Result<(), QueueFull> {
        self.enqueue(VfxRequest::StopNamed(effect.into()))
    }

    /// Annulation directe dans la file : fonctionne même lorsqu'elle est pleine.
    pub fn cancel_pending(&self, ticket: SpawnTicket) -> bool {
        let mut requests = self.requests.borrow_mut();
        let Some(index) = requests.queue.iter().position(|request| {
            matches!(request, VfxRequest::Spawn { ticket: Some(queued), .. } if *queued == ticket)
        }) else { return false; };
        requests.queue.remove(index);
        requests
            .results
            .insert(ticket, Some(SpawnResult::Rejected(VfxRejection::Cancelled)));
        true
    }

    pub fn update(&self, handle: VfxHandle, change: VfxChange) -> Result<CommandTicket, QueueFull> {
        self.command(|ticket| VfxRequest::Update(ticket, handle, change))
    }

    pub fn stop(&self, handle: VfxHandle, mode: StopMode) -> Result<CommandTicket, QueueFull> {
        self.command(|ticket| VfxRequest::Stop(ticket, handle, mode))
    }

    pub fn take_command_result(
        &self,
        ticket: CommandTicket,
    ) -> Option<Result<(), VfxCommandError>> {
        let mut requests = self.requests.borrow_mut();
        let result = requests.command_results.get(&ticket).copied().flatten()?;
        requests.command_results.remove(&ticket);
        Some(result)
    }

    fn command(
        &self,
        build: impl FnOnce(CommandTicket) -> VfxRequest,
    ) -> Result<CommandTicket, QueueFull> {
        let mut requests = self.requests.borrow_mut();
        if requests.queue.len() >= requests.capacity
            || requests.results.len() + requests.command_results.len() >= requests.capacity
        {
            requests.rejected += 1;
            return Err(QueueFull);
        }
        let ticket = CommandTicket(requests.next_ticket);
        requests.next_ticket = requests
            .next_ticket
            .checked_add(1)
            .expect("VFX tickets exhausted");
        requests.command_results.insert(ticket, None);
        requests.queue.push_back(build(ticket));
        Ok(ticket)
    }

    pub fn reconcile(&self, prediction: PredictionKey, confirmed: bool) -> Result<(), QueueFull> {
        self.enqueue(VfxRequest::Reconcile {
            prediction,
            confirmed,
        })
    }

    pub(super) fn enqueue(&self, request: VfxRequest) -> Result<(), QueueFull> {
        let mut requests = self.requests.borrow_mut();
        if requests.queue.len() >= requests.capacity {
            requests.rejected += 1;
            return Err(QueueFull);
        }
        requests.queue.push_back(request);
        Ok(())
    }
}
