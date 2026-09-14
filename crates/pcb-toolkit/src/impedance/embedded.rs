//! Embedded microstrip impedance calculator.
//!
//! An embedded (buried) microstrip is a surface microstrip covered by a
//! dielectric overlay. The burial reduces both Zo and Er_eff compared to
//! a surface microstrip.
//!
//! Reference: Brooks, "Signal Integrity Issues and Printed Circuit Board Design".
//!
//! ```text
//! Er_eff_embedded = Er − (Er − Er_eff_surface) × exp(−2 × cover_height / height)
//! Z0_embedded     = Z0_surface × √(Er_eff_surface / Er_eff_embedded)
//! ```
//!
//! The Zo relation is exact: a dielectric cover changes no conductor geometry, so the
//! per-unit-length inductance is unchanged and Zo scales as 1/√Er_eff.
//!
//! The Er_eff blend is **not validated against any reference vector**. It has the right
//! endpoints (Er_eff_surface at zero cover, Er when fully buried), but its interior
//! shape — and the choice of `2·cover/height` as the decay constant, which scales burial
//! depth against the *lower* substrate thickness — is unverified. The reverse-engineered
//! Saturn form (`docs/notes/ghidra-edge-coupled.md`) is a rational blend rather than an
//! exponential one, sharing only those two endpoints.

use crate::CalcError;
use crate::impedance::{common, microstrip, types::ImpedanceResult};

/// Inputs for embedded microstrip impedance calculation. All dimensions in mils.
pub struct EmbeddedMicrostripInput {
    /// Conductor width (mils).
    pub width: f64,
    /// Dielectric height — distance from trace to ground plane (mils).
    pub height: f64,
    /// Conductor thickness (mils).
    pub thickness: f64,
    /// Substrate relative permittivity.
    pub er: f64,
    /// Cover height — dielectric above the trace (mils).
    /// When 0, result equals the surface microstrip.
    pub cover_height: f64,
    /// Frequency (Hz). Passed through to the surface microstrip calculation.
    pub frequency: f64,
}

