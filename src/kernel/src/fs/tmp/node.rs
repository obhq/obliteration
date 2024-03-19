use crate::errno::{Errno, ENOENT, ENOSPC};
use crate::fs::{Access, OpenFlags, VFile, Vnode, VnodeAttrs, VnodeType};
use crate::process::VThread;
use gmtx::{Gutex, GutexGroup, GutexWriteGuard};
use macros::Errno;
use std::collections::VecDeque;
use std::sync::{Arc, RwLock};
use thiserror::Error;

use super::{AllocVnodeError, TempFs};

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
    pub fn alloc(&self, ty: NodeType) -> Result<Arc<Node>, AllocNodeError> {
        // Check if maximum number of nodes has been reached.
        let mut list = self.list.write().unwrap();

        if list.len() >= self.max {
            return Err(AllocNodeError::LimitReached);
        }

        // TODO: Implement node creation.
        let gg = GutexGroup::new();
        let node = Arc::new(Node {
            vnode: gg.spawn(None),
            ty,
        });

        list.push_front(node.clone());

        Ok(node)
    }
}

/// An implementation of the `tmpfs_node` structure.
#[derive(Debug)]
pub struct Node {
    vnode: Gutex<Option<Arc<Vnode>>>, // tn_vnode
    ty: NodeType,                     // tn_type
}

impl Node {
    pub fn vnode_mut(&self) -> GutexWriteGuard<Option<Arc<Vnode>>> {
        self.vnode.write()
    }

    pub fn ty(&self) -> &NodeType {
        &self.ty
    }
}

/// An implementation of the `tmpfs_dirent` structure.
#[derive(Debug)]
pub struct Dirent {
    name: Box<str>,  // td_name + td_namelen
    node: Arc<Node>, // td_node
}

impl Dirent {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn node(&self) -> &Arc<Node> {
        &self.node
    }
}

#[derive(Debug)]
pub enum NodeType {
    Directory {
        is_root: bool,
        entries: Gutex<Vec<Arc<Dirent>>>,
    },
    File,
}

impl NodeType {
    pub(super) fn into_vnode_type(&self) -> VnodeType {
        match self {
            Self::Directory { is_root, .. } => VnodeType::Directory(*is_root),
            Self::File => VnodeType::File,
        }
    }
}

/// An implementation of [`crate::fs::VnodeBackend`] for tmpfs.
#[derive(Debug)]
pub struct VnodeBackend {
    tmpfs: Arc<TempFs>,
    node: Arc<Node>,
}

impl VnodeBackend {
    pub fn new(tmpfs: &Arc<TempFs>, node: Arc<Node>) -> Self {
        Self {
            tmpfs: tmpfs.clone(),
            node,
        }
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
        Ok(())
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
        vn.access(td, Access::EXEC)
            .map_err(LookupError::AccessDenied)?;

        match name {
            ".." => todo!(),
            "." => Ok(vn.clone()),
            _ => {
                let NodeType::Directory { entries, .. } = self.node.ty() else {
                    unreachable!()
                };

                let entries = entries.read();

                let dirent = entries
                    .iter()
                    .find(|dirent| dirent.name() == name)
                    .ok_or_else(|| LookupError::NoParent)?;

                let vnode = self
                    .tmpfs
                    .alloc_vnode(vn.mount(), &dirent.node())
                    .map_err(LookupError::FailedToAllocVnode)?;

                Ok(vnode)
            }
        }
    }

    fn mkdir(
        &self,
        parent: &Arc<Vnode>,
        name: &str,
        _mode: u32,
        _td: Option<&VThread>,
    ) -> Result<Arc<Vnode>, Box<dyn Errno>> {
        // The node for the newly created directory.
        let gg = GutexGroup::new();

        let node = self
            .tmpfs
            .nodes
            .alloc(NodeType::Directory {
                is_root: false,
                entries: gg.spawn(Vec::new()),
            })
            .map_err(MkDirError::FailedToAllocNode)?;

        let dirent = Dirent {
            name: name.into(),
            node: node.clone(),
        };

        let vnode = self
            .tmpfs
            .alloc_vnode(parent.mount(), &node)
            .map_err(MkDirError::FailedToAllocVnode)?;

        let NodeType::Directory { entries, .. } = self.node.ty() else {
            unreachable!()
        };

        entries.write().push(Arc::new(dirent));

        Ok(vnode)
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

/// Represents an error when [`VnodeBackend::lookup()`] fails.
#[derive(Debug, Error, Errno)]
pub enum LookupError {
    #[error("access denied")]
    AccessDenied(#[source] Box<dyn Errno>),

    #[error("tmpfs node not found")]
    #[errno(ENOENT)]
    NoParent,

    #[error("failed to alloc vnode")]
    FailedToAllocVnode(#[from] AllocVnodeError),
}

/// Represents an error when [`VnodeBackend::mkdir()`] fails.
#[derive(Debug, Error, Errno)]
pub enum MkDirError {
    #[error("couldn't allocate a node")]
    FailedToAllocNode(#[from] AllocNodeError),

    #[error("couldn't allocate a vnode")]
    FailedToAllocVnode(#[from] AllocVnodeError),
}
