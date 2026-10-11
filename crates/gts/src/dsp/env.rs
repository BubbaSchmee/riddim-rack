//! ADSR envelopes (per-oscillator amp, shared filter envelope).
//!
//! Analog-style exponential segments, each a one-pole chasing a target past its end point:
//! attack chases 1.3 so it lands on 1.0 at exactly `attack` (0.99 at ~0.98 x attack); decay
//! chases sustain with tau = decay/5; release chases -0.01 with tau = release/5. From level 1.0
//! the release passes 1e-3 at ~0.9 x release and goes idle at ~0.92 x release; from a lower
//! sustain it ends earlier (0.79 x release from 0.5), because the time constant is fixed.
//! Retrigger restarts the attack from the current level, so it never clicks.
//!
//! Curve (SPEC v2 1.11), -1 .. +1: 0 is the shape above, bit for bit. Other values move each
//! segment's overshoot `o` (how far past its end point it aims) geometrically, `o = o0 * (9/o0)^c`:
//! +1 aims 9x the segment span past the end, which is a near-straight line ("lin"); -1 aims
//! `o0^2/9` past it, a fast start and long settle ("log"). Attack and release keep their end
//! times (attack lands at `attack`, release from 1.0 idles at ~0.92 x release); decay keeps its
//! end-to-end time `decay` measured to its 1e-4 snap.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

const ATTACK_TARGET: f32 = 1.3;
const RELEASE_TARGET: f32 = -0.01;
/// Curve-0 overshoots: attack 0.3; decay 1/(e^5 - 1) (tau = decay/5 reaches within 0.7 % of
/// the span after `decay`); release 0.01.
const ATTACK_OVER: f32 = 0.3;
const DECAY_OVER: f32 = 0.006_784;
const RELEASE_OVER: f32 = 0.01;
/// Release time from 1.0 to idle at curve 0, in units of `release`: ln(1.01 / 0.01) / 5.
const RELEASE_END: f32 = 0.923_02;

#[derive(Debug, Clone, Copy)]
pub struct Adsr {
    stage: Stage,
    level: f32,
    a_coef: f32,
    d_coef: f32,
    r_coef: f32,
    sus: f32,
    /// Attack target, decay overshoot below sustain (fraction of 1 - sustain), release target.
    a_target: f32,
    d_over: f32,
    r_target: f32,
}

impl Default for Adsr {
    fn default() -> Self {
        Self {
            stage: Stage::Idle,
            level: 0.0,
            a_coef: 0.0,
            d_coef: 0.0,
            r_coef: 0.0,
            sus: 1.0,
            a_target: ATTACK_TARGET,
            d_over: 0.0,
            r_target: RELEASE_TARGET,
        }
    }
}

fn coef(tau_s: f32, sr: f32) -> f32 {
    (-1.0 / (tau_s.max(1e-5) * sr)).exp()
}

/// Overshoot for curve `c`: `o0` at 0, 9 at +1, `o0^2/9` at -1.
fn over(o0: f32, c: f32) -> f32 {
    o0 * (9.0 / o0).powf(c)
}

impl Adsr {
    /// Times in seconds, sustain 0..1, curve 0 (the v1 shape). Cheap enough to call once per block.
    pub fn set(&mut self, attack: f32, decay: f32, sustain: f32, release: f32, sr: f32) {
        self.set_curved(attack, decay, sustain, release, 0.0, sr);
    }

    /// [`Adsr::set`] with a curve, -1 log .. 0 exponential .. +1 linear.
    pub fn set_curved(&mut self, attack: f32, decay: f32, sustain: f32, release: f32, curve: f32, sr: f32) {
        let c = curve.clamp(-1.0, 1.0);
        if c == 0.0 {
            self.a_target = ATTACK_TARGET;
            self.d_over = 0.0;
            self.r_target = RELEASE_TARGET;
            self.d_coef = coef(decay / 5.0, sr);
            self.r_coef = coef(release / 5.0, sr);
        } else {
            self.a_target = 1.0 + over(ATTACK_OVER, c);
            let d = over(DECAY_OVER, c);
            self.d_over = d;
            self.d_coef = coef(decay / ((1.0 + d) / d).ln(), sr);
            let r = over(RELEASE_OVER, c);
            self.r_target = -r;
            self.r_coef = coef(release * RELEASE_END / ((1.0 + r) / r).ln(), sr);
        }
        self.a_coef = coef(attack / -(1.0 - 1.0 / self.a_target).ln(), sr);
        self.sus = sustain.clamp(0.0, 1.0);
    }

    pub fn gate_on(&mut self) {
        self.stage = Stage::Attack;
    }

    pub fn gate_off(&mut self) {
        if self.stage != Stage::Idle {
            self.stage = Stage::Release;
        }
    }

    /// Hard stop (choke / reset).
    pub fn kill(&mut self) {
        self.stage = Stage::Idle;
        self.level = 0.0;
    }

    pub fn stage(&self) -> Stage {
        self.stage
    }

    pub fn level(&self) -> f32 {
        self.level
    }

    pub fn is_active(&self) -> bool {
        self.stage != Stage::Idle
    }

    #[inline]
    pub fn next(&mut self) -> f32 {
        match self.stage {
            Stage::Idle => {}
            Stage::Attack => {
                self.level = self.a_target + (self.level - self.a_target) * self.a_coef;
                if self.level >= 1.0 {
                    self.level = 1.0;
                    self.stage = Stage::Decay;
                }
            }
            Stage::Decay => {
                let target = self.sus - self.d_over * (1.0 - self.sus);
                self.level = target + (self.level - target) * self.d_coef;
                if self.level - self.sus < 1e-4 {
                    self.level = self.sus;
                    self.stage = Stage::Sustain;
                }
            }
            // Follows sustain-knob moves without stepping.
            Stage::Sustain => self.level += (self.sus - self.level) * 0.002,
            Stage::Release => {
                self.level = self.r_target + (self.level - self.r_target) * self.r_coef;
                if self.level <= 0.0 {
                    self.level = 0.0;
                    self.stage = Stage::Idle;
                }
            }
        }
        self.level
    }
}
