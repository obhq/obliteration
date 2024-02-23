use crate::errno::{Errno, ENOSPC};
use crate::fs::{Access, OpenFlags, VFile, Vnode, VnodeAttrs, VnodeBackend};
use crate::process::VThread;
use macros::Errno;
use std::collections::VecDeque;
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

impl VnodeBackend for Node {
    #[allow(unused_variables)] // TODO: remove when implementing
    fn access(
        self: Arc<Self>,
        vn: &Arc<Vnode>,
        td: Option<&VThread>,
        mode: Access,
    ) -> Result<(), Box<dyn Errno>> {
        todo!()
    }

    #[allow(unused_variables)] // TODO: remove when implementing
    fn getattr(self: Arc<Self>, vn: &Arc<Vnode>) -> Result<VnodeAttrs, Box<dyn Errno>> {
        todo!()
    }

    #[allow(unused_variables)] // TODO: remove when implementing
    fn lookup(
        self: Arc<Self>,
        vn: &Arc<Vnode>,
        td: Option<&VThread>,
        name: &str,
    ) -> Result<Arc<Vnode>, Box<dyn Errno>> {
        todo!()
    }

    #[allow(unused_variables)] // TODO: remove when implementing
    fn mkdir(
        self: Arc<Self>,
        parent: &Arc<Vnode>,
        name: &str,
        mode: u32,
        td: Option<&VThread>,
    ) -> Result<Arc<Vnode>, Box<dyn Errno>> {
        todo!()
    }

    #[allow(unused_variables)] // TODO: remove when implementing
    fn open(
        self: Arc<Self>,
        vn: &Arc<Vnode>,
        td: Option<&VThread>,
        mode: OpenFlags,
        #[allow(unused_variables)] file: Option<&mut VFile>,
    ) -> Result<(), Box<dyn Errno>> {
        todo!()
    }
}

/// Represents an error when [`Nodes::alloc()`] fails.
#[derive(Debug, Error, Errno)]
pub enum AllocNodeError {
    #[error("maximum number of nodes has been reached")]
    #[errno(ENOSPC)]
    LimitReached,
}
