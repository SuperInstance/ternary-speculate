# ternary-speculate

**Speculative synchronization: never wait, simulate instead.**

When two agents need to coordinate, the naive approach is: agent A sends a message, agent B receives it, B sends confirmation, A waits. This is *synchronous* — and it's slow because agents spend time waiting instead of working.

Speculative sync flips this: agent A doesn't wait for B's confirmation. Instead, it *simulates* what B would say and proceeds on that assumption. When B's actual response arrives, A checks: was my speculation correct? If yes (hint = OnTrack), no time was wasted. If no (hint = OffTrack), A rolls back and re-syncs.

This is the same architecture as CPU branch prediction, optimistic database transactions, and speculative execution in processors. Applied to multi-agent coordination.

## What's Inside

- **`Hint`** — ternary speculation feedback: `OnTrack (+1)`, `Neutral (0)`, `OffTrack (-1)`
- **`HintVector`** — hints from multiple sources: self, echo, shadow, rhythm
- **`SpeculativeLayer`** — for each room: execution state, speculation state, shadow state
- **`simulate_partners(state, room_ids)`** — predict what partners would say
- **`confirm(speculation, actual)`** — compare prediction with reality, produce Hint
- **`rollback(state, checkpoint)`** — revert to checkpoint if speculation was wrong

## Quick Example

```rust
use ternary_speculate::*;

// Agent speculates about partner's response
let my_state = 1; // I'm at +1
let partner_id = 42;

// Simulate: "given I'm at +1, partner is probably at +1 too"
let speculation = simulate_partner(my_state, partner_id);

// ... proceed with work based on speculation ...

// Later: partner's actual response arrives
let actual = -1; // partner is actually at -1

// Confirm: was I right?
let hint = confirm(speculation, actual);
assert_eq!(hint, Hint::OffTrack); // speculation was wrong

// Rollback if needed
// ... restore to checkpoint and re-sync
```

## The Deeper Truth

**Speculation is a bet on regularity.** If the system is regular (agents behave predictably), speculation is almost always right and the system runs at full speed. If the system is chaotic (agents are unpredictable), speculation fails often and the rollback cost eats the savings. The ternary hint vector captures this: if hints are mostly +1 (on track), the system is predictable and speculation pays off. If hints are mostly -1 (off track), the system is chaotic and you should fall back to synchronous coordination.

The shadow state is the deepest layer: it's how things look *from the partner's perspective*. Not just "what would partner say?" but "what does partner think *I* would say?" This recursive modeling — I model you modeling me — is the foundation of theory of mind in multi-agent systems.

**Use cases:**
- **Distributed systems** — optimistic concurrency without locks
- **Multi-agent coordination** — proceed on assumption, verify later
- **Game AI** — model opponent's strategy, act on prediction, adjust on surprise
- **Database transactions** — optimistic concurrency control
- **Network protocols** — speculative acknowledgment

## See Also

- **ternary-predict** — prediction-first perception (speculation's input)
- **ternary-sync** — Z₃ synchronization (the fallback when speculation fails)
- **ternary-trust** — trust determines when speculation is safe
- **ternary-room** — the room model that speculative agents inhabit

## Install

```bash
cargo add ternary-speculate
```

## License

MIT
