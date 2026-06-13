# ternary-speculate

Speculative synchronization for **ternary agent coordination**. Never wait — simulate instead. Each room (agent) maintains three layers: an **execution layer** (what it's doing), a **speculation layer** (what it predicts partners will do), and a **shadow layer** (how things look from each partner's viewpoint).

## Why It Matters

Distributed systems typically coordinate via blocking (barriers, locks) or polling (heartbeat, consensus rounds). Both add latency. Speculative sync eliminates wait time by having each agent **simulate its partners** and proceed optimistically:

| Layer | Signal | Role |
|-------|--------|------|
| Execution | Actual state | What this room is doing now |
| Speculation | Predicted responses | What partners would say if asked |
| Shadow | Simulated partner models | How the world looks from each partner |

When reality arrives, shadows are **reconciled** (T+1 to T+3 ticks later). Correct predictions reinforce confidence; incorrect ones trigger re-simulation. This is essentially **speculative execution** (as in CPU branch prediction) applied to multi-agent coordination.

## How It Works

### Hint Vectors

Each room aggregates hints from four sources into a ternary signal vector:

| Hint Type | Source | Values |
|-----------|--------|--------|
| Self | Internal consistency | +1 = coherent, -1 = broken |
| Echo | Bounced output check | +1 = echoed, -1 = dropped |
| Shadow | Simulation vs reality | +1 = matched, -1 = diverged |
| Rhythm | Phase alignment | +1 = in-phase, -1 = out-of-phase |

**Aggregate score:**

```
H = mean(all_hints) ∈ [-1, +1]
```

`H < 0` triggers `needs_resimulation()`.

### Shadow Tracking

Each `Shadow` models one partner's expected state:

```
expected_position: i8     // predicted next position
expected_velocity: f64     // predicted rate of change
confidence: f64            // [0, 1] — hit rate
hits, misses: u64          // prediction accuracy
```

**Reconciliation** compares prediction to reality:

```
pos_error = |actual_position - expected_position|
was_correct = (pos_error == 0)
confidence = hits / (hits + misses)
```

**Complexity:** O(1) per shadow reconciliation.

### Simulated Responses

```
uncertainty = 0  if confidence > 0.8
            = 1  if confidence > 0.5
            = 2  otherwise
```

High-confidence shadows produce sharp predictions; low-confidence shadows express uncertainty.

### T-Minus Events

Pre-scheduled events that fire at a specific tick:

```
should_fire(t) = (t ≥ fires_at) ∧ ¬fired
t_minus(t) = fires_at - t
```

Events are self-syncing: the originating room can attach a `SimulatedResponse` speculation so partners can prepare. After firing, events are marked `reconciled` once confirmed.

### Speculation Accuracy

Overall room accuracy is the average shadow confidence:

```
speculation_accuracy = mean(confidence(s) for s in shadows)
```

This provides a single-number health metric for the speculation layer.

## Quick Start

```rust
use ternary_speculate::{SpeculativeRoom, TMinusEvent, Hint};

let mut room = SpeculativeRoom::new(0);
room.add_shadow(1);  // model partner 1
room.add_shadow(2);  // model partner 2

// Speculate what partners will do
let predictions = room.speculate_all();
assert_eq!(predictions.len(), 2);

// Reconcile with actual states
let deltas = room.reconcile_shadows(&[(1, 0, 0.5), (2, -1, 0.0)]);
// Shadows update confidence based on accuracy

// Schedule a future event
room.schedule(TMinusEvent::new(42, fires_at: 10, data: 1, room: 0));
for _ in 0..10 { room.tick(); }
let fired = room.fire_due();
assert_eq!(fired.len(), 1);
```

## API

| Type | Key Methods |
|------|-------------|
| `SpeculativeRoom` | `add_shadow(id)`, `speculate_all()`, `reconcile_shadows(actuals)`, `check_hints()`, `fire_due()`, `tick()` |
| `Shadow` | `reconcile(pos, vel)`, `simulate_response()`, `check_hint(actual)` |
| `HintVector` | `self_hint(v)`, `echo_hint(v)`, `shadow_hint(v)`, `rhythm_hint(v)`, `aggregate()` |
| `TMinusEvent` | `should_fire(tick)`, `t_minus(tick)` |

## Architecture Notes

The **γ + η = C** invariant is the design principle of the entire crate. *Generation* (γ) is the simulation layer producing predicted partner states. *Entropy* (η) is the hint divergence — when predictions are wrong, shadows accumulate misses and confidence drops (η↑). *Conservation* (C) is the invariant that every shadow must eventually be reconciled — speculative state is provisional, and the T-minus event system ensures reconciliation deadlines are met. When η exceeds the hint threshold (`has_failures()`), the room triggers re-simulation, restoring the γ-η balance.

## References

- **Speculative execution in CPUs:** Hennessy, J. & Patterson, D. *Computer Architecture* (2017), §3.3
- **Optimistic replication:** Saito, Y. & Shapiro, M. "Optimistic Replication" (2005)
- **Bayesian tracking:** Thrun, S., Burgard, W. & Fox, D. *Probabilistic Robotics* (2005)
- **Eventual consistency:** Vogels, W. "Eventually Consistent" (2009)

## License

MIT
