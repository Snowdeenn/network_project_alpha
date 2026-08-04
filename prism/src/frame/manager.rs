use crossbeam::queue::ArrayQueue;
use std::sync::Arc;
use crate::frame::frame::Frame;

pub struct FrameManager {
    queue: Arc<ArrayQueue<Frame>>,
}

impl FrameManager {
    pub fn new() -> Self {
        Self {
            queue: Arc::new(ArrayQueue::new(2)),
        }
    }

    // Thread principal — pousse une frame prête
    pub fn push(&self, frame: Frame) {
        match self.queue.push(frame) {
         Ok(_) => (),
         Err(_t) => eprintln!("ArrayQueue de frame pleine"),   
        }
    }

    // Thread rendu — consomme la prochaine frame
    pub fn pop(&self) -> Option<Frame> {
        self.queue.pop()
    }

    // Clone l'Arc pour partager entre threads
    pub fn handle(&self) -> Arc<ArrayQueue<Frame>> {
        Arc::clone(&self.queue)
    }
}