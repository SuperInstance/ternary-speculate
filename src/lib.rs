#![forbid(unsafe_code)]
//! Speculative sync — never wait, simulate instead.
//!
//! Each room (agent) has three layers:
//! - Execution: what it's actually doing
//! - Speculation: what it thinks partners would say
//! - Shadow: how things look from each partner's viewpoint

use std::collections::HashMap;

/// A room identifier.
pub type RoomId = usize;

/// Ternary hint signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hint { OnTrack = 1, Neutral = 0, OffTrack = -1 }

impl Hint { pub fn to_i8(self) -> i8 { self as i8 } }

/// Vector of hints from multiple sources.
#[derive(Debug, Clone, Default)]
pub struct HintVector {
    pub self_hints: Vec<i8>,
    pub echo_hints: Vec<i8>,
    pub shadow_hints: Vec<i8>,
    pub rhythm_hints: Vec<i8>,
}

impl HintVector {
    pub fn new() -> Self { Self::default() }

    /// Aggregate hint score: +1 = all clear, 0 = neutral, -1 = problem.
    pub fn aggregate(&self) -> f64 {
        let all: Vec<i8> = self.self_hints.iter()
            .chain(self.echo_hints.iter())
            .chain(self.shadow_hints.iter())
            .chain(self.rhythm_hints.iter())
            .copied().collect();
        if all.is_empty() { return 0.0; }
        all.iter().map(|&v| v as f64).sum::<f64>() / all.len() as f64
    }

    /// Any off-track hints?
    pub fn has_failures(&self) -> bool {
        let check = |v: &i8| *v == -1;
        self.self_hints.iter().any(&check) || self.echo_hints.iter().any(&check)
            || self.shadow_hints.iter().any(&check) || self.rhythm_hints.iter().any(&check)
    }

    /// Number of hints.
    pub fn len(&self) -> usize {
        self.self_hints.len() + self.echo_hints.len() + self.shadow_hints.len() + self.rhythm_hints.len()
    }

    pub fn is_empty(&self) -> bool { self.len() == 0 }

    /// Add a self-hint (internal consistency).
    pub fn self_hint(&mut self, h: i8) { self.self_hints.push(h.clamp(-1, 1)); }
    /// Add an echo-hint (bounced output check).
    pub fn echo_hint(&mut self, h: i8) { self.echo_hints.push(h.clamp(-1, 1)); }
    /// Add a shadow-hint (simulation vs reality).
    pub fn shadow_hint(&mut self, h: i8) { self.shadow_hints.push(h.clamp(-1, 1)); }
    /// Add a rhythm-hint (phase alignment).
    pub fn rhythm_hint(&mut self, h: i8) { self.rhythm_hints.push(h.clamp(-1, 1)); }
}

/// Shadow state — a room's model of a partner.
#[derive(Debug, Clone)]
pub struct Shadow {
    pub partner: RoomId,
    pub expected_position: i8,
    pub expected_velocity: f64,
    pub confidence: f64,
    pub last_delta: Option<ShadowDelta>,
    pub hits: u64,
    pub misses: u64,
}

impl Shadow {
    pub fn new(partner: RoomId) -> Self {
        Self { partner, expected_position: 0, expected_velocity: 0.0, confidence: 0.5, last_delta: None, hits: 0, misses: 0 }
    }

    /// Update shadow with actual partner state. Returns delta.
    pub fn reconcile(&mut self, actual_position: i8, actual_velocity: f64) -> ShadowDelta {
        let pos_delta = (actual_position - self.expected_position).abs();
        let vel_delta = (actual_velocity - self.expected_velocity).abs();
        let delta = ShadowDelta {
            partner: self.partner,
            position_error: pos_delta,
            velocity_error: vel_delta,
            was_correct: pos_delta == 0,
        };

        if delta.was_correct { self.hits += 1; } else { self.misses += 1; }
        let total = self.hits + self.misses;
        self.confidence = if total > 0 { self.hits as f64 / total as f64 } else { 0.5 };
        self.expected_position = actual_position;
        self.expected_velocity = actual_velocity;
        self.last_delta = Some(delta.clone());
        delta
    }

    /// Simulate what the partner would respond.
    pub fn simulate_response(&self) -> SimulatedResponse {
        let noise = if self.confidence > 0.8 { 0 } else if self.confidence > 0.5 { 1 } else { 2 };
        SimulatedResponse {
            partner: self.partner,
            predicted_position: self.expected_position,
            confidence: self.confidence,
            uncertainty: noise,
        }
    }

    /// Check shadow hint — is my simulation consistent with reality?
    pub fn check_hint(&self, actual: i8) -> i8 {
        if actual == self.expected_position { 1 }
        else if (actual - self.expected_position).abs() == 1 { 0 }
        else { -1 }
    }
}

/// Delta between simulation and reality.
#[derive(Debug, Clone)]
pub struct ShadowDelta {
    pub partner: RoomId,
    pub position_error: i8,
    pub velocity_error: f64,
    pub was_correct: bool,
}

