use crate::errno::{Errno, ENOSPC};
use std::collections::VecDeque;
use std::num::NonZeroI32;
use std::sync::{Arc, RwLock};
use thiserror::Error;

/// A collection of [`Node`].
#[derive(Debug)]
pub struct Nodes {
    max: usize,                        // tm_nodes_max
    list: RwLock<VecDeque<Arc<Node>>>, // tm_nodes_used + tm_nodes_inuse
}

impl Nodes {
    pub fn new(max: usize) -> Self {
        Self {
            max,
            list: RwLock::default(),
        }
    }

    /// See `tmpfs_alloc_node` on the PS4 for a reference.
    pub fn alloc(&self) -> Result<Arc<Node>, AllocNodeError> {
        // Check if maximum number of nodes has been reached.
        let mut list = self.list.write().unwrap();

        if list.len() >= self.max {
            return Err(AllocNodeError::LimitReached);
        }

        // TODO: Implement node creation.
        let node = Arc::new(Node {});

        list.push_front(node.clone());

        Ok(node)
    }
}

/// An implementation of `tmpfs_node` structure.
#[derive(Debug)]
pub struct Node {}

/// Represents an error when [`Nodes::alloc()`] fails.
#[derive(Debug, Error)]
pub enum AllocNodeError {
    #[error("maximum number of nodes has been reached")]
    LimitReached,
}

impl Errno for AllocNodeError {
    fn errno(&self) -> NonZeroI32 {
        match self {
            Self::LimitReached => ENOSPC,
        }
    }
}
