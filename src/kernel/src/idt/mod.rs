pub use self::entry::*;

mod entry;

/// An implementation of `sys/kern/orbis_idt.c`.
#[derive(Debug)]
pub struct IdTable<T> {
    sets: Vec<[Option<IdEntry<T>>; 0x80]>,
    next: usize,
    limit: usize,
}

impl<T> IdTable<T> {
    /// See `_id_table_create` on the PS4 for a reference.
    pub fn new(limit: usize) -> Self {
        assert_ne!(limit, 0);

        // Allocate the first set.
        let mut sets = Vec::with_capacity(1);

        sets.push(std::array::from_fn(|_| None));

        Self {
            sets,
            next: 0,
            limit,
        }
    }

    /// See `id_alloc` on the PS4 for a reference.
    pub fn alloc<F, E>(&mut self, factory: F) -> Result<(&mut IdEntry<T>, usize), E>
    where
        F: FnOnce(usize) -> Result<T, E>,
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
        let value = entry.insert(IdEntry::new(factory(id)?));

        // Update table states.
        self.next += 1;

        Ok((value, id))
    }
}
