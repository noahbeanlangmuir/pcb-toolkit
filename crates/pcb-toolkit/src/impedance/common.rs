//! Shared impedance computation helpers.
//!
//! Implements the Hammerstad-Jensen 1980 microstrip model (static effective
//! permittivity, air-microstrip impedance, and the finite-thickness correction),
//! plus Kirschning-Jansen frequency dispersion and the shared result
//! post-condition used by every topology module.
//!
//! Reference: E. Hammerstad and Ø. Jensen, "Accurate Models for Microstrip
//! Computer-Aided Design", IEEE MTT-S Int. Microwave Symp. Digest, 1980.

use super::types::ImpedanceResult;
use crate::CalcError;
use crate::constants;

/// Static effective dielectric constant for a **zero-thickness** strip
/// (Hammerstad-Jensen 1980).
///
/// ```text
/// a(u)  = 1 + (1/49)·ln((u⁴+(u/52)²)/(u⁴+0.432)) + (1/18.7)·ln(1+(u/18.1)³)
/// b(er) = 0.564·((er−0.9)/(er+3))^0.053
/// Er_eff = (er+1)/2 + ((er−1)/2)·(1+10/u)^(−a·b)
/// ```
///
/// `u` = W/H ratio, `er` = substrate relative permittivity. For finite conductor
/// thickness use [`microstrip_static`], which applies the H-J thickness correction.
pub fn er_eff_static(u: f64, er: f64) -> f64 {
    let a = 1.0
        + (1.0 / 49.0) * ((u.powi(4) + (u / 52.0).powi(2)) / (u.powi(4) + 0.432)).ln()
        + (1.0 / 18.7) * (1.0 + (u / 18.1).powi(3)).ln();
    let b = 0.564 * ((er - 0.9) / (er + 3.0)).powf(0.053);
    (er + 1.0) / 2.0 + ((er - 1.0) / 2.0) * (1.0 + 10.0 / u).powf(-a * b)
}

/// Characteristic impedance of the equivalent **air-filled** microstrip, Z01(u)
/// (Hammerstad-Jensen 1980).
///
/// ```text
/// F(u)   = 6 + (2π−6)·exp(−(30.666/u)^0.7528)
/// Z01(u) = (η₀/2π)·ln(F/u + √(1+(2/u)²))
/// ```
fn z01(u: f64) -> f64 {
    let f = 6.0 + (2.0 * std::f64::consts::PI - 6.0) * (-((30.666 / u).powf(0.7528))).exp();
    (constants::ETA_0 / (2.0 * std::f64::consts::PI)) * (f / u + (1.0 + (2.0 / u).powi(2)).sqrt()).ln()
}

/// Hammerstad-Jensen static microstrip solution with the finite-thickness
/// correction applied. Returns `(Zo, Er_eff)`.
///
/// The thickness correction widens the strip by different amounts for the air and
/// dielectric problems:
///
/// ```text
/// Δu₁ = (t/h)/π · ln(1 + 4e/((t/h)·coth²(√(6.517u))))
/// Δur = ½·(1 + 1/cosh(√(er−1)))·Δu₁
/// Er_eff = er_eff_static(u+Δur)·(Z01(u+Δu₁)/Z01(u+Δur))²
/// Zo     = Z01(u+Δu₁)/√(Er_eff)
/// ```
///
/// Because `Δu₁ > Δur`, the `(Z01(u₁)/Z01(ur))²` factor is less than one — so finite
/// thickness *lowers* Er_eff, which is the physically correct direction (a thicker
/// conductor pushes more field into the air above the substrate).
pub fn microstrip_static(width: f64, height: f64, thickness: f64, er: f64) -> (f64, f64) {
    let u = width / height;

    let (du1, dur) = if thickness > 0.0 {
        let th = thickness / height;
        let coth = 1.0 / (6.517 * u).sqrt().tanh();
        let du1 = (th / std::f64::consts::PI) * (1.0 + (4.0 * std::f64::consts::E) / (th * coth * coth)).ln();
        let dur = 0.5 * (1.0 + 1.0 / (er - 1.0).sqrt().cosh()) * du1;
        (du1, dur)
    } else {
        (0.0, 0.0)
    };

    let u1 = u + du1;
    let ur = u + dur;

    let z01_u1 = z01(u1);
    let er_eff = er_eff_static(ur, er) * (z01_u1 / z01(ur)).powi(2);
    let zo = z01_u1 / er_eff.sqrt();

    (zo, er_eff)
}

