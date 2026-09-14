# Validation Report

Independent correctness and usability audit of `pcb-toolkit` v0.1.5.

**Date:** 2026-09-14 · **Toolchain:** cargo/rustc 1.98.0 (x86_64-pc-windows-msvc)
**Method:** every calculator was driven through the CLI and compared against an
independent reimplementation of the published formula (written separately, in Python),
against the reference vectors in `docs/notes/17-test-vectors.md`, and against physical
invariants. Nothing in this report was taken from the library's own test suite.

---

## Resolution status (updated 2026-09-14)

The correctness defects below were fixed in a follow-up pass. **All Critical, High and
Medium findings are resolved**; the deferred items were explicitly scoped out and are
listed as such.

| ID | Finding | Status |
|---|---|---|
| C1 | microstrip Er_eff/Zo wrong | **Resolved** — rewritten as true Hammerstad-Jensen 1980 |
| C2 | pi-pad used the T-pad series formula | **Resolved** — `Z·(K²−1)/(2K)`; tests + source table corrected |
| H1 | `--freq` silently ignored | **Resolved** — Kirschning-Jansen dispersion implemented |
| H2 | stripline negative Zo | **Resolved** — rejected by a shared post-condition |
| H3 | coplanar Er_eff > Er | **Resolved** — bounded partial-capacitance form; Zo moved to the matching CBCPW form |
| M1 | formulas misattributed | **Resolved** — the implementation is now genuinely H-J |
| M2 | two disagreeing microstrip models | **Resolved (narrowed)** — divergence cut from 5.4% to 0.19%; `differential` deliberately keeps IPC-2141 (see below) |
| M3 | NaN / impossible Er_eff at high t/h | **Resolved** — H-J is well-behaved there, and the post-condition backstops it |
| — | embedded microstrip Zo discontinuity | **Resolved** (found while fixing; Zo now scales as 1/√Er_eff) |
| — | `elliptic_ratio` had no domain guard | **Resolved** — clamped, and degenerate W/h rejected |
| — | `thickness` unvalidated in stripline/coplanar | **Resolved** |
| M4 | STATUS.md overstates completeness | **Deferred** (doc accuracy, out of scope) |
| L1–L6 | gold melting-point constant, dead code, hygiene | **Deferred** (out of scope) |

Verification: 214 tests pass (up from 192), `cargo clippy` clean, and every microstrip
result was diffed against an independently written Python implementation of H-J and K-J
across 9 geometries including dispersion — agreement is exact to 0.00000000%.

**`differential::edge_coupled_external` intentionally still uses IPC-2141.** It is the one
module matching its reference vector exactly on all 8 outputs, so re-baselining it would
have discarded the repo's only end-to-end validation. The two models now agree to 0.19%.

---

## Verdict

> **Everything from here down describes the state of the repo as audited, before the
> fixes.** It is kept as the historical record and as the rationale for each change — see
> the Resolution status table above for what has since been fixed. Present-tense claims
> below refer to the pre-fix code, and the per-module confidence table likewise predates
> the repairs.

**Usable: yes. Correct: mostly, with two calculators that return materially wrong
numbers and three that return unphysical output outside their validity range.**

The repo builds clean, has no `unsafe`, no stubs, no panics on the paths tested, and 192
tests that genuinely pass. Most calculators are right — several match their reference to
5+ significant figures. The CLI works, its help is accurate, and the README examples
reproduce exactly.

But the test suite does not establish correctness, and that gap is not cosmetic. The
flagship calculator — `impedance::microstrip` — is wrong by ~5% and no test detects it,
because its only impedance assertion is `20.0 < Zo < 80.0`. A second defect
(`ohms_law::pi_pad`) is actively *pinned in place* by tests that assert the wrong value.

The headline "192 tests passing" in `STATUS.md` is accurate as a count and misleading as
a claim of validation.

### Severity summary

| | Count | Items |
|---|---|---|
| **Critical** — silently returns wrong numbers for valid, ordinary input | 2 | C1 microstrip, C2 pi-pad |
| **High** — unphysical output or silently ignored input | 3 | H1 frequency, H2 stripline, H3 coplanar |
| **Medium** — correctness-adjacent, or misleading documentation | 4 | M1–M4 |
| **Low / informational** | 5 | L1–L5 |

---

## Baseline health

| Check | Result |
|---|---|
| `cargo build --workspace` | Clean, zero warnings |
| `cargo test --workspace --all-targets` | **192 passed, 0 failed** (matches STATUS.md) |
| `cargo clippy --workspace --all-targets` | Clean, zero warnings |
| `cargo fmt --check` | **Fails** — 103 diffs across 35 files |
| `cargo doc --no-deps` | Builds; warns of bin/lib output filename collision |
| CLI tests | **Zero.** No `tests/`, no benches, no doc tests |

