//! Stripline impedance calculator (centered).
//!
//! Reference: Cohn / Wadell, centered stripline between two ground planes.
//!
//! Z0 = (60 / √Er) × ln(1.9 × (2H + T) / (0.8W + T))
//! Er_eff = Er  (fully embedded in dielectric, no air interface)
//!
//! # Range of validity
//!
//! This closed form is accurate for `W/(2H − T) < 0.35`. Outside that range it
//! progressively overestimates the fringing term, and for a sufficiently wide trace the
//! logarithm's argument falls below 1 and Zo goes negative. Such geometries are rejected
//! by the shared post-condition in [`common::finish`] rather than returning a negative
//! impedance, but values approaching the limit should be treated as approximate.

use crate::CalcError;
use crate::impedance::{common, types::ImpedanceResult};

/// Inputs for centered stripline impedance calculation. All dimensions in mils.
pub struct StriplineInput {
    /// Conductor width (mils).
    pub width: f64,
    /// Dielectric height — distance from trace to each ground plane (mils).
    /// For centered stripline, total dielectric thickness = 2 × height.
    pub height: f64,
    /// Conductor thickness (mils).
    pub thickness: f64,
    /// Substrate relative permittivity.
    pub er: f64,
}

/// Compute centered stripline characteristic impedance and derived quantities.
///
/// # TODO
/// - Full Cohn model from Saturn binary (~60 constants extracted at 0x00422bd8–0x00422dd4,
///   but intermediate computation flow only partially reconstructed). See
///   `docs/notes/ghidra-stripline.md` for extracted constants and partial analysis.
/// - Asymmetric (offset) stripline variant
pub fn calculate(input: &StriplineInput) -> Result<ImpedanceResult, CalcError> {
    let StriplineInput { width, height, thickness, er } = *input;

    if width <= 0.0 {
        return Err(CalcError::NegativeDimension { name: "width", value: width });
    }
    if height <= 0.0 {
        return Err(CalcError::NegativeDimension { name: "height", value: height });
    }
    if thickness < 0.0 {
        return Err(CalcError::NegativeDimension { name: "thickness", value: thickness });
    }
    if er < 1.0 {
        return Err(CalcError::OutOfRange {
            name: "er",
            value: er,
            expected: ">= 1.0",
        });
    }

    // Cohn/Wadell centered stripline
    let zo = (60.0 / er.sqrt()) * (1.9 * (2.0 * height + thickness) / (0.8 * width + thickness)).ln();

    // For stripline, er_eff = er (trace fully embedded in dielectric)
    let er_eff = er;

    common::finish(zo, er_eff, er)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_stripline() {
        let result = calculate(&StriplineInput {
            width: 10.0,
            height: 10.0,
            thickness: 1.4,
            er: 4.6,
        })
        .unwrap();

        // Reasonable impedance range for this geometry
        assert!(result.zo > 20.0 && result.zo < 100.0, "Zo = {}", result.zo);
        // Er_eff equals Er for stripline
        assert!((result.er_eff - 4.6).abs() < 1e-10);
        assert!(result.tpd_ps_per_in > 0.0);
        assert!(result.lo_nh_per_in > 0.0);
        assert!(result.co_pf_per_in > 0.0);
    }

    #[test]
    fn narrow_trace_higher_impedance() {
        let narrow = calculate(&StriplineInput {
            width: 3.0,
            height: 10.0,
            thickness: 1.4,
            er: 4.6,
        })
        .unwrap();

        let wide = calculate(&StriplineInput {
            width: 20.0,
            height: 10.0,
            thickness: 1.4,
            er: 4.6,
        })
        .unwrap();

        assert!(
            narrow.zo > wide.zo,
            "narrow Zo {} should be > wide Zo {}",
            narrow.zo,
            wide.zo
        );
    }

    #[test]
    fn higher_er_lower_impedance() {
        let low_er = calculate(&StriplineInput {
            width: 10.0,
            height: 10.0,
            thickness: 1.4,
            er: 2.2,
        })
        .unwrap();

        let high_er = calculate(&StriplineInput {
            width: 10.0,
            height: 10.0,
            thickness: 1.4,
            er: 4.6,
        })
        .unwrap();

        assert!(
            low_er.zo > high_er.zo,
            "low Er Zo {} should be > high Er Zo {}",
            low_er.zo,
            high_er.zo
        );
    }

    #[test]
    fn rejects_negative_width() {
        let result = calculate(&StriplineInput {
            width: -1.0,
            height: 10.0,
            thickness: 1.4,
            er: 4.6,
        });
        assert!(result.is_err());
    }

    #[test]
    fn rejects_low_er() {
        let result = calculate(&StriplineInput {
            width: 10.0,
            height: 10.0,
            thickness: 1.4,
            er: 0.5,
        });
        assert!(result.is_err());
    }

    /// A sufficiently wide trace drives the logarithm's argument below 1, making Zo
    /// negative. That previously reached the caller as a negative impedance (and a
    /// negative Lo) with no error. See VALIDATION.md finding H2.
    #[test]
    fn rejects_geometry_that_would_give_negative_impedance() {
        let r = calculate(&StriplineInput {
            width: 100.0,
            height: 20.0,
            thickness: 1.4,
            er: 4.6,
        });
        assert!(r.is_err(), "w=100/h=20 should be rejected, got {r:?}");
    }

    /// Within the formula's validity range Zo must stay positive and fall with width.
    #[test]
    fn impedance_positive_and_decreasing_within_validity() {
        let mut previous = f64::INFINITY;
        for width in [5.0, 10.0, 14.0] {
            let r = calculate(&StriplineInput { width, height: 20.0, thickness: 1.4, er: 4.6 })
                .unwrap();
            assert!(r.zo > 0.0, "w={width} gave Zo={}", r.zo);
            assert!(r.zo < previous, "Zo must fall with width: {} !< {previous}", r.zo);
            previous = r.zo;
        }
    }

    #[test]
    fn rejects_negative_thickness() {
        let r = calculate(&StriplineInput {
            width: 10.0,
            height: 20.0,
            thickness: -1.0,
            er: 4.6,
        });
        assert!(r.is_err());
    }
}
