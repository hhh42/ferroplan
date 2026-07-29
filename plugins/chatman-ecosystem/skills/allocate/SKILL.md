---
name: allocate
description: Run the Chatman Multifractal Cascade Allocator over exactly eight admitted work surfaces and bind a replayable allocation receipt. Use after observation admission and before Ferroplan planning, including recursive local frontiers.
context: fork
agent: chatman-ecosystem:cmca-allocator
effort: high
---

Allocate scarce capacity for `$ARGUMENTS`.

Require:

- an admitted observation frontier;
- exactly eight candidates;
- the canonical ten-factor order from `profiles/work-surfaces.json`;
- an acyclic parent forest;
- explicit projection laws and uncertainty;
- the pinned BCINR-CMCA revision;
- a parent allocation receipt when this frontier descends from an allocated node.

1. Call `cmca_allocate` with the exact candidates.
2. Call `bind_allocation_receipt` with candidates, allocation result, observation frontier, predecessor receipt, and parent allocation receipt when recursive.
3. Call `verify_receipt` on the exact envelope.
4. Return shares, digests, BCINR revision, receipt, selected subtree, required return consequence, uncertainty, and typed refusals.

Every local allocation remains eight nodes. Do not flatten recursive frontiers into an unbounded candidate array.

Do not plan or execute work. Allocation standing is not task order, validation, or actuation authority.
