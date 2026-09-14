//! Coplanar waveguide (CPW over ground) impedance calculator.
//!
//! Reference: Wadell, "Transmission Line Design Handbook", 1991.

use crate::CalcError;
use crate::impedance::{common, types::ImpedanceResult};

/// Inputs for coplanar waveguide (CPW over ground) impedance calculation.
/// All dimensions in mils.
pub struct CoplanarInput {
    /// Center conductor width (mils).
    pub width: f64,
    /// Gap between center conductor and coplanar ground (mils).
    pub gap: f64,
    /// Substrate height to bottom ground plane (mils).
    pub height: f64,
    /// Conductor thickness (mils).
    pub thickness: f64,
    /// Substrate relative permittivity.
    pub er: f64,
}

/// Complete elliptic integral ratio K(k)/K(k') via the Hilberg approximation.
///
/// Returns K(k)/K(k'), where k' = sqrt(1 - k²).
///
/// For k <= 1/sqrt(2): K(k)/K(k') = π / ln(2·(1+sqrt(k'))/(1-sqrt(k')))
/// For k >  1/sqrt(2): K(k)/K(k') = (1/π) · ln(2·(1+sqrt(k))/(1-sqrt(k)))
fn elliptic_ratio(k: f64) -> f64 {
    debug_assert!(
        (0.0..1.0).contains(&k),
        "elliptic_ratio requires k in [0,1), got {k}"
    );
    // At k == 1 the `1 - sqrt(k)` denominator is exactly zero, yielding +inf. Callers
    // reject the geometries that reach it; this clamp is a last-resort backstop.
    let k = k.clamp(0.0, 1.0 - 1e-12);

    let threshold = 1.0 / std::f64::consts::SQRT_2;
    if k <= threshold {
        let kp = (1.0 - k * k).sqrt();
        std::f64::consts::PI / (2.0 * (1.0 + kp.sqrt()) / (1.0 - kp.sqrt())).ln()
    } else {
        (1.0 / std::f64::consts::PI) * (2.0 * (1.0 + k.sqrt()) / (1.0 - k.sqrt())).ln()
    }
}

