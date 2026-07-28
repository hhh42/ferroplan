---
name: allocate
description: Run the Chatman Multifractal Cascade Allocator over exactly eight admitted repository work surfaces and bind a replayable allocation receipt. Use after semantic admission and before Ferroplan planning.
context: fork
agent: chatman-ecosystem:cmca-allocator
effort: high
---

Allocate scarce capacity for `$ARGUMENTS`.

Require an admitted observation frontier and exactly eight candidates in the factor order declared by `profiles/work-surfaces.json`.

1. Call `cmca_allocate` with the exact candidates.
2. Call `bind_allocation_receipt` with candidates, allocation result, observation frontier, and predecessor receipt.
3. Call `verify_receipt` on the envelope.
4. Return exact shares, digests, BCINR revision, receipt, and refusals.

Do not plan or execute work. Allocation standing is not actuation authority.
