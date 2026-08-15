// Absorbed from varisat 0.2.2 (https://github.com/jix/varisat, master
// @ 33e87693, file varisat/src/clause/alloc.rs).
// Copyright (c) 2017-2019 Jannis Harder, MIT OR Apache-2.0 — ferroplan's
// exact dual license. Ferroplan code from the absorption commit onward;
// see ATTRIBUTION.md.

//! Clause allocator.
use std::{mem::transmute, slice};

use crate::lit::{Lit, LitIdx};

use super::{Clause, ClauseHeader, HEADER_LEN};

/// Integer type used to store offsets into [`ClauseAlloc`]'s memory.
type ClauseOffset = u32;

/// Bump allocator for clause storage.
///
/// Clauses are allocated from a single continuous buffer. Clauses cannot be freed individually. To
/// reclaim space from deleted clauses, a new `ClauseAlloc` is created and the remaining clauses are
/// copied over.
///
/// When the `ClauseAlloc`'s buffer is full, it is reallocated using the growing strategy of
/// [`Vec`]. External references ([`ClauseRef`]) store an offset into the `ClauseAlloc`'s memory and
/// remain valid when the buffer is grown. Clauses are aligned and the offset represents a multiple
/// of the alignment size. This allows using 32-bit offsets while still supporting up to 16GB of
/// clauses.
#[derive(Default)]
pub struct ClauseAlloc {
    buffer: Vec<LitIdx>,
}

impl ClauseAlloc {
    /// Create a clause allocator with preallocated capacity.
    pub fn with_capacity(capacity: usize) -> ClauseAlloc {
        ClauseAlloc {
            buffer: Vec::with_capacity(capacity),
        }
    }

    /// Allocate space for and add a new clause.
    ///
    /// Clauses have a minimal size of 3, as binary and unit clauses are handled separately. This is
    /// enforced on the ClauseAlloc level to safely avoid extra bound checks when accessing the
    /// initial literals of a clause.
    ///
    /// The size of the header will be set to the size of the given slice. The returned
    /// [`ClauseRef`] can be used to access the new clause.
    pub fn add_clause(&mut self, mut header: ClauseHeader, lits: &[Lit]) -> ClauseRef {
        let offset = self.buffer.len();

        assert!(
            lits.len() >= 3,
            "ClauseAlloc can only store ternary and larger clauses"
        );

        assert!(
            offset <= (ClauseRef::max_offset() as usize),
            "Exceeded ClauseAlloc's maximal buffer size"
        );

        header.set_len(lits.len());

        self.buffer.extend_from_slice(&header.data);

        let lit_idx_slice = unsafe {
            // This is safe as Lit and LitIdx have the same representation
            slice::from_raw_parts(lits.as_ptr() as *const LitIdx, lits.len())
        };

        self.buffer.extend_from_slice(lit_idx_slice);

        ClauseRef {
            offset: offset as ClauseOffset,
        }
    }

    /// Access the header of a clause.
    pub fn header(&self, cref: ClauseRef) -> &ClauseHeader {
        let offset = cref.offset as usize;
        assert!(
            offset + HEADER_LEN <= self.buffer.len(),
            "ClauseRef out of bounds"
        );
        unsafe { self.header_unchecked(cref) }
    }

    /// Mutate the header of a clause.
    pub fn header_mut(&mut self, cref: ClauseRef) -> &mut ClauseHeader {
        let offset = cref.offset as usize;
        assert!(
            offset + HEADER_LEN <= self.buffer.len(),
            "ClauseRef out of bounds"
        );
        unsafe { self.header_unchecked_mut(cref) }
    }

    unsafe fn header_unchecked(&self, cref: ClauseRef) -> &ClauseHeader {
        let offset = cref.offset as usize;
        let header_pointer = self.buffer.as_ptr().add(offset) as *const ClauseHeader;
        &*header_pointer
    }

    /// Mutate the header of a clause without bound checks.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `cref` points at a live clause, e.g. via
    /// [`check_bounds`](ClauseAlloc::check_bounds).
    pub unsafe fn header_unchecked_mut(&mut self, cref: ClauseRef) -> &mut ClauseHeader {
        let offset = cref.offset as usize;
        let header_pointer = self.buffer.as_mut_ptr().add(offset) as *mut ClauseHeader;
        &mut *header_pointer
    }

    /// Access a clause.
    pub fn clause(&self, cref: ClauseRef) -> &Clause {
        let header = self.header(cref);
        let len = header.len();

        // Even on 32 bit systems these additions can't overflow as we never create clause refs with
        // an offset larger than ClauseRef::max_offset()

        let lit_offset = cref.offset as usize + HEADER_LEN;
        let lit_end = lit_offset + len;
        assert!(lit_end <= self.buffer.len(), "ClauseRef out of bounds");
        unsafe { self.clause_with_len_unchecked(cref, len) }
    }

