use crate::errno::{Errno, ENOSPC};
use crate::fs::{Access, OpenFlags, VFile, Vnode, VnodeAttrs};
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

/// An implementation of [`crate::fs::VnodeBackend`] for tmpfs.
#[derive(Debug)]
pub struct VnodeBackend {
    node: Arc<Node>,
}

impl VnodeBackend {
    pub fn new(node: Arc<Node>) -> Self {
        Self { node }
    }
}

impl crate::fs::VnodeBackend for VnodeBackend {
    #[allow(unused_variables)] // TODO: remove when implementing
    fn access(
        &self,
        vn: &Arc<Vnode>,
        td: Option<&VThread>,
        mode: Access,
    ) -> Result<(), Box<dyn Errno>> {
        todo!()
    }

    #[allow(unused_variables)] // TODO: remove when implementing
    fn getattr(&self, vn: &Arc<Vnode>) -> Result<VnodeAttrs, Box<dyn Errno>> {
        todo!()
    }

    #[allow(unused_variables)] // TODO: remove when implementing
    fn lookup(
        &self,
        vn: &Arc<Vnode>,
        td: Option<&VThread>,
        name: &str,
    ) -> Result<Arc<Vnode>, Box<dyn Errno>> {
        match name {
            ".." => todo!(),
            "." => todo!(),
            _ => {
                todo!()
            }
        }
    }

    #[allow(unused_variables)] // TODO: remove when implementing
    fn mkdir(
        &self,
        parent: &Arc<Vnode>,
        name: &str,
        mode: u32,
        td: Option<&VThread>,
    ) -> Result<Arc<Vnode>, Box<dyn Errno>> {
        todo!()
    }

    #[allow(unused_variables)] // TODO: remove when implementing
    fn open(
        &self,
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