/// Compute coplanar waveguide (CPW over ground) impedance and derived quantities.
pub fn calculate(input: &CoplanarInput) -> Result<ImpedanceResult, CalcError> {
    let CoplanarInput { width, gap, height, thickness, er } = *input;

    if width <= 0.0 {
        return Err(CalcError::NegativeDimension { name: "width", value: width });
    }
    if gap <= 0.0 {
        return Err(CalcError::NegativeDimension { name: "gap", value: gap });
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
    // k3 = tanh(πW/4h)/tanh(π(W+2G)/4h) saturates to exactly 1.0 in f64 once
    // πW/(4h) exceeds ~18.7, i.e. W/h above roughly 24, and loses most of its
    // precision from about W/h = 20. Past that the backing ground dominates and the
    // structure is a microstrip, not a coplanar waveguide — so reject rather than
    // return noise.
    if width / height >= 20.0 {
        return Err(CalcError::OutOfRange {
            name: "width/height",
            value: width / height,
            expected: "< 20 (above this the structure is a microstrip, not CPW)",
        });
    }

    // Modulus for the air-region elliptic integral
    let k = width / (width + 2.0 * gap);

    // Modulus for the substrate elliptic integral (via hyperbolic tangents)
    let k3 = (std::f64::consts::PI * width / (4.0 * height)).tanh()
        / (std::f64::consts::PI * (width + 2.0 * gap) / (4.0 * height)).tanh();

    // Partial-capacitance model for conductor-backed CPW (Ghione-Naldi / Wadell):
    //   C_air  = 2·e0·K(k)/K(k')       (upper half-space)
    //   C_diel = 2·e0·er·K(k3)/K(k3')  (region down to the backing ground)
    // so Er_eff = C/C0 is a convex combination of 1 and er:
    //   q      = [K(k3)/K(k3')] / ([K(k3)/K(k3')] + [K(k)/K(k')])
    //   Er_eff = 1 + (er - 1)·q
    //
    // The previous expression, `1 + (er-1)/2 · [K(k')/K(k)] · [K(k3)/K(k3')]`, is the
    // *ungrounded* CPW filling factor. It is only bounded when paired with the sinh
    // modulus; paired with the tanh modulus k3 used here it is unbounded, and produced
    // Er_eff > Er (6.48 against Er=4.6). Writing it as the weight q makes the bound
    // structural: q is in [0,1] by construction, so Er_eff is in [1, er]. See
    // VALIDATION.md finding H3.
    let rk = elliptic_ratio(k);
    let rk3 = elliptic_ratio(k3);
    let q = rk3 / (rk3 + rk);
    let er_eff = 1.0 + (er - 1.0) * q;

    // Characteristic impedance for the same partial-capacitance model:
    //   Z0 = 60π / (sqrt(Er_eff) · [K(k)/K(k') + K(k3)/K(k3')])
    // The earlier `30π/(sqrt(Er_eff)·K(k)/K(k'))` is the ungrounded-CPW form; using it
    // alongside a grounded Er_eff made the module self-inconsistent, and let Zo run away
    // as the gap widened instead of tending to a microstrip-like value.
    let zo = (60.0 * std::f64::consts::PI) / (er_eff.sqrt() * (rk + rk3));

    common::finish(zo, er_eff, er)
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    fn typical() -> CoplanarInput {
        CoplanarInput { width: 10.0, gap: 5.0, height: 10.0, thickness: 1.4, er: 4.6 }
    }

    #[test]
    fn reasonable_impedance() {
        let result = calculate(&typical()).unwrap();
        assert!(
            result.zo >= 30.0 && result.zo <= 150.0,
            "Z0 = {} should be in 30–150 Ω range",
            result.zo
        );
    }

    #[test]
    fn narrower_gap_lowers_impedance() {
        let wide_gap = calculate(&typical()).unwrap();
        let narrow_gap = calculate(&CoplanarInput { gap: 2.0, ..typical() }).unwrap();
        assert!(
            narrow_gap.zo < wide_gap.zo,
            "narrow gap Z0 {} should be < wide gap Z0 {}",
            narrow_gap.zo,
            wide_gap.zo
        );
    }

    #[test]
    fn higher_er_lowers_impedance() {
        let low_er = calculate(&typical()).unwrap();
        let high_er = calculate(&CoplanarInput { er: 9.8, ..typical() }).unwrap();
        assert!(
            high_er.zo < low_er.zo,
            "high-Er Z0 {} should be < low-Er Z0 {}",
            high_er.zo,
            low_er.zo
        );
    }

    #[test]
    fn er_eff_between_one_and_er() {
        let result = calculate(&typical()).unwrap();
        assert!(
            result.er_eff > 1.0 && result.er_eff < 4.6,
            "er_eff = {} should be in (1.0, 4.6)",
            result.er_eff
        );
    }

    #[test]
    fn wide_gap_approaches_microstrip_range() {
        // With a very wide gap the coplanar grounds are far from the center conductor;
        // the bottom ground plane dominates and Z0 should be in the microstrip ballpark.
        let result = calculate(&CoplanarInput {
            width: 10.0,
            gap: 1000.0,
            height: 10.0,
            thickness: 1.4,
            er: 4.6,
        })
        .unwrap();
        // With the coplanar grounds pushed far away, the backing ground plane dominates
        // and the structure behaves like a microstrip: Er_eff rises toward Er (~3.88 for
        // Er=4.6) and Z0 settles around 82 Ω.
        //
        // The previous comment here claimed Er_eff approaches 1 and Z0 ~140 Ω — that was
        // rationalising the unbounded filling-factor formula, which let Z0 run away with
        // gap. Both are now bounded.
        assert!(
            result.zo > 40.0 && result.zo < 200.0,
            "wide-gap Z0 {} should be in a plausible transmission-line range (40–200 Ω)",
            result.zo
        );
        assert!(
            result.er_eff > 3.0 && result.er_eff <= 4.6,
            "wide-gap Er_eff {} should approach Er, not 1",
            result.er_eff
        );
    }

    #[test]
    fn rejects_non_positive_width() {
        let result = calculate(&CoplanarInput { width: 0.0, ..typical() });
        assert!(result.is_err());
        let result = calculate(&CoplanarInput { width: -1.0, ..typical() });
        assert!(result.is_err());
    }

    #[test]
    fn rejects_non_positive_gap() {
        let result = calculate(&CoplanarInput { gap: 0.0, ..typical() });
        assert!(result.is_err());
        let result = calculate(&CoplanarInput { gap: -5.0, ..typical() });
        assert!(result.is_err());
    }

    #[test]
    fn rejects_non_positive_height() {
        let result = calculate(&CoplanarInput { height: 0.0, ..typical() });
        assert!(result.is_err());
    }

    #[test]
    fn rejects_er_below_one() {
        let result = calculate(&CoplanarInput { er: 0.5, ..typical() });
        assert!(result.is_err());
    }

    #[test]
    fn derived_quantities_consistent() {
        let r = calculate(&typical()).unwrap();
        // Lo = Zo × Tpd (ps/in → ns/in ÷1000)
        let lo_check = r.zo * r.tpd_ps_per_in / 1000.0;
        assert_relative_eq!(r.lo_nh_per_in, lo_check, max_relative = 1e-10);
        // Co = Tpd / Zo (same unit bookkeeping)
        let co_check = (r.tpd_ps_per_in / 1000.0) / r.zo * 1000.0;
        assert_relative_eq!(r.co_pf_per_in, co_check, max_relative = 1e-10);
    }

    /// Er_eff is a convex combination of 1 and Er, so it is bounded on both sides for
    /// every geometry. The previous unbounded filling-factor form reached 6.48 (and
    /// 8.19 at gap=1000) against Er=4.6. See VALIDATION.md finding H3.
    #[test]
    fn er_eff_stays_within_one_and_er() {
        for gap in [0.1, 1.0, 5.0, 10.0, 50.0, 200.0, 1000.0] {
            let r = calculate(&CoplanarInput { gap, ..typical() }).unwrap();
            assert!(
                (1.0..=4.6).contains(&r.er_eff),
                "gap={gap} gave Er_eff={} outside [1, 4.6]",
                r.er_eff
            );
            assert!(r.zo.is_finite() && r.zo > 0.0, "gap={gap} gave Zo={}", r.zo);
        }
    }

    /// Widening the gap moves field out of the substrate, so Er_eff must rise toward Er
    /// monotonically and Zo must rise with it -- both bounded.
    #[test]
    fn wider_gap_raises_er_eff_monotonically() {
        let mut previous = 0.0;
        for gap in [1.0, 5.0, 10.0, 50.0, 200.0] {
            let r = calculate(&CoplanarInput { gap, ..typical() }).unwrap();
            assert!(r.er_eff > previous, "Er_eff must rise with gap: {} !> {previous}", r.er_eff);
            previous = r.er_eff;
        }
    }

    /// k3 saturates to exactly 1.0 in f64 for W/h above roughly 24, which made
    /// `elliptic_ratio` divide by zero. Such geometries are microstrip, not CPW.
    #[test]
    fn rejects_degenerate_width_to_height() {
        let r = calculate(&CoplanarInput { width: 50.0, height: 1.0, ..typical() });
        assert!(r.is_err(), "W/h = 50 should be rejected, got {r:?}");
    }

    #[test]
    fn rejects_negative_thickness() {
        assert!(calculate(&CoplanarInput { thickness: -1.0, ..typical() }).is_err());
    }
}