/// Apply frequency dispersion to a static microstrip solution.
/// Returns the dispersed `(Zo, Er_eff)`.
///
/// `Er_eff(f)` is the Kirschning-Jansen 1982 model:
///
/// ```text
/// fn = f[GHz]·h[mm]
/// P1 = 0.27488 + (0.6315 + 0.525/(1+0.0157·fn)²⁰)·u − 0.065683·exp(−8.7513u)
/// P2 = 0.33622·(1 − exp(−0.03442·er))
/// P3 = 0.0363·exp(−4.6u)·(1 − exp(−(fn/38.7)^4.97))
/// P4 = 1 + 2.751·(1 − exp(−(er/15.916)⁸))
/// Pf = P1·P2·((0.1844 + P3·P4)·fn)^1.5763
/// Er_eff(f) = er − (er − Er_eff)/(1 + Pf)
/// ```
///
/// `Zo(f)` uses the Hammerstad-Jensen / Getsinger form rather than Kirschning-Jansen's
/// full R₁..R₁₇ set:
///
/// ```text
/// Zo(f) = Zo·((Er_eff(f)−1)/(Er_eff−1))·√(Er_eff/Er_eff(f))
/// ```
///
/// Note that Zo *increases* modestly with frequency under this model (about +1% at
/// 10 GHz for 10 mil FR-4) — that is the expected behaviour, not an artifact.
/// At `freq_hz <= 0` the inputs are returned unchanged, so the static path is exact.
///
/// `u` is the physical strip-width ratio W/h, as Kirschning-Jansen define it — *not*
/// the thickness-corrected `u₁` used inside [`microstrip_static`]. Feeding `u₁` here
/// instead shifts the result by about 0.13% at 10 GHz; both choices appear in the wild,
/// and this one follows the paper.
pub fn apply_dispersion(
    zo: f64,
    er_eff: f64,
    er: f64,
    u: f64,
    height_mils: f64,
    freq_hz: f64,
) -> (f64, f64) {
    if freq_hz <= 0.0 || er_eff <= 1.0 {
        return (zo, er_eff);
    }

    // Normalised frequency: GHz × mm.
    let fx = (freq_hz * 1e-9) * (height_mils * 0.0254);

    let p1 = 0.27488 + (0.6315 + 0.525 / (1.0 + 0.0157 * fx).powi(20)) * u
        - 0.065683 * (-8.7513 * u).exp();
    let p2 = 0.33622 * (1.0 - (-0.03442 * er).exp());
    let p3 = 0.0363 * (-4.6 * u).exp() * (1.0 - (-((fx / 38.7).powf(4.97))).exp());
    let p4 = 1.0 + 2.751 * (1.0 - (-((er / 15.916).powi(8))).exp());
    let pf = p1 * p2 * ((0.1844 + p3 * p4) * fx).powf(1.5763);

    let er_eff_f = er - (er - er_eff) / (1.0 + pf);
    let zo_f = zo * ((er_eff_f - 1.0) / (er_eff - 1.0)) * (er_eff / er_eff_f).sqrt();

    (zo_f, er_eff_f)
}

/// Validate a computed impedance solution and package it with its derived quantities.
///
/// Every topology module funnels through here, so the physical invariants are checked
/// in exactly one place:
///
/// - `Zo` must be finite and strictly positive.
/// - `Er_eff` must be finite and lie in `[1, er]` — it is a weighted average of the
///   substrate permittivity and air, so it can never exceed `er` nor fall below 1.
///
/// A geometry that violates either has fallen outside the model's range of validity;
/// returning [`CalcError::UnphysicalResult`] is strictly better than handing back a
/// negative impedance or a NaN that serialises to JSON `null`.
pub fn finish(zo: f64, er_eff: f64, er: f64) -> Result<ImpedanceResult, CalcError> {
    if !er_eff.is_finite() {
        return Err(CalcError::UnphysicalResult {
            name: "er_eff",
            value: er_eff,
            reason: "not finite — geometry outside the model's valid range",
        });
    }
    if er_eff < 1.0 {
        return Err(CalcError::UnphysicalResult {
            name: "er_eff",
            value: er_eff,
            reason: "below 1.0 — Er_eff cannot be less than that of air",
        });
    }
    // Small relative tolerance so an exactly-homogeneous line (Er_eff == Er, e.g.
    // stripline) is not rejected by floating-point noise.
    if er_eff > er * (1.0 + 1e-9) {
        return Err(CalcError::UnphysicalResult {
            name: "er_eff",
            value: er_eff,
            reason: "exceeds Er — geometry outside the model's valid range",
        });
    }
    if !zo.is_finite() || zo <= 0.0 {
        return Err(CalcError::UnphysicalResult {
            name: "zo",
            value: zo,
            reason: "not a positive finite impedance — geometry outside the model's valid range",
        });
    }

    let tpd = propagation_delay(er_eff);
    Ok(ImpedanceResult {
        zo,
        er_eff,
        tpd_ps_per_in: tpd,
        lo_nh_per_in: inductance_per_length(zo, tpd),
        co_pf_per_in: capacitance_per_length(zo, tpd),
    })
}

/// Conductor thickness correction — effective width increase due to finite thickness.
///
/// `w` = conductor width (mils), `h` = dielectric height (mils), `t` = conductor thickness (mils).
/// Returns the effective width We (mils).
///
/// This is the Wheeler/IPC-2141 widening rule. It is **not** used by the
/// Hammerstad-Jensen path in [`microstrip_static`], which applies its own dual
/// (air / dielectric) correction; it is retained as a standalone utility.
pub fn effective_width(w: f64, h: f64, t: f64) -> f64 {
    if t <= 0.0 {
        return w;
    }
    let u = w / h;
    let dw = if u >= std::f64::consts::FRAC_PI_2 {
        (t / std::f64::consts::PI) * (1.0 + (2.0 * h / t).ln())
    } else {
        (t / std::f64::consts::PI) * (1.0 + (4.0 * std::f64::consts::PI * w / t).ln())
    };
    w + dw
}