/// Compute embedded microstrip impedance and derived quantities.
///
/// Delegates to [`microstrip::calculate`] for the surface result, then applies
/// the burial correction factor.
///
/// # TODO
/// - Verify exact formula against Saturn binary (Brooks vs IPC-2141)
pub fn calculate(input: &EmbeddedMicrostripInput) -> Result<ImpedanceResult, CalcError> {
    let EmbeddedMicrostripInput {
        width, height, thickness, er, cover_height, frequency,
    } = *input;

    if cover_height < 0.0 {
        return Err(CalcError::NegativeDimension {
            name: "cover_height",
            value: cover_height,
        });
    }

    // Compute surface microstrip first
    let surface = microstrip::calculate(&microstrip::MicrostripInput {
        width,
        height,
        thickness,
        er,
        frequency,
    })?;

    // Early return when cover=0 (surface microstrip)
    if cover_height == 0.0 {
        return Ok(surface);
    }

    // Burial correction factor
    let exp_factor = (-2.0 * cover_height / height).exp();

    let er_eff = er - (er - surface.er_eff) * exp_factor;

    // Adding a dielectric cover changes no conductor geometry, so the per-unit-length
    // inductance (and hence the air capacitance) is unchanged; only Er_eff changes.
    // Since Zo = Z_air/sqrt(Er_eff) with Z_air fixed, the ratio of two impedances for
    // the same cross-section is exactly sqrt(Er_eff_1/Er_eff_2).
    //
    // The previous `surface.zo * (1.0 - exp_factor)` was the complement of the needed
    // factor: it drove Zo to 0 as cover -> 0 and back to the *surface* value as
    // cover -> infinity, giving a discontinuity at 0 and the wrong buried limit.
    // See VALIDATION.md.
    let zo = surface.zo * (surface.er_eff / er_eff).sqrt();

    common::finish(zo, er_eff, er)
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn zero_cover_equals_surface() {
        let surface = microstrip::calculate(&microstrip::MicrostripInput {
            width: 10.0,
            height: 5.0,
            thickness: 1.4,
            er: 4.6,
            frequency: 0.0,
        })
        .unwrap();

        let embedded = calculate(&EmbeddedMicrostripInput {
            width: 10.0,
            height: 5.0,
            thickness: 1.4,
            er: 4.6,
            cover_height: 0.0,
            frequency: 0.0,
        })
        .unwrap();

        assert_relative_eq!(embedded.zo, surface.zo, max_relative = 1e-10);
        assert_relative_eq!(embedded.er_eff, surface.er_eff, max_relative = 1e-10);
    }

    #[test]
    fn burial_reduces_impedance() {
        let surface = microstrip::calculate(&microstrip::MicrostripInput {
            width: 10.0,
            height: 5.0,
            thickness: 1.4,
            er: 4.6,
            frequency: 0.0,
        })
        .unwrap();

        let embedded = calculate(&EmbeddedMicrostripInput {
            width: 10.0,
            height: 5.0,
            thickness: 1.4,
            er: 4.6,
            cover_height: 5.0,
            frequency: 0.0,
        })
        .unwrap();

        assert!(
            embedded.zo < surface.zo,
            "embedded Zo {} should be < surface Zo {}",
            embedded.zo,
            surface.zo
        );
    }

    #[test]
    fn burial_increases_er_eff() {
        let surface = microstrip::calculate(&microstrip::MicrostripInput {
            width: 10.0,
            height: 5.0,
            thickness: 1.4,
            er: 4.6,
            frequency: 0.0,
        })
        .unwrap();

        let embedded = calculate(&EmbeddedMicrostripInput {
            width: 10.0,
            height: 5.0,
            thickness: 1.4,
            er: 4.6,
            cover_height: 5.0,
            frequency: 0.0,
        })
        .unwrap();

        assert!(
            embedded.er_eff > surface.er_eff,
            "embedded er_eff {} should be > surface er_eff {}",
            embedded.er_eff,
            surface.er_eff
        );
    }

    #[test]
    fn deep_burial_approaches_er() {
        let embedded = calculate(&EmbeddedMicrostripInput {
            width: 10.0,
            height: 5.0,
            thickness: 1.4,
            er: 4.6,
            cover_height: 50.0, // very deep
            frequency: 0.0,
        })
        .unwrap();

        // er_eff should approach er for deep burial
        assert_relative_eq!(embedded.er_eff, 4.6, max_relative = 0.01);
    }

    #[test]
    fn rejects_negative_cover() {
        let result = calculate(&EmbeddedMicrostripInput {
            width: 10.0,
            height: 5.0,
            thickness: 1.4,
            er: 4.6,
            cover_height: -1.0,
            frequency: 0.0,
        });
        assert!(result.is_err());
    }

    fn zo_at(cover_height: f64) -> f64 {
        calculate(&EmbeddedMicrostripInput {
            width: 10.0,
            height: 5.0,
            thickness: 1.4,
            er: 4.6,
            cover_height,
            frequency: 0.0,
        })
        .unwrap()
        .zo
    }

    /// Regression guard: Zo was previously scaled by `(1 - exp(-2c/h))`, which collapsed
    /// to ~0 for any infinitesimal cover. `cover=0` gave 44.36 Ω while `cover=0.0001`
    /// gave 0.0018 Ω. See VALIDATION.md.
    #[test]
    fn zo_is_continuous_at_zero_cover() {
        let at_zero = zo_at(0.0);
        let just_above = zo_at(1e-4);
        assert_relative_eq!(just_above, at_zero, max_relative = 1e-3);
    }

    /// Zo must fall monotonically with burial depth. The old formula rose, and converged
    /// back to the *surface* value instead of the fully-buried one.
    #[test]
    fn zo_decreases_monotonically_with_burial() {
        let mut previous = zo_at(0.0);
        for cover in [0.01, 0.1, 1.0, 5.0, 20.0, 50.0] {
            let zo = zo_at(cover);
            assert!(zo < previous, "Zo must fall as cover grows: {zo} !< {previous} at {cover}");
            previous = zo;
        }
        // Fully buried: Er_eff -> Er, so Zo -> Z_air/sqrt(Er), well below the surface value.
        assert!(zo_at(50.0) < zo_at(0.0) * 0.9);
    }
}