    /// Mutate a clause.
    pub fn clause_mut(&mut self, cref: ClauseRef) -> &mut Clause {
        let header = self.header(cref);
        let len = header.len();

        // Even on 32 bit systems these additions can't overflow as we never create clause refs with
        // an offset larger than ClauseRef::max_offset()

        let lit_offset = cref.offset as usize + HEADER_LEN;
        let lit_end = lit_offset + len;
        assert!(lit_end <= self.buffer.len(), "ClauseRef out of bounds");
        unsafe { self.clause_with_len_unchecked_mut(cref, len) }
    }

    /// Mutate the literals of a clause without bound checks.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `cref` points at a live clause whose literals stay inside the
    /// buffer, e.g. via [`check_bounds`](ClauseAlloc::check_bounds).
    pub unsafe fn lits_ptr_mut_unchecked(&mut self, cref: ClauseRef) -> *mut Lit {
        let offset = cref.offset as usize;
        self.buffer.as_ptr().add(offset + HEADER_LEN) as *mut Lit
    }

    /// Perform a manual bound check on a ClauseRef assuming a given clause length.
    pub fn check_bounds(&self, cref: ClauseRef, len: usize) {
        // Even on 32 bit systems these additions can't overflow as we never create clause refs with
        // an offset larger than ClauseRef::max_offset()

        let lit_offset = cref.offset as usize + HEADER_LEN;
        let lit_end = lit_offset + len;
        assert!(lit_end <= self.buffer.len(), "ClauseRef out of bounds");
    }

    unsafe fn clause_with_len_unchecked(&self, cref: ClauseRef, len: usize) -> &Clause {
        let offset = cref.offset as usize;
        #[allow(clippy::transmute_ptr_to_ptr)]
        transmute::<&[LitIdx], &Clause>(slice::from_raw_parts(
            self.buffer.as_ptr().add(offset),
            len + HEADER_LEN,
        ))
    }

    unsafe fn clause_with_len_unchecked_mut(&mut self, cref: ClauseRef, len: usize) -> &mut Clause {
        let offset = cref.offset as usize;
        #[allow(clippy::transmute_ptr_to_ptr)]
        transmute::<&mut [LitIdx], &mut Clause>(slice::from_raw_parts_mut(
            self.buffer.as_mut_ptr().add(offset),
            len + HEADER_LEN,
        ))
    }

    /// Current buffer size in multiples of [`LitIdx`].
    pub fn buffer_size(&self) -> usize {
        self.buffer.len()
    }
}

/// Compact reference to a clause.
///
/// Used with [`ClauseAlloc`] to access the clause.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct ClauseRef {
    offset: ClauseOffset,
}

impl ClauseRef {
    /// The largest offset supported by the ClauseAlloc
    const fn max_offset() -> ClauseOffset {
        // Make sure we can safely add a length to an offset without overflowing usize
        ((usize::MAX >> 1) & (ClauseOffset::MAX as usize)) as ClauseOffset
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lit::Lit;

    fn clause(lits: &[isize]) -> Vec<Lit> {
        lits.iter().map(|&l| Lit::from_dimacs(l)).collect()
    }

    #[test]
    fn roundtrip_and_mutation() {
        let mut alloc = ClauseAlloc::default();
        let input: Vec<Vec<Lit>> = vec![
            clause(&[1, 2, 3]),
            clause(&[4, -5, 6, -7]),
            clause(&[-1, -2, -3, -4, -5]),
        ];

        let crefs: Vec<ClauseRef> = input
            .iter()
            .map(|lits| alloc.add_clause(ClauseHeader::new(), lits))
            .collect();

        for (&cref, lits) in crefs.iter().zip(input.iter()) {
            let stored = alloc.clause(cref);
            assert_eq!(stored.header().len(), lits.len());
            assert_eq!(stored.lits(), lits.as_slice());
        }

        alloc.clause_mut(crefs[1]).lits_mut().reverse();
        let reversed: Vec<Lit> = input[1].iter().rev().cloned().collect();
        assert_eq!(alloc.clause(crefs[1]).lits(), reversed.as_slice());

        // Shrinking a clause in place keeps the prefix.
        let len = alloc.clause(crefs[2]).lits().len();
        alloc.header_mut(crefs[2]).set_len(len - 1);
        assert_eq!(alloc.clause(crefs[2]).lits(), &input[2][..len - 1]);
    }
}