/// A simulated response from a partner room.
#[derive(Debug, Clone)]
pub struct SimulatedResponse {
    pub partner: RoomId,
    pub predicted_position: i8,
    pub confidence: f64,
    pub uncertainty: usize, // 0 = sure, 1 = maybe, 2 = guessing
}

/// T-minus event — pre-scheduled, self-syncing.
#[derive(Debug, Clone)]
pub struct TMinusEvent {
    pub id: u64,
    pub fires_at: u64,        // Absolute tick when event fires
    pub data: i8,             // Ternary payload
    pub room: RoomId,         // Origin room
    pub speculation: Option<SimulatedResponse>,
    pub fired: bool,
    pub reconciled: bool,
}

impl TMinusEvent {
    pub fn new(id: u64, fires_at: u64, data: i8, room: RoomId) -> Self {
        Self { id, fires_at, data, room, speculation: None, fired: false, reconciled: false }
    }

    /// Is it time to fire?
    pub fn should_fire(&self, current_tick: u64) -> bool {
        current_tick >= self.fires_at && !self.fired
    }

    /// Time until fire.
    pub fn t_minus(&self, current_tick: u64) -> i64 {
        self.fires_at as i64 - current_tick as i64
    }
}

/// A speculative room — the full three-layer architecture.
pub struct SpeculativeRoom {
    pub id: RoomId,
    pub position: i8,
    pub velocity: f64,
    pub shadows: HashMap<RoomId, Shadow>,
    pub hints: HintVector,
    pub scheduled: Vec<TMinusEvent>,
    pub tick: u64,
    pub speculation_accuracy: f64,
}

impl SpeculativeRoom {
    pub fn new(id: RoomId) -> Self {
        Self {
            id, position: 0, velocity: 0.0,
            shadows: HashMap::new(), hints: HintVector::new(),
            scheduled: Vec::new(), tick: 0, speculation_accuracy: 0.5,
        }
    }

    /// Add a partner shadow.
    pub fn add_shadow(&mut self, partner: RoomId) {
        self.shadows.insert(partner, Shadow::new(partner));
    }

    /// Simulate all partners' responses (speculation layer).
    pub fn speculate_all(&self) -> Vec<SimulatedResponse> {
        self.shadows.values().map(|s| s.simulate_response()).collect()
    }

    /// Check all hints.
    pub fn check_hints(&mut self) -> f64 {
        // Self-hint: am I internally consistent?
        self.hints.self_hint(if self.position.abs() <= 1 { 1 } else { -1 });
        self.hints.aggregate()
    }

    /// Check shadow hints against actual partner states.
    pub fn check_shadows(&mut self, actuals: &[(RoomId, i8)]) {
        for &(partner, actual) in actuals {
            if let Some(shadow) = self.shadows.get(&partner) {
                self.hints.shadow_hint(shadow.check_hint(actual));
            }
        }
    }

    /// Reconcile shadows with reality (T+1 to T+3).
    pub fn reconcile_shadows(&mut self, actuals: &[(RoomId, i8, f64)]) -> Vec<ShadowDelta> {
        let mut deltas = Vec::new();
        for &(partner, pos, vel) in actuals {
            if let Some(shadow) = self.shadows.get_mut(&partner) {
                deltas.push(shadow.reconcile(pos, vel));
            }
        }
        // Update overall speculation accuracy
        let total: f64 = self.shadows.values().map(|s| s.confidence).sum();
        let count = self.shadows.len().max(1);
        self.speculation_accuracy = total / count as f64;
        deltas
    }

    /// Schedule a T-minus event.
    pub fn schedule(&mut self, event: TMinusEvent) {
        self.scheduled.push(event);
    }

    /// Fire any events that are due.
    pub fn fire_due(&mut self) -> Vec<&TMinusEvent> {
        for event in &mut self.scheduled {
            if event.should_fire(self.tick) {
                event.fired = true;
            }
        }
        self.scheduled.iter().filter(|e| e.fired && !e.reconciled).collect()
    }

    /// Advance one tick.
    pub fn tick(&mut self) {
        self.tick += 1;
    }

    /// Should I re-simulate? (hint score negative)
    pub fn needs_resimulation(&self) -> bool { self.hints.has_failures() }

    /// Confidence in current speculation.
    pub fn speculation_confidence(&self) -> f64 { self.speculation_accuracy }

