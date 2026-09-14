//! Microstrip impedance calculator.
//!
//! Reference: Hammerstad & Jensen, "Accurate Models for Microstrip Computer-Aided
//! Design", IEEE MTT-S International Microwave Symposium Digest, 1980.

use crate::CalcError;
use crate::impedance::{common, types::ImpedanceResult};

/// Inputs for microstrip impedance calculation. All dimensions in mils.
pub struct MicrostripInput {
    /// Conductor width (mils).
    pub width: f64,
    /// Dielectric height — distance from trace to ground plane (mils).
    pub height: f64,
    /// Conductor thickness (mils). Usually from copper weight.
    pub thickness: f64,
    /// Substrate relative permittivity (e.g., 4.6 for FR-4).
    pub er: f64,
    /// Frequency (Hz). Used for Kirschning-Jansen dispersion correction.
    pub frequency: f64,
}

/// Compute microstrip characteristic impedance and derived quantities.
pub fn calculate(input: &MicrostripInput) -> Result<ImpedanceResult, CalcError> {
    let MicrostripInput { width, height, thickness, er, frequency } = *input;

    if width <= 0.0 {
        return Err(CalcError::NegativeDimension { name: "width", value: width });
    }
    if height <= 0.0 {
        return Err(CalcError::NegativeDimension { name: "height", value: height });
    }
    if er < 1.0 {
        return Err(CalcError::OutOfRange {
            name: "er",
            value: er,
            expected: ">= 1.0",
        });
    }

    if thickness < 0.0 {
        return Err(CalcError::NegativeDimension { name: "thickness", value: thickness });
    }

    // Static Hammerstad-Jensen solution, including the finite-thickness correction.
    let (zo, er_eff) = common::microstrip_static(width, height, thickness, er);

    // Kirschning-Jansen dispersion. At frequency == 0 this is exactly the identity.
    let (zo, er_eff) =
        common::apply_dispersion(zo, er_eff, er, width / height, height, frequency);

    common::finish(zo, er_eff, er)
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    fn input(width: f64, height: f64, thickness: f64, er: f64, frequency: f64) -> MicrostripInput {
        MicrostripInput { width, height, thickness, er, frequency }
    }

    /// Hammerstad-Jensen reference point (`docs/notes/17-test-vectors.md` §1 geometry),
    /// cross-checked against an independent implementation of the 1980 paper.
    ///
    /// This assertion replaces a `20.0 < Zo < 80.0` range check, which was wide enough
    /// to pass while the calculator was ~5% wrong. See VALIDATION.md finding C1.
    #[test]
    fn hammerstad_jensen_reference_vector() {
        let r = calculate(&input(17.0, 10.0, 2.10, 4.6, 500e6)).unwrap();
        assert_relative_eq!(r.er_eff, 3.2776, epsilon = 0.001);
        assert_relative_eq!(r.zo, 49.6806, epsilon = 0.001);
        // Derived quantities must stay self-consistent: Zo = sqrt(Lo/Co).
        // Lo is nH/in and Co is pF/in, so the ratio carries a factor of 1e3.
        assert_relative_eq!(
            r.zo,
            (r.lo_nh_per_in / r.co_pf_per_in * 1000.0).sqrt(),
            epsilon = 1e-6
        );
    }

    /// Published 50 Ω microstrip designs (`docs/notes/17-test-vectors.md`,
    /// "Secondary Validation Vectors" — disk91.com). Er=4.3, T=0.035 mm.
    #[test]
    fn fifty_ohm_reference_designs() {
        const MM: f64 = 1.0 / 0.0254; // mm -> mils
        for (h_mm, w_mm) in [(0.8, 1.52), (1.0, 1.90), (1.6, 3.06)] {
            let r = calculate(&input(w_mm * MM, h_mm * MM, 0.035 * MM, 4.3, 0.0)).unwrap();
            assert!(
                (r.zo - 50.0).abs() / 50.0 < 0.005,
                "h={h_mm}mm w={w_mm}mm should give ~50 ohm, got {}",
                r.zo
            );
        }
    }

    /// The inverted invariant that caused C1: finite conductor thickness pushes field
    /// into the air above the substrate, so Er_eff must *fall* as thickness rises.
    #[test]
    fn er_eff_falls_with_thickness() {
        let thin = calculate(&input(17.0, 10.0, 0.5, 4.6, 0.0)).unwrap();
        let thick = calculate(&input(17.0, 10.0, 3.0, 4.6, 0.0)).unwrap();
        assert!(
            thick.er_eff < thin.er_eff,
            "Er_eff must fall with thickness: {} !< {}",
            thick.er_eff,
            thin.er_eff
        );
    }

    /// `frequency` was previously discarded, so `--freq` was a silent no-op.
    #[test]
    fn frequency_changes_the_result() {
        let dc = calculate(&input(17.0, 10.0, 2.10, 4.6, 0.0)).unwrap();
        let rf = calculate(&input(17.0, 10.0, 2.10, 4.6, 10e9)).unwrap();
        assert!(
            rf.er_eff > dc.er_eff,
            "dispersion must raise Er_eff: {} !> {}",
            rf.er_eff,
            dc.er_eff
        );
        assert!(rf.er_eff <= 4.6);
    }

    /// Er_eff is a weighted average of Er and air, so it is bounded on both sides for
    /// any geometry. This previously blew up to 36362 with `zo` becoming NaN.
    #[test]
    fn er_eff_stays_bounded_for_extreme_geometry() {
        for (w, h, t) in [(10.0, 1.0, 100.0), (10.0, 1.0, 500.0), (0.5, 50.0, 1.4)] {
            if let Ok(r) = calculate(&input(w, h, t, 4.6, 0.0)) {
                assert!(
                    r.er_eff >= 1.0 && r.er_eff <= 4.6,
                    "w={w} h={h} t={t} gave Er_eff={}",
                    r.er_eff
                );
                assert!(r.zo.is_finite() && r.zo > 0.0, "w={w} h={h} t={t} gave Zo={}", r.zo);
            }
        }
    }

    #[test]
    fn narrow_trace_higher_impedance() {
        let narrow = calculate(&MicrostripInput {
            width: 3.0,
            height: 5.0,
            thickness: 1.4,
            er: 4.6,
            frequency: 0.0,
        })
        .unwrap();

        let wide = calculate(&MicrostripInput {
            width: 20.0,
            height: 5.0,
            thickness: 1.4,
            er: 4.6,
            frequency: 0.0,
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
    fn rejects_negative_width() {
        let result = calculate(&MicrostripInput {
            width: -1.0,
            height: 5.0,
            thickness: 1.4,
            er: 4.6,
            frequency: 0.0,
        });
        assert!(result.is_err());
    }
}