/// Propagation delay from Er_eff (ps/in).
pub fn propagation_delay(er_eff: f64) -> f64 {
    // Tpd = sqrt(Er_eff) / c, where c = 11.803 in/ns = 11803 in/µs
    // Result in ps/in: (sqrt(Er_eff) / 11.803) * 1000
    er_eff.sqrt() / constants::SPEED_OF_LIGHT_IN_NS * 1000.0
}

/// Inductance per unit length from Zo and Tpd (nH/in).
pub fn inductance_per_length(zo: f64, tpd_ps_per_in: f64) -> f64 {
    // Lo = Zo × Tpd, with Tpd in ns/in → Lo in nH/in
    zo * tpd_ps_per_in / 1000.0
}

/// Capacitance per unit length from Zo and Tpd (pF/in).
pub fn capacitance_per_length(zo: f64, tpd_ps_per_in: f64) -> f64 {
    // Co = Tpd / Zo, with Tpd in ns/in → Co in nF/in → ×1000 for pF/in
    (tpd_ps_per_in / 1000.0) / zo * 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn er_eff_fr4_wide_trace() {
        // W/H = 2.0, Er = 4.6 → Er_eff should be roughly 3.0-3.8
        let er_eff = er_eff_static(2.0, 4.6);
        assert!(er_eff > 3.0 && er_eff < 4.6, "er_eff = {er_eff}");
    }

    #[test]
    fn er_eff_narrow_trace() {
        // W/H = 0.5, Er = 4.6 → Er_eff should be lower (more field in air)
        let narrow = er_eff_static(0.5, 4.6);
        let wide = er_eff_static(2.0, 4.6);
        assert!(narrow < wide, "narrow {narrow} should be < wide {wide}");
    }

    /// The defect this module was rewritten to fix: the old thickness correction
    /// was applied so that Er_eff *rose* with conductor thickness. Physically a
    /// thicker conductor puts more field in the air, so Er_eff must fall.
    #[test]
    fn thickness_lowers_er_eff() {
        let (_, thin) = microstrip_static(17.0, 10.0, 0.0, 4.6);
        let (_, thick) = microstrip_static(17.0, 10.0, 2.10, 4.6);
        assert!(
            thick < thin,
            "Er_eff must fall with thickness: t=0 gave {thin}, t=2.1 gave {thick}"
        );
    }

    /// Hammerstad-Jensen reference point, cross-checked against an independent
    /// implementation of the 1980 paper. W=17, H=10, T=2.10 mils, Er=4.6.
    #[test]
    fn hammerstad_jensen_reference_point() {
        let (zo, er_eff) = microstrip_static(17.0, 10.0, 2.10, 4.6);
        assert_relative_eq!(er_eff, 3.2772, epsilon = 0.001);
        assert_relative_eq!(zo, 49.6744, epsilon = 0.001);
    }

    #[test]
    fn dispersion_is_identity_at_dc() {
        let (zo, er_eff) = microstrip_static(17.0, 10.0, 2.10, 4.6);
        let (zo_f, er_f) = apply_dispersion(zo, er_eff, 4.6, 1.7, 10.0, 0.0);
        assert_relative_eq!(zo_f, zo, max_relative = 1e-15);
        assert_relative_eq!(er_f, er_eff, max_relative = 1e-15);
    }

    #[test]
    fn dispersion_raises_er_eff_toward_er() {
        let (zo, er_eff) = microstrip_static(17.0, 10.0, 2.10, 4.6);
        let mut last = er_eff;
        for f in [1e9, 5e9, 1e10, 2e10, 4e10] {
            let (_, er_f) = apply_dispersion(zo, er_eff, 4.6, 1.7, 10.0, f);
            assert!(er_f > last, "Er_eff must rise with frequency: {er_f} !> {last}");
            assert!(er_f <= 4.6, "Er_eff must stay <= Er: {er_f}");
            last = er_f;
        }
    }

    #[test]
    fn thickness_correction_increases_width() {
        let we = effective_width(5.0, 4.0, 1.4);
        assert!(we > 5.0, "effective width {we} should be > original 5.0");
    }

    #[test]
    fn finish_rejects_negative_impedance() {
        assert!(finish(-95.0, 4.6, 4.6).is_err());
    }

    #[test]
    fn finish_rejects_er_eff_above_er() {
        assert!(finish(50.0, 6.48, 4.6).is_err());
    }

    #[test]
    fn finish_rejects_nan() {
        assert!(finish(f64::NAN, 3.5, 4.6).is_err());
        assert!(finish(50.0, f64::NAN, 4.6).is_err());
    }

    #[test]
    fn finish_accepts_homogeneous_line() {
        // Stripline: Er_eff == Er exactly. Must not trip the upper bound.
        let r = finish(59.43, 4.6, 4.6).unwrap();
        assert_relative_eq!(r.er_eff, 4.6);
    }
}