    /// How many shadow predictions have been correct.
    pub fn shadow_accuracy(&self) -> (u64, u64) {
        let hits: u64 = self.shadows.values().map(|s| s.hits).sum();
        let total = hits + self.shadows.values().map(|s| s.misses).sum::<u64>();
        (hits, total)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test] fn test_hint_values() { assert_eq!(Hint::OnTrack.to_i8(), 1); assert_eq!(Hint::Neutral.to_i8(), 0); assert_eq!(Hint::OffTrack.to_i8(), -1); }
    #[test] fn test_hint_vector_empty() { let hv = HintVector::new(); assert!(hv.is_empty()); assert_eq!(hv.aggregate(), 0.0); }
    #[test] fn test_hint_vector_positive() { let mut hv = HintVector::new(); hv.self_hint(1); hv.echo_hint(1); assert!(hv.aggregate() > 0.5); }
    #[test] fn test_hint_vector_negative() { let mut hv = HintVector::new(); hv.self_hint(-1); assert!(hv.aggregate() < 0.0); }
    #[test] fn test_hint_vector_mixed() { let mut hv = HintVector::new(); hv.self_hint(1); hv.self_hint(-1); assert!(hv.aggregate().abs() < 0.5); }
    #[test] fn test_hint_failures() { let mut hv = HintVector::new(); hv.self_hint(1); assert!(!hv.has_failures()); hv.shadow_hint(-1); assert!(hv.has_failures()); }
    #[test] fn test_shadow_creation() { let s = Shadow::new(1); assert_eq!(s.partner, 1); assert_eq!(s.confidence, 0.5); }
    #[test] fn test_shadow_reconcile_correct() { let mut s = Shadow::new(1); s.expected_position = 1; let d = s.reconcile(1, 0.5); assert!(d.was_correct); assert_eq!(s.confidence, 1.0); }
    #[test] fn test_shadow_reconcile_wrong() { let mut s = Shadow::new(1); s.expected_position = 1; let d = s.reconcile(-1, 0.0); assert!(!d.was_correct); assert!(s.confidence < 1.0); }
    #[test] fn test_shadow_simulate() { let s = Shadow::new(1); let resp = s.simulate_response(); assert_eq!(resp.partner, 1); assert!(resp.confidence >= 0.0); }
    #[test] fn test_shadow_check_hint_match() { let mut s = Shadow::new(1); s.expected_position = 1; assert_eq!(s.check_hint(1), 1); }
    #[test] fn test_shadow_check_hint_near() { let mut s = Shadow::new(1); s.expected_position = 1; assert_eq!(s.check_hint(0), 0); }
    #[test] fn test_shadow_check_hint_far() { let mut s = Shadow::new(1); s.expected_position = 1; assert_eq!(s.check_hint(-1), -1); }
    #[test] fn test_shadow_accuracy_improves() { let mut s = Shadow::new(1); s.expected_position = 1; s.reconcile(1, 0.0); s.reconcile(1, 0.0); assert!(s.confidence > 0.8); }
    #[test] fn test_tminus_creation() { let e = TMinusEvent::new(0, 10, 1, 0); assert_eq!(e.t_minus(5), 5); assert!(!e.should_fire(5)); }
    #[test] fn test_tminus_fires() { let e = TMinusEvent::new(0, 10, 1, 0); assert!(e.should_fire(10)); }
    #[test] fn test_tminus_fired_once() { let mut e = TMinusEvent::new(0, 10, 1, 0); e.fired = true; assert!(!e.should_fire(10)); }
    #[test] fn test_room_creation() { let r = SpeculativeRoom::new(0); assert_eq!(r.position, 0); assert_eq!(r.tick, 0); }
    #[test] fn test_room_add_shadow() { let mut r = SpeculativeRoom::new(0); r.add_shadow(1); assert!(r.shadows.contains_key(&1)); }
    #[test] fn test_room_speculate() { let mut r = SpeculativeRoom::new(0); r.add_shadow(1); r.add_shadow(2); let resps = r.speculate_all(); assert_eq!(resps.len(), 2); }
    #[test] fn test_room_hints() { let mut r = SpeculativeRoom::new(0); let score = r.check_hints(); assert!(score > 0.0); }
    #[test] fn test_room_reconcile() { let mut r = SpeculativeRoom::new(0); r.add_shadow(1); let deltas = r.reconcile_shadows(&[(1, 0, 0.0)]); assert_eq!(deltas.len(), 1); }
    #[test] fn test_room_schedule_and_fire() { let mut r = SpeculativeRoom::new(0); r.schedule(TMinusEvent::new(0, 5, 1, 0)); r.tick(); r.tick(); r.tick(); r.tick(); r.tick(); let fired = r.fire_due(); assert_eq!(fired.len(), 1); }
    #[test] fn test_room_shadow_accuracy() { let mut r = SpeculativeRoom::new(0); r.add_shadow(1); r.reconcile_shadows(&[(1, 0, 0.0)]); let (hits, total) = r.shadow_accuracy(); assert!(hits > 0); assert_eq!(total, 1); }
    #[test] fn test_room_needs_resimulation() { let mut r = SpeculativeRoom::new(0); r.hints.shadow_hint(-1); assert!(r.needs_resimulation()); }
    #[test] fn test_room_no_resimulation() { let r = SpeculativeRoom::new(0); assert!(!r.needs_resimulation()); }
    #[test] fn test_room_tick_advance() { let mut r = SpeculativeRoom::new(0); r.tick(); assert_eq!(r.tick, 1); }
}
