//! ADSR envelopes (per-oscillator amp, shared filter envelope).
//!
//! Analog-style exponential segments. Attack chases 1.3 so it lands on 1.0 at exactly `attack`
//! (0.99 at ~0.98 x attack); decay chases sustain with tau = decay/5; release chases -0.01 with
//! tau = release/5, so it passes 1e-3 at ~0.9 x release and goes idle at ~0.92 x release.
//! Retrigger restarts the attack from the current level, so it never clicks.

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

#[derive(Debug, Clone, Copy)]
pub struct Adsr {
    stage: Stage,
    level: f32,
    a_coef: f32,
    d_coef: f32,
    r_coef: f32,
    sus: f32,
}

impl Default for Adsr {
    fn default() -> Self {
        Self { stage: Stage::Idle, level: 0.0, a_coef: 0.0, d_coef: 0.0, r_coef: 0.0, sus: 1.0 }
    }
}

fn coef(tau_s: f32, sr: f32) -> f32 {
    (-1.0 / (tau_s.max(1e-5) * sr)).exp()
}

impl Adsr {
    /// Times in seconds, sustain 0..1. Cheap enough to call once per block.
    pub fn set(&mut self, attack: f32, decay: f32, sustain: f32, release: f32, sr: f32) {
        self.a_coef = coef(attack / -(1.0 - 1.0 / ATTACK_TARGET).ln(), sr);
        self.d_coef = coef(decay / 5.0, sr);
        self.r_coef = coef(release / 5.0, sr);
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
                self.level = ATTACK_TARGET + (self.level - ATTACK_TARGET) * self.a_coef;
                if self.level >= 1.0 {
                    self.level = 1.0;
                    self.stage = Stage::Decay;
                }
            }
            Stage::Decay => {
                self.level = self.sus + (self.level - self.sus) * self.d_coef;
                if self.level - self.sus < 1e-4 {
                    self.level = self.sus;
                    self.stage = Stage::Sustain;
                }
            }
            // Follows sustain-knob moves without stepping.
            Stage::Sustain => self.level += (self.sus - self.level) * 0.002,
            Stage::Release => {
                self.level = RELEASE_TARGET + (self.level - RELEASE_TARGET) * self.r_coef;
                if self.level <= 0.0 {
                    self.level = 0.0;
                    self.stage = Stage::Idle;
                }
            }
        }
        self.level
    }
}