CI (`.github/workflows/ci.yml`) runs only `build` + `test`. It runs neither `clippy` nor
`fmt --check`, which is why 103 formatting diffs accumulated unnoticed. The workspace
declares `[workspace.lints.clippy] disallowed_types = "deny"` (`Cargo.toml:25`), but that
lint is inert without a `clippy.toml` listing the disallowed types, and none exists — the
lint configuration currently enforces nothing.

---

## Per-module confidence

Ratings earned by independent recomputation, not by the presence of tests.

| Module | Rating | Evidence |
|---|---|---|
| `differential::edge_coupled_external` | **Verified** | All 8 outputs match ref p11 to 5+ s.f. |
| `via` | **Verified** | C/L/Z/f_res match ref p36 to 5+ s.f. |
| `pdn` | **Verified** | All 3 outputs match ref p28 exactly |
| `padstack::thru_hole` | **Verified** | 56/56/80 exact |
| `padstack::corner_to_corner` | **Verified** | √2·100 = 141.421 exact |
| `reactance` | **Verified** | Matches closed form; *more* precise than the doc's f_res |
| `wavelength` | **Verified** | 59.015 vs ref 59.01426 |
| `inductor` | **Verified** | Matches Mohan/Wheeler to 0.009%; the doc's value is the outlier |
| `spacing` | **Verified** | Matches IPC-2221 at 4 spot voltages exactly |
| `wire_gauge` | **Verified** | AWG 4/0, 10, 22, 40 match standard tables |
| `materials` | **Verified** | 45 entries; Er correct on spot checks |
| `ohms_law` (E-I-R, LED, R/C/L, **t-pad**) | **Verified** | All exact |
| `ohms_law::pi_pad` | **WRONG** | **C2** — series arm uses the T-pad formula |
| `impedance::microstrip` | **WRONG** | **C1** — Er_eff +5.8%, Zo −5.4% |
| `impedance::stripline` | **Plausible / unguarded** | Exact within validity; **H2** negative Zo outside it |
| `impedance::coplanar` | **Plausible / unguarded** | **H3** Er_eff exceeds Er at large gap |
| `impedance::embedded` | **Unverified** | No reference vector; **H1** ignores frequency |
| `differential::*` (other 4) | **Unverified** | No reference vectors exist |
| `differential::broadside_coupled` | **Unverified** | Self-declared "confidence: LOW" |
| `current` | **Partial** | DC resistance & skin depth correct; IPC-2152 path unreachable from CLI |
| `fusing` | **Partial** | Onderdonk core correct; trace-area path unvalidatable (**L3**) |
| `crosstalk` | **Known-divergent** | Diverges from Saturn — but *ours is the more physical* (**L5**) |

---

## Critical findings

### C1 — `impedance::microstrip` returns wrong Zo and Er_eff  ✅ RESOLVED

**Files:** [`impedance/common.rs`](crates/pcb-toolkit/src/impedance/common.rs) (`er_eff_static`, `effective_width`), applied at [`microstrip.rs:42-46`](crates/pcb-toolkit/src/impedance/microstrip.rs:42)

The conductor-thickness correction is applied by widening the trace (`we = w + Δw`) and
feeding `u = we/h` into `er_eff_static`. But `er_eff_static` is monotonically *increasing*
in `u`, so finite thickness makes Er_eff go **up**. Physically it must go **down**: a
thicker conductor pushes more field into the air above the substrate.

Reference geometry (`docs/notes/17-test-vectors.md` §1 — W=17, H=10, T=2.10, Er=4.6):

| Quantity | pcb-toolkit | Independent Hammerstad-Jensen | Reference | Error |
|---|---|---|---|---|
| Er_eff | 3.4681 | **3.2772** | 3.2802 | **+5.8%** |
| Zo (Ω) | 49.1445 | 49.6744 | 50.7426 | −3.1% |
| Tpd (ps/in) | 157.7812 | 153.3784 | 153.4869 | +2.8% |
| Co (pF/in) | 3.2106 | 3.0877 | 3.0248 | +6.1% |

The direction of the error is confirmed by the zero-thickness case: independent HJ gives
Er_eff 3.4323 at t=0 falling to 3.2772 at t=2.10, while pcb-toolkit rises from 3.4341 to
3.4681.

**The library contradicts itself.** For identical single-ended geometry (W=10, H=15,
T=2.10, Er=4.6), two modules disagree:

```
impedance microstrip          -> Zo = 73.4951   <-- outlier
differential (its internal Zo) -> Zo = 77.5036
reference (Saturn p11)         -> Zo = 77.5040
independent Hammerstad-Jensen  -> Zo = 77.6499
```

`differential::edge_coupled_external` computes single-ended Zo with the IPC-2141 formula
`87/√(er+1.41)·ln(5.98h/(0.8w+t))` ([`edge_coupled_external.rs:47`](crates/pcb-toolkit/src/differential/edge_coupled_external.rs:47))
and agrees with all three references. `impedance::microstrip` uses a different model and
is the sole disagreeing value.

**The README's own headline example is affected.** `impedance microstrip -w 10 --height 5
--er 4.6` is printed in all three READMEs as `Zo = 44.3599, Er_eff = 3.5172`. Independent
Hammerstad-Jensen for that geometry gives `Zo = 44.8322, Er_eff = 3.3075` — the published
example is **+6.3% on Er_eff**. The example reproduces exactly, so the documentation is
faithful; the number it faithfully documents is wrong.

**Failure scenario:** a user sizing a 50 Ω trace on 10 mil FR-4 gets a width ~5% off. On a
controlled-impedance board that is a real fabrication error, and it is the single most
likely thing anyone uses this library for.

**Why no test catches it:** `microstrip.rs` has three tests — `20.0 < Zo < 80.0`, "narrow
> wide", and "negative width errors". Every value in the table above passes all three.

### C2 — `ohms_law::pi_pad` computes the series resistor with the T-pad formula  ✅ RESOLVED

**File:** [`ohms_law.rs:325`](crates/pcb-toolkit/src/ohms_law.rs:325)

```rust
let r_shunt_ohm  = z_ohm * (k + 1.0) / (k - 1.0);   // correct for Pi
let r_series_ohm = z_ohm * (k - 1.0) / (k + 1.0);   // this is the T-pad series formula
```

The Pi-pad series arm should be `Z·(K²−1)/(2K)`. As written, `pi_pad` and `t_pad` return
*identical* series resistance at every attenuation:

| dB | pi_series (ours) | t_series (ours) | correct pi_series | error |
|---|---|---|---|---|
| 1 | 2.8751 | 2.8751 | 5.7692 | 2.0× |
| 6 | 16.6139 | 16.6139 | 37.3519 | 2.2× |
| 20 | 40.9091 | 40.9091 | 247.5000 | **6.0×** |

Shunt values are correct for both topologies; only the Pi series arm is wrong.

**Failure scenario:** anyone building a Pi attenuator from this output gets the wrong
series resistor — at 20 dB, 41 Ω instead of 248 Ω. The pad will not attenuate correctly
and will not present the design impedance.

**This one is worse than C1 in one respect:** tests at
[`ohms_law.rs:413`](crates/pcb-toolkit/src/ohms_law.rs:413) and
[`:421`](crates/pcb-toolkit/src/ohms_law.rs:421) assert the *wrong* values (2.88, 25.97),
so the bug is locked in. Those expectations trace to the Pi-pad table in
`docs/notes/17-test-vectors.md`, whose series column is itself a duplicate of the T-pad
column — a bad source faithfully reproduced.

---

## High findings

### H1 — `--freq` is silently ignored by `microstrip` and `embedded`  ✅ RESOLVED

`MicrostripInput::frequency` is documented "Used for Kirschning-Jansen dispersion
correction" ([`microstrip.rs:19`](crates/pcb-toolkit/src/impedance/microstrip.rs:19)) but
is destructured away with `..` at [`microstrip.rs:25`](crates/pcb-toolkit/src/impedance/microstrip.rs:25)
and never read. Output is bit-identical from 0 Hz to 50 GHz:

```
f=0       zo=49.144468 er_eff=3.468128
f=50GHz   zo=49.144468 er_eff=3.468128
```

No dispersion code exists anywhere. `KJ_DISPERSION_A/B` (`constants.rs:35,38`) are
declared and never used. Yet `common.rs:4` advertises "Kirschning-Jansen frequency
dispersion — used by all topology modules", and CLAUDE.md and STATUS.md both claim
"frequency-dependent Er_eff".

**Failure scenario:** a user computing impedance at 10 GHz silently gets the DC answer.
Dispersion is precisely what matters at that frequency. The CLI accepts the flag, so
there is no signal that it did nothing.

`stripline` correctly omits `-f` (homogeneous medium — genuinely non-dispersive).

### H2 — `stripline` returns negative impedance outside its validity range  ✅ RESOLVED

The IPC-2141 formula is valid only for `w/(b−t) < 0.35`. There is no guard. Sweeping
width at h=20 mil, t=1.4 mil, Er=4.6 (validity limit ≈ 14 mil):

```
w=10  -> Zo =  59.4311   (valid, matches IPC-2141 to 0.008%)
w=14  -> Zo =  51.2347   (validity limit)
w=50  -> Zo =  17.9560   (well outside validity, no warning)
w=100 -> Zo =  -0.9579   <-- negative
```

**Failure scenario:** a wide stripline (power distribution, low-impedance trace) silently
returns a negative characteristic impedance, and negative Lo. Nothing errors.

### H3 — `coplanar` reports Er_eff greater than Er  ✅ RESOLVED

Er_eff is a weighted average of the substrate Er and air, so `1 ≤ Er_eff ≤ Er` always.
At w=10, h=10, Er=4.6:

```
gap=10  -> Er_eff = 3.6777   ok
gap=50  -> Er_eff = 5.0643   <-- exceeds Er = 4.6
gap=200 -> Er_eff = 6.4802   <-- exceeds Er = 4.6
```

**Failure scenario:** wide-gap CPW returns an impossible Er_eff, and every derived value
(Tpd, Lo, Co) inherits the error.

---

## Medium findings

**M1 — Formulas are misattributed.** `impedance::microstrip` is documented
"Hammerstad-Jensen 1980" in CLAUDE.md, STATUS.md, and its module header. It actually
implements the **Wheeler/IPC-2141** closed form for Zo and the **Schneider 1975**
`(1+12/u)^−0.5` approximation for Er_eff ([`common.rs:12-18`](crates/pcb-toolkit/src/impedance/common.rs:12)).
Hammerstad-Jensen 1980 is the `a(u)`/`b(εr)` formulation, which appears nowhere. A reader
choosing this library for HJ accuracy gets a cruder model than advertised.

**M2 — Two inconsistent microstrip models coexist.** See C1. Whichever is correct, the
library should not return two different answers for one physical structure.

**M3 — `microstrip` produces NaN and impossible Er_eff at high t/h.** At h=1 mil:

```
t=100 -> er_eff =   509.0669, zo = null   (NaN serialized as JSON null)
t=500 -> er_eff = 36362.5008, zo = null
```

Er_eff must be ≤ 4.6. `zo` reaches consumers as JSON `null`, which will break
deserialization into `f64`. Validation bounds each dimension independently but never
their ratio. There is also a discontinuity at `u = π/2` where `effective_width` switches
branches (Zo jumps 14.29 → 52.61 between t=5 and t=20).

**M4 — `STATUS.md` overstates completeness.** It marks every CLI command "Complete".
In fact: `padstack` exposes 2 of the 7 sub-calculators documented in
`docs/notes/06-padstack.md`; the entire IPC-2152 path (`current::calculate_ipc2152`,
~230 lines) has no CLI exposure; `ohms_law::pi_pad`/`t_pad` support only matched
impedances, not the unmatched case described in `docs/notes/ghidra-ohmslaw.md`; and the
45-material database is never consumed by any calculator or command.

---

## Low / informational

**L1 — Dead `pub` constant holds gold's melting point.**
[`constants.rs:23`](crates/pcb-toolkit/src/constants.rs:23) defines
`COPPER_MELTING_POINT_C = 1064.62`. Copper melts at 1084.62 °C; 1064.18 °C is *gold*.
The value is unused — `fusing.rs` correctly uses its own `COPPER_MELTING_TEMP_C = 1084.62` —
but it is public API, so a downstream consumer can import the wrong constant and get a
~2% fusing-current error.

**L2 — `docs/notes/17-test-vectors.md` §7 repeats the same error**, quoting melting temp
as 1064.62 where every other document in the repo says 1084.62.

**L3 — The fusing reference vector is internally inconsistent and cannot validate the
trace path.** §7 states W=10 mil, thickness 0.70 mil, cross-section 18.79 sq.mils — but
10 × 0.70 = 7.0 sq.mils, and etching only reduces area. The stated area implies a ~26.8
mil trace. The library's own test sidesteps this by feeding area directly
(`saturn_fusing_current_from_area`), so it passes while `fusing_current_trace` disagrees
with the vector by 2.7× (1.304 A vs 3.515 A). The Onderdonk core itself is correct.

**L4 — `wire_gauge::area_saturn` is an undocumented magic quantity.**
[`wire_gauge.rs:174`](crates/pcb-toolkit/src/wire_gauge.rs:174) exports
`diameter_mils² / 700.0` with no units and no physical interpretation. It is public API.

**L5 — `crosstalk` diverges from its reference, and that is probably correct.** Ours:
−35.46 dB / 0.084 V. Saturn: −2.233 dB / 3.866 V. Saturn's figure implies 77% coupling
from a 250 mil run — implausible, and consistent with Saturn marking this calculator
"Unsupported". `crosstalk.rs:13` frames this as our shortcoming; the evidence suggests
the opposite. Recommend re-framing rather than "fixing" toward the reference.

**L6 — Hygiene.** Unused `toml` dependency in the CLI (declared, zero references);
`print_result`'s non-JSON branch is dead (all 34 call sites pass `true`); 103 rustfmt
diffs; `PROGRESS.md` still lists "Begin Rust implementation" as the next priority at
v0.1.5; `NOTES.md`'s material table is self-contradicting and superseded by
`materials-er-mapping.md`.

---

## On the test suite

192 tests pass, but they are weighted toward assertions that cannot fail for a plausible
implementation: 159 `assert!` (largely range and monotonicity) against 122
`assert_relative_eq!`, and only ~14 tests named for a reference vector.

Three distinct problems, in descending order of importance:

1. **Absolute assertions sourced from a bad reference** actively entrench bugs — C2 is
   the example. Worse than no test.
2. **Range assertions on the flagship path** (`20.0 < Zo < 80.0`) admit a 60 Ω window and
   cannot detect a 5% error.
3. **Tests that avoid the disputed step** — `saturn_fusing_current_from_area` validates
   Onderdonk but skips the area derivation that actually disagrees (L3).

`docs/notes/17-test-vectors.md` contains 18 transcribed reference vectors; roughly half
are never asserted anywhere, including the microstrip vector that would have caught C1 on
day one. The `tests/vectors/*.json` store planned in
`docs/notes/16-rust-design-research.md` was never created.

---

## Remediation backlog

Not executed — this audit made no code changes.

**Priority 1 — wrong numbers**
1. Fix `pi_pad` series arm to `Z·(K²−1)/(2K)`; correct the tests at `ohms_law.rs:413,421`
   and the Pi-pad table in `17-test-vectors.md` (C2). Smallest fix, unambiguous, highest
   confidence.
2. Fix the microstrip thickness correction so Er_eff decreases with thickness (C1).
   Decide deliberately whether the target is true Hammerstad-Jensen or the IPC-2141 model
   already used by `edge_coupled_external` — then use *one* model library-wide (M1, M2).
3. Wire the microstrip vector (§1) into a test as an absolute assertion. It is already
   transcribed and would have caught C1.

**Priority 2 — unphysical output**
4. Implement Kirschning-Jansen dispersion, or remove `--freq` and the claims in
   CLAUDE.md / STATUS.md / `common.rs:4` (H1). Silently ignoring an input is the worst of
   the three options.
5. Add validity guards: stripline `w/(b−t) < 0.35` (H2), coplanar `Er_eff ≤ Er` (H3),
   microstrip `t/h` bound (M3). Return `CalcError::OutOfRange` rather than a wrong number.
6. Add a debug assertion on the universal invariant `1 ≤ Er_eff ≤ Er` in every impedance
   path — it alone would have caught H3 and M3.

**Priority 3 — trust**
7. Convert the remaining vectors in `17-test-vectors.md` into absolute assertions;
   mark unvalidatable ones `#[ignore]` with a reason rather than substituting a range check.
8. Correct `STATUS.md`'s "Complete" markers (M4); delete or fix `COPPER_MELTING_POINT_C`
   (L1); document or remove `area_saturn` (L4); re-frame the crosstalk note (L5).
9. Add `clippy` + `fmt --check` to CI, and either populate `clippy.toml` or drop the inert
   `disallowed_types` lint. Add CLI integration tests — there are currently none.

---

## Reproducing this report

```bash
cargo build --workspace && cargo test --workspace --all-targets
```

Every numeric claim above comes from driving `target/debug/pcb-toolkit` with `--json` at
the stated inputs, compared against an independent Python implementation of the published
formula. Key reproductions:

```bash
pcb-toolkit --json impedance microstrip -w 17 --height 10 -t 2.10 --er 4.6 -f 500MHz
pcb-toolkit --json ohms-law pi-pad --attenuation 20 --impedance 50
pcb-toolkit --json ohms-law t-pad  --attenuation 20 --impedance 50
pcb-toolkit --json impedance stripline -w 100 --height 20 -t 1.4 --er 4.6
pcb-toolkit --json impedance coplanar -w 10 -g 200 --height 10 --er 4.6
pcb-toolkit --json impedance microstrip -w 10 --height 1 -t 500 --er 4.6
```
