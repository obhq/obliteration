use std::convert::Infallible;

pub use self::entry::*;

mod entry;

/// An implementation of `sys/kern/orbis_idt.c`.
#[derive(Debug)]
pub struct Idt<T> {
    sets: Vec<[Option<Entry<T>>; 0x80]>,
    next: usize,
    limit: usize,
}

impl<T> Idt<T> {
    const NONE: Option<Entry<T>> = None;

    /// See `_id_table_create` on the PS4 for a reference.
    pub fn new(limit: usize) -> Self {
        assert_ne!(limit, 0);

        // Allocate the first set.
        let sets = vec![[Self::NONE; 0x80]];

        Self {
            sets,
            next: 0,
            limit,
        }
    }

    pub fn alloc_infallible<F>(&mut self, factory: F) -> usize
    where
        F: FnOnce(usize) -> Entry<T>,
    {
        let Ok((_, id)) = self.alloc::<_, Infallible>(|id| Ok(factory(id))) else {
            unreachable!();
        };

        id
    }

    /// See `id_alloc` on the PS4 for a reference.
    pub fn alloc<F, E>(&mut self, factory: F) -> Result<(&mut Entry<T>, usize), E>
    where
        F: FnOnce(usize) -> Result<Entry<T>, E>,
    {
        // Allocate a new set if necessary.
        let id = self.next;
        let set = id / 0x80;

        while set >= self.sets.len() {
            todo!("id_alloc with entries span across the first set");
        }

        // Get the entry.
        let set = &mut self.sets[set];
        let entry = &mut set[id % 0x80];

        assert!(entry.is_none());

        // Set the value.
        let value = entry.insert(factory(id)?);

        // Update table states.
        self.next += 1;

        Ok((value, id))
    }

    /// See `id_rlock` on the PS4 for a reference.
    pub fn get_mut(&mut self, id: usize, ty: Option<u16>) -> Option<&mut Entry<T>> {
        if id >= 0x10000 {
            return None;
        }

        let i = id & 0x1fff;
        let set = self.sets.get_mut(i / 0x80)?;
        let entry = set[i % 0x80].as_mut()?;

        if let Some(ty) = ty {
            if entry.ty() != ty {
                return None;
            }
        }

        Some(entry)
    }
}
