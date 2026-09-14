/**
 * Declarative description of every calculator.
 *
 * This is the single source of truth for the UI: the form renderer, the results panel and
 * the sidebar are all driven from here, so adding a calculator is a data change rather
 * than new components. `id` must match the dispatch arm in `src-tauri/src/lib.rs`, and
 * each field `key` must match the parameter name that arm reads.
 */

import type { Dimension } from "../units";

export type Field =
  | {
      kind: "number";
      key: string;
      label: string;
      dim?: Dimension;
      default?: string;
      /** Optional inputs may be left blank. */
      optional?: boolean;
      help?: string;
    }
  | {
      kind: "select";
      key: string;
      label: string;
      options: { value: string; label: string }[];
      default: string;
      help?: string;
    }
  | { kind: "bool"; key: string; label: string; default: boolean; help?: string }
  | {
      kind: "list";
      key: string;
      label: string;
      dim?: Dimension;
      default: string;
      help?: string;
    };

export type Output = {
  key: string;
  label: string;
  unit?: string;
  precision?: number;
  /** Render larger, at the top of the results panel. */
  primary?: boolean;
};

export type Calculator = {
  id: string;
  name: string;
  group: string;
  blurb: string;
  fields: Field[];
  outputs: Output[];
  /** Field key the substrate picker should fill, when this calculator has an Er input. */
  erField?: string;
};

// ---- shared fragments ------------------------------------------------------------

const ER: Field = {
  kind: "number",
  key: "er",
  label: "Dielectric constant (Er)",
  dim: "plain",
  default: "4.6",
};
const THICKNESS: Field = {
  kind: "number",
  key: "thickness",
  label: "Copper thickness",
  dim: "length",
  default: "1.4",
  help: "1.4 mil = 1 oz copper",
};
const FREQ: Field = {
  kind: "number",
  key: "frequency",
  label: "Frequency",
  dim: "freq",
  default: "0",
  help: "0 = static (no dispersion)",
};

const IMPEDANCE_OUT: Output[] = [
  { key: "zo", label: "Zo", unit: "Ω", primary: true },
  { key: "er_eff", label: "Er effective" },
  { key: "tpd_ps_per_in", label: "Propagation delay", unit: "ps/in" },
  { key: "lo_nh_per_in", label: "Inductance", unit: "nH/in" },
  { key: "co_pf_per_in", label: "Capacitance", unit: "pF/in" },
];

const DIFF_OUT: Output[] = [
  { key: "zdiff", label: "Z differential", unit: "Ω", primary: true },
  { key: "zo", label: "Zo (single-ended)", unit: "Ω" },
  { key: "zodd", label: "Z odd mode", unit: "Ω" },
  { key: "zeven", label: "Z even mode", unit: "Ω" },
  { key: "kb", label: "Kb (unterminated)", precision: 6 },
  { key: "kb_db", label: "Kb (unterminated)", unit: "dB" },
  { key: "kb_term", label: "Kb (terminated)", precision: 6 },
  { key: "kb_term_db", label: "Kb (terminated)", unit: "dB" },
];

const ETCH: Field = {
  kind: "select",
  key: "etch_factor",
  label: "Etch factor",
  default: "none",
  options: [
    { value: "none", label: "None (rectangular)" },
    { value: "1:1", label: "1:1 (trapezoid)" },
    { value: "2:1", label: "2:1 (trapezoid)" },
  ],
};

const COPPER_WEIGHTS = [
  { value: "0.25", label: "0.25 oz (0.35 mil)" },
  { value: "0.5", label: "0.5 oz (0.70 mil)" },
  { value: "1", label: "1 oz (1.40 mil)" },
  { value: "1.5", label: "1.5 oz (2.10 mil)" },
  { value: "2", label: "2 oz (2.80 mil)" },
  { value: "2.5", label: "2.5 oz (3.50 mil)" },
  { value: "3", label: "3 oz (4.20 mil)" },
  { value: "4", label: "4 oz (5.60 mil)" },
  { value: "5", label: "5 oz (7.00 mil)" },
];

const COMBO_OUT = (key: string, label: string, unit: string): Output[] => [
  { key, label, unit, primary: true },
];

// ---- the catalogue ----------------------------------------------------------------

export const CALCULATORS: Calculator[] = [
  // ================= Impedance =================
  {
    id: "impedance.microstrip",
    name: "Microstrip",
    group: "Impedance",
    blurb: "Surface trace over a single ground plane. Hammerstad-Jensen 1980 with Kirschning-Jansen dispersion.",
    erField: "er",
    fields: [
      { kind: "number", key: "width", label: "Trace width", dim: "length", default: "10" },
      { kind: "number", key: "height", label: "Dielectric height", dim: "length", default: "5" },
      THICKNESS,
      ER,
      FREQ,
    ],
    outputs: IMPEDANCE_OUT,
  },
  {
    id: "impedance.stripline",
    name: "Stripline",
    group: "Impedance",
    blurb: "Trace centered between two ground planes. Accurate for W/(2H−T) < 0.35.",
    erField: "er",
    fields: [
      { kind: "number", key: "width", label: "Trace width", dim: "length", default: "10" },
      {
        kind: "number",
        key: "height",
        label: "Height to each plane",
        dim: "length",
        default: "20",
        help: "Total dielectric = 2 × this",
      },
      THICKNESS,
      ER,
    ],
    outputs: IMPEDANCE_OUT,
  },
  {
    id: "impedance.embedded",
    name: "Embedded Microstrip",
    group: "Impedance",
    blurb: "Microstrip buried under a dielectric cover. Zo scales as 1/√Er_eff.",
    erField: "er",
    fields: [
      { kind: "number", key: "width", label: "Trace width", dim: "length", default: "10" },
      { kind: "number", key: "height", label: "Dielectric height", dim: "length", default: "5" },
      {
        kind: "number",
        key: "cover_height",
        label: "Cover above trace",
        dim: "length",
        default: "5",
        help: "0 = surface microstrip",
      },
      THICKNESS,
      ER,
      FREQ,
    ],
    outputs: IMPEDANCE_OUT,
  },
  {
    id: "impedance.coplanar",
    name: "Coplanar Waveguide",
    group: "Impedance",
    blurb: "Conductor-backed CPW — center trace with coplanar grounds over a backing plane.",
    erField: "er",
    fields: [
      { kind: "number", key: "width", label: "Center conductor width", dim: "length", default: "10" },
      { kind: "number", key: "gap", label: "Gap to coplanar ground", dim: "length", default: "8" },
      {
        kind: "number",
        key: "height",
        label: "Height to backing plane",
        dim: "length",
        default: "10",
        help: "Width/height must stay below 20",
      },
      THICKNESS,
      ER,
    ],
    outputs: IMPEDANCE_OUT,
  },

  // ================= Differential pairs =================
  {
    id: "differential.edge_coupled_external",
    name: "Edge Coupled External",
    group: "Differential Pairs",
    blurb: "Surface differential pair over one ground plane.",
    erField: "er",
    fields: [
      { kind: "number", key: "width", label: "Trace width", dim: "length", default: "10" },
      { kind: "number", key: "spacing", label: "Edge-to-edge spacing", dim: "length", default: "5" },
      { kind: "number", key: "height", label: "Dielectric height", dim: "length", default: "15" },
      { ...THICKNESS, default: "2.1" },
      ER,
    ],
    outputs: DIFF_OUT,
  },
  {
    id: "differential.edge_coupled_internal_sym",
    name: "Edge Coupled Internal (Sym)",
    group: "Differential Pairs",
    blurb: "Differential pair centered between two ground planes.",
    erField: "er",
    fields: [
      { kind: "number", key: "width", label: "Trace width", dim: "length", default: "10" },
      { kind: "number", key: "spacing", label: "Edge-to-edge spacing", dim: "length", default: "5" },
      {
        kind: "number",
        key: "height",
        label: "Height to each plane",
        dim: "length",
        default: "15",
        help: "Total dielectric = 2 × this",
      },
      { ...THICKNESS, default: "2.1" },
      ER,
    ],
    outputs: DIFF_OUT,
  },
  {
    id: "differential.edge_coupled_internal_asym",
    name: "Edge Coupled Internal (Asym)",
    group: "Differential Pairs",
    blurb: "Offset stripline differential pair with unequal plane spacing.",
    erField: "er",
    fields: [
      { kind: "number", key: "width", label: "Trace width", dim: "length", default: "10" },
      { kind: "number", key: "spacing", label: "Edge-to-edge spacing", dim: "length", default: "5" },
      { kind: "number", key: "height1", label: "Height to top plane", dim: "length", default: "15" },
      { kind: "number", key: "height2", label: "Height to bottom plane", dim: "length", default: "10" },
      { ...THICKNESS, default: "2.1" },
      ER,
    ],
    outputs: DIFF_OUT,
  },
  {
    id: "differential.edge_coupled_embedded",
    name: "Edge Coupled Embedded",
    group: "Differential Pairs",
    blurb: "Buried differential pair under a dielectric cover.",
    erField: "er",
    fields: [
      { kind: "number", key: "width", label: "Trace width", dim: "length", default: "10" },
      { kind: "number", key: "spacing", label: "Edge-to-edge spacing", dim: "length", default: "5" },
      { kind: "number", key: "height", label: "Dielectric height", dim: "length", default: "15" },
      { kind: "number", key: "cover_height", label: "Cover above trace", dim: "length", default: "5" },
      { ...THICKNESS, default: "2.1" },
      ER,
    ],
    outputs: DIFF_OUT,
  },
  {
    id: "differential.broadside_coupled",
    name: "Broadside Coupled",
    group: "Differential Pairs",
    blurb: "Vertically stacked differential pair, shielded or unshielded.",
    erField: "er",
    fields: [
      { kind: "number", key: "width", label: "Trace width", dim: "length", default: "10" },
      { kind: "number", key: "separation", label: "Vertical separation", dim: "length", default: "5" },
      {
        kind: "number",
        key: "height_total",
        label: "Ground-to-ground spacing",
        dim: "length",
        default: "30",
        help: "Used in shielded mode only",
      },
      { ...THICKNESS, default: "2.1" },
      ER,
      { kind: "bool", key: "shielded", label: "Shielded (between two planes)", default: true },
    ],
    outputs: DIFF_OUT,
  },

  // ================= Current & thermal =================
  {
    id: "current.ipc2221a",
    name: "Conductor Current (IPC-2221A)",
    group: "Current & Thermal",
    blurb: "Trace current capacity, DC resistance, voltage drop and skin depth.",
    fields: [
      { kind: "number", key: "width", label: "Trace width", dim: "length", default: "50" },
      THICKNESS,
      { kind: "number", key: "length", label: "Trace length", dim: "length", default: "1000" },
      { kind: "number", key: "temperature_rise", label: "Temperature rise", dim: "plain", default: "10", help: "°C above ambient" },
      { kind: "number", key: "ambient_temp", label: "Ambient temperature", dim: "plain", default: "25", help: "°C" },
      { kind: "number", key: "frequency", label: "Frequency", dim: "freq", default: "0", help: "0 = DC only" },
      ETCH,
      { kind: "bool", key: "is_internal", label: "Internal layer", default: false },
    ],
    outputs: [
      { key: "current_capacity", label: "Current capacity", unit: "A", primary: true },
      { key: "cross_section", label: "Cross section", unit: "sq mil" },
      { key: "resistance_dc", label: "DC resistance", unit: "Ω", precision: 6 },
      { key: "voltage_drop", label: "Voltage drop", unit: "V" },
      { key: "power_dissipation", label: "Power dissipation", unit: "W" },
      { key: "current_density", label: "Current density", unit: "A/mil²", precision: 6 },
      { key: "skin_depth_mils", label: "Skin depth", unit: "mil" },
    ],
  },
  {
    id: "current.ipc2152",
    name: "Conductor Current (IPC-2152)",
    group: "Current & Thermal",
    blurb: "IPC-2152 with area, temperature and board-thickness modifiers.",
    fields: [
      { kind: "number", key: "width", label: "Trace width", dim: "length", default: "50" },
      THICKNESS,
      { kind: "number", key: "length", label: "Trace length", dim: "length", default: "1000" },
      { kind: "number", key: "temperature_rise", label: "Temperature rise", dim: "plain", default: "10", help: "°C above ambient" },
      { kind: "number", key: "ambient_temp", label: "Ambient temperature", dim: "plain", default: "25", help: "°C" },
      { kind: "number", key: "frequency", label: "Frequency", dim: "freq", default: "0" },
      { kind: "number", key: "board_thickness_mils", label: "Board thickness", dim: "length", default: "62" },
      { kind: "number", key: "material_modifier", label: "Material modifier", dim: "plain", default: "1.0" },
      { kind: "number", key: "user_modifier", label: "User modifier", dim: "plain", default: "1.0" },
      ETCH,
      { kind: "bool", key: "is_internal", label: "Internal layer", default: false },
      { kind: "bool", key: "has_copper_plane", label: "Adjacent copper plane", default: false },
    ],
    outputs: [
      { key: "current_capacity", label: "Current capacity", unit: "A", primary: true },
      { key: "cross_section", label: "Cross section", unit: "sq mil" },
      { key: "resistance_dc", label: "DC resistance", unit: "Ω", precision: 6 },
      { key: "voltage_drop", label: "Voltage drop", unit: "V" },
      { key: "power_dissipation", label: "Power dissipation", unit: "W" },
      { key: "current_density", label: "Current density", unit: "A/mil²", precision: 6 },
      { key: "skin_depth_mils", label: "Skin depth", unit: "mil" },
      { key: "m_area", label: "Area modifier" },
      { key: "m_temp", label: "Temperature modifier" },
      { key: "m_board", label: "Board modifier" },
    ],
  },
  {
    id: "fusing.trace",
    name: "Fusing Current (Trace)",
    group: "Current & Thermal",
    blurb: "Onderdonk fusing current from trace geometry and copper weight.",
    fields: [
      { kind: "number", key: "width_mils", label: "Trace width", dim: "length", default: "10" },
      {
        kind: "select",
        key: "base_copper",
        label: "Base copper",
        default: "1",
        options: COPPER_WEIGHTS,
      },
      {
        kind: "select",
        key: "plating",
        label: "Plating",
        default: "bare",
        options: [
          { value: "bare", label: "Bare (no plating)" },
          { value: "0.5", label: "0.5 oz (0.70 mil)" },
          { value: "1", label: "1 oz (1.40 mil)" },
          { value: "1.5", label: "1.5 oz (2.10 mil)" },
          { value: "2", label: "2 oz (2.80 mil)" },
          { value: "2.5", label: "2.5 oz (3.50 mil)" },
          { value: "3", label: "3 oz (4.20 mil)" },
        ],
      },
      ETCH,
      { kind: "number", key: "time_s", label: "Fusing time", dim: "time", default: "1" },
      { kind: "number", key: "ambient_c", label: "Ambient temperature", dim: "plain", default: "25", help: "°C" },
    ],
    outputs: [
      { key: "fusing_current_a", label: "Fusing current", unit: "A", primary: true },
      { key: "copper_thickness_mils", label: "Total copper thickness", unit: "mil" },
      { key: "area_sq_mils", label: "Cross section", unit: "sq mil" },
      { key: "area_circular_mils", label: "Cross section", unit: "circular mil" },
      { key: "melting_temp_c", label: "Melting temperature", unit: "°C" },
    ],
  },
  {
    id: "fusing.area",
    name: "Fusing Current (Area)",
    group: "Current & Thermal",
    blurb: "Onderdonk equation directly from a known cross-sectional area.",
    fields: [
      { kind: "number", key: "area_circular_mils", label: "Cross section", dim: "plain", default: "23.93", help: "circular mils" },
      { kind: "number", key: "time_s", label: "Fusing time", dim: "time", default: "1" },
      { kind: "number", key: "ambient_c", label: "Ambient temperature", dim: "plain", default: "25", help: "°C" },
      { kind: "number", key: "melting_temp_c", label: "Melting temperature", dim: "plain", default: "1084.62", help: "°C — copper" },
    ],
    outputs: [{ key: "fusing_current_a", label: "Fusing current", unit: "A", primary: true }],
  },
  {
    id: "thermal",
    name: "Thermal Management",
    group: "Current & Thermal",
    blurb: "Junction temperature from thermal resistance and dissipated power.",
    fields: [
      { kind: "number", key: "r_theta_ja", label: "Thermal resistance θJA", dim: "plain", default: "50", help: "°C/W" },
      { kind: "number", key: "power_w", label: "Power dissipation", dim: "plain", default: "2", help: "W" },
      { kind: "number", key: "t_ambient_c", label: "Ambient temperature", dim: "plain", default: "25", help: "°C" },
    ],
    outputs: [
      { key: "t_junction_c", label: "Junction temperature", unit: "°C", primary: true },
      { key: "t_junction_f", label: "Junction temperature", unit: "°F" },
    ],
  },

  // ================= Via & padstack =================
  {
    id: "via",
    name: "Via Properties",
    group: "Via & Padstack",
    blurb: "Via parasitics from the coaxial model — capacitance, inductance, impedance, resonance.",
    erField: "er",
    fields: [
      { kind: "number", key: "hole_diameter_mils", label: "Hole diameter", dim: "length", default: "10" },
      { kind: "number", key: "pad_diameter_mils", label: "Pad diameter", dim: "length", default: "20" },
      {
        kind: "number",
        key: "antipad_diameter_mils",
        label: "Antipad diameter",
        dim: "length",
        default: "40",
        help: "Must exceed the pad diameter",
      },
      { kind: "number", key: "height_mils", label: "Via height (board thickness)", dim: "length", default: "62" },
      { kind: "number", key: "plating_thickness_mils", label: "Plating thickness", dim: "length", default: "1" },
      ER,
    ],
    outputs: [
      { key: "impedance_ohms", label: "Via impedance", unit: "Ω", primary: true },
      { key: "capacitance_pf", label: "Capacitance", unit: "pF" },
      { key: "inductance_nh", label: "Inductance", unit: "nH" },
      { key: "resonant_freq_mhz", label: "Resonant frequency", unit: "MHz" },
    ],
  },
  {
    id: "padstack.thru_hole",
    name: "Thru-Hole Padstack",
    group: "Via & Padstack",
    blurb: "Pad diameters for external, internal signal and internal plane layers.",
    fields: [
      { kind: "number", key: "hole_diameter_mils", label: "Hole diameter", dim: "length", default: "32" },
      { kind: "number", key: "annular_ring_mils", label: "Annular ring", dim: "length", default: "12" },
      { kind: "number", key: "isolation_width_mils", label: "Isolation width", dim: "length", default: "12" },
    ],
    outputs: [
      { key: "pad_external_mils", label: "External layers", unit: "mil", primary: true },
      { key: "pad_internal_signal_mils", label: "Internal signal layers", unit: "mil" },
      { key: "pad_internal_plane_mils", label: "Internal plane layers", unit: "mil" },
    ],
  },
  {
    id: "padstack.corner_to_corner",
    name: "Corner to Corner",
    group: "Via & Padstack",
    blurb: "Diagonal span across a rectangular footprint.",
    fields: [
      { kind: "number", key: "a_mils", label: "Horizontal span", dim: "length", default: "100" },
      { kind: "number", key: "b_mils", label: "Vertical span", dim: "length", default: "100" },
    ],
    outputs: [{ key: "diagonal_mils", label: "Diagonal", unit: "mil", primary: true }],
  },
  {
    id: "spacing",
    name: "Conductor Spacing",
    group: "Via & Padstack",
    blurb: "Minimum electrical clearance per the IPC-2221 voltage tables.",
    fields: [
      { kind: "number", key: "voltage", label: "Voltage across gap", dim: "plain", default: "100", help: "V peak" },
      {
        kind: "select",
        key: "device_type",
        label: "Device category",
        default: "b1",
        options: [
          { value: "b1", label: "B1 — Internal conductors" },
          { value: "b2", label: "B2 — External, uncoated, sea level to 3050 m" },
          { value: "b3", label: "B3 — External, uncoated, over 3050 m" },
          { value: "b4", label: "B4 — External, permanent polymer coating" },
          { value: "b5", label: "B5 — External, conformal coating over assembly" },
          { value: "a6", label: "A6 — External component lead/termination, uncoated" },
          { value: "a7", label: "A7 — External component lead, uncoated, over 3050 m" },
          { value: "a8", label: "A8 — External component lead/termination, coated" },
        ],
      },
    ],
    outputs: [
      { key: "spacing_mils", label: "Minimum spacing", unit: "mil", primary: true },
      { key: "spacing_mm", label: "Minimum spacing", unit: "mm", precision: 6 },
    ],
  },

  // ================= Signal integrity =================
  {
    id: "crosstalk",
    name: "Crosstalk (NEXT)",
    group: "Signal Integrity",
    blurb: "Backward crosstalk estimate between parallel microstrip traces.",
    erField: "er",
    fields: [
      { kind: "number", key: "rise_time_ns", label: "Signal rise time", dim: "plain", default: "1", help: "ns" },
      { kind: "number", key: "voltage", label: "Signal voltage", dim: "plain", default: "5", help: "V" },
      { kind: "number", key: "coupled_length_mils", label: "Coupled length", dim: "length", default: "250" },
      { kind: "number", key: "spacing_mils", label: "Edge-to-edge spacing", dim: "length", default: "10" },
      { kind: "number", key: "height_mils", label: "Dielectric height", dim: "length", default: "30" },
      { kind: "number", key: "trace_width_mils", label: "Trace width", dim: "length", default: "10" },
      ER,
    ],
    outputs: [
      { key: "crosstalk_db", label: "Crosstalk", unit: "dB", primary: true },
      { key: "coupled_voltage", label: "Coupled voltage", unit: "V" },
      { key: "kb", label: "Kb coefficient", precision: 6 },
      { key: "next_coefficient", label: "NEXT coefficient", precision: 6 },
      { key: "lsat_mils", label: "Saturation length", unit: "mil" },
    ],
  },
  {
    id: "wavelength",
    name: "Wavelength",
    group: "Signal Integrity",
    blurb: "Signal wavelength in a dielectric, with common fractional lengths.",
    fields: [
      { kind: "number", key: "freq_hz", label: "Frequency", dim: "freq", default: "100MHz" },
      { kind: "number", key: "er_eff", label: "Effective Er", dim: "plain", default: "4.0" },
    ],
    outputs: [
      { key: "lambda_inches", label: "Full wavelength λ", unit: "in", primary: true },
      { key: "lambda_half_inches", label: "λ / 2", unit: "in" },
      { key: "lambda_quarter_inches", label: "λ / 4", unit: "in" },
      { key: "lambda_seventh_inches", label: "λ / 7", unit: "in" },
      { key: "lambda_tenth_inches", label: "λ / 10", unit: "in" },
      { key: "lambda_twentieth_inches", label: "λ / 20", unit: "in" },
      { key: "period_ns", label: "Period", unit: "ns" },
    ],
  },
  {
    id: "reactance",
    name: "Reactance",
    group: "Signal Integrity",
    blurb: "Capacitive and inductive reactance, and LC resonant frequency.",
    fields: [
      { kind: "number", key: "freq_hz", label: "Frequency", dim: "freq", default: "1MHz" },
      { kind: "number", key: "capacitance_f", label: "Capacitance", dim: "cap", default: "1uF", optional: true },
      { kind: "number", key: "inductance_h", label: "Inductance", dim: "ind", default: "1mH", optional: true },
    ],
    outputs: [
      { key: "xc_ohms", label: "Xc (capacitive)", unit: "Ω", primary: true },
      { key: "xl_ohms", label: "Xl (inductive)", unit: "Ω", primary: true },
      { key: "f_res_hz", label: "Resonant frequency", unit: "Hz" },
    ],
  },
  {
    id: "inductor",
    name: "Planar Spiral Inductor",
    group: "Signal Integrity",
    blurb: "Planar spiral inductance via the modified Wheeler (Mohan) expression.",
    fields: [
      { kind: "number", key: "n_turns", label: "Number of turns", dim: "plain", default: "5" },
      { kind: "number", key: "width_mils", label: "Trace width", dim: "length", default: "10" },
      { kind: "number", key: "spacing_mils", label: "Turn spacing", dim: "length", default: "10" },
      { kind: "number", key: "dout_mils", label: "Outer diameter", dim: "length", default: "350" },
      {
        kind: "select",
        key: "shape",
        label: "Geometry",
        default: "square",
        options: [
          { value: "square", label: "Square" },
          { value: "hexagonal", label: "Hexagonal" },
          { value: "octagonal", label: "Octagonal" },
          { value: "circle", label: "Circular" },
        ],
      },
    ],
    outputs: [
      { key: "inductance_nh", label: "Inductance", unit: "nH", primary: true },
      { key: "din_mils", label: "Inner diameter", unit: "mil" },
      { key: "d_avg_mils", label: "Average diameter", unit: "mil" },
      { key: "rho", label: "Fill factor ρ", precision: 6 },
    ],
  },
  {
    id: "pdn",
    name: "PDN Impedance",
    group: "Signal Integrity",
    blurb: "Power distribution network target impedance and plane capacitance.",
    erField: "er",
    fields: [
      { kind: "number", key: "v_supply", label: "Supply voltage", dim: "plain", default: "5", help: "V" },
      { kind: "number", key: "i_max", label: "Maximum current", dim: "plain", default: "2", help: "A" },
      { kind: "number", key: "i_step_pct", label: "Transient step", dim: "plain", default: "50", help: "% of max current" },
      { kind: "number", key: "v_ripple_pct", label: "Allowed ripple", dim: "plain", default: "5", help: "% of supply" },
      { kind: "number", key: "area_sq_in", label: "Plane area", dim: "plain", default: "5", help: "sq in" },
      { kind: "number", key: "d_mils", label: "Plane separation", dim: "length", default: "2" },
      ER,
      { kind: "number", key: "freq_mhz", label: "Frequency", dim: "plain", default: "1", help: "MHz — 0 skips Xc" },
    ],
    outputs: [
      { key: "z_target_ohms", label: "Target impedance", unit: "Ω", primary: true, precision: 6 },
      { key: "c_plane_pf", label: "Plane capacitance", unit: "pF" },
      { key: "xc_ohms", label: "Capacitive reactance", unit: "Ω", precision: 6 },
    ],
  },

  // ================= Ohm's law =================
  {
    id: "ohms_law.eir",
    name: "Ohm's Law (E-I-R)",
    group: "Ohm's Law",
    blurb: "Enter exactly two of voltage, current and resistance.",
    fields: [
      { kind: "number", key: "voltage_v", label: "Voltage", dim: "plain", default: "12", optional: true, help: "V" },
      { kind: "number", key: "current_a", label: "Current", dim: "plain", default: "1", optional: true, help: "A" },
      { kind: "number", key: "resistance_ohm", label: "Resistance", dim: "plain", default: "", optional: true, help: "Ω" },
    ],
    outputs: [
      { key: "voltage_v", label: "Voltage", unit: "V", primary: true },
      { key: "current_a", label: "Current", unit: "A", primary: true },
      { key: "resistance_ohm", label: "Resistance", unit: "Ω", primary: true },
      { key: "power_w", label: "Power", unit: "W" },
    ],
  },
  {
    id: "ohms_law.led_bias",
    name: "LED Bias Resistor",
    group: "Ohm's Law",
    blurb: "Series resistor and its dissipation for a given LED operating point.",
    fields: [
      { kind: "number", key: "supply_v", label: "Supply voltage", dim: "plain", default: "12", help: "V" },
      { kind: "number", key: "led_v", label: "LED forward voltage", dim: "plain", default: "2", help: "V" },
      { kind: "number", key: "led_current_a", label: "LED current", dim: "plain", default: "0.01", help: "A" },
    ],
    outputs: [
      { key: "resistance_ohm", label: "Series resistor", unit: "Ω", primary: true },
      { key: "power_w", label: "Resistor dissipation", unit: "W" },
    ],
  },
  {
    id: "ohms_law.pi_pad",
    name: "Pi-Pad Attenuator",
    group: "Ohm's Law",
    blurb: "Matched Pi attenuator: one series arm, two shunt arms.",
    fields: [
      { kind: "number", key: "attenuation_db", label: "Attenuation", dim: "plain", default: "6", help: "dB" },
      { kind: "number", key: "z_ohm", label: "System impedance", dim: "plain", default: "50", help: "Ω" },
    ],
    outputs: [
      { key: "r_series_ohm", label: "Series resistor", unit: "Ω", primary: true },
      { key: "r_shunt_ohm", label: "Shunt resistors (×2)", unit: "Ω", primary: true },
      { key: "k", label: "Voltage ratio K", precision: 6 },
      { key: "attenuation_db", label: "Attenuation", unit: "dB" },
    ],
  },
  {
    id: "ohms_law.t_pad",
    name: "T-Pad Attenuator",
    group: "Ohm's Law",
    blurb: "Matched T attenuator: two series arms, one shunt arm.",
    fields: [
      { kind: "number", key: "attenuation_db", label: "Attenuation", dim: "plain", default: "6", help: "dB" },
      { kind: "number", key: "z_ohm", label: "System impedance", dim: "plain", default: "50", help: "Ω" },
    ],
    outputs: [
      { key: "r_series_ohm", label: "Series resistors (×2)", unit: "Ω", primary: true },
      { key: "r_shunt_ohm", label: "Shunt resistor", unit: "Ω", primary: true },
      { key: "k", label: "Voltage ratio K", precision: 6 },
      { key: "attenuation_db", label: "Attenuation", unit: "dB" },
    ],
  },
  {
    id: "ohms_law.resistors_series",
    name: "Resistors in Series",
    group: "Ohm's Law",
    blurb: "Total resistance of resistors in series.",
    fields: [{ kind: "list", key: "values", label: "Resistances", dim: "plain", default: "100, 220, 470", help: "Ω — comma separated" }],
    outputs: COMBO_OUT("resistance_ohm", "Total resistance", "Ω"),
  },
  {
    id: "ohms_law.resistors_parallel",
    name: "Resistors in Parallel",
    group: "Ohm's Law",
    blurb: "Equivalent resistance of resistors in parallel.",
    fields: [{ kind: "list", key: "values", label: "Resistances", dim: "plain", default: "100, 100", help: "Ω — comma separated" }],
    outputs: COMBO_OUT("resistance_ohm", "Equivalent resistance", "Ω"),
  },
  {
    id: "ohms_law.capacitors_series",
    name: "Capacitors in Series",
    group: "Ohm's Law",
    blurb: "Equivalent capacitance of capacitors in series.",
    fields: [{ kind: "list", key: "values", label: "Capacitances", dim: "plain", default: "10e-9, 10e-9", help: "Farads — comma separated" }],
    outputs: COMBO_OUT("capacitance_f", "Equivalent capacitance", "F"),
  },
  {
    id: "ohms_law.capacitors_parallel",
    name: "Capacitors in Parallel",
    group: "Ohm's Law",
    blurb: "Total capacitance of capacitors in parallel.",
    fields: [{ kind: "list", key: "values", label: "Capacitances", dim: "plain", default: "10e-9, 10e-9", help: "Farads — comma separated" }],
    outputs: COMBO_OUT("capacitance_f", "Total capacitance", "F"),
  },
  {
    id: "ohms_law.inductors_series",
    name: "Inductors in Series",
    group: "Ohm's Law",
    blurb: "Total inductance of inductors in series.",
    fields: [{ kind: "list", key: "values", label: "Inductances", dim: "plain", default: "10e-6, 10e-6", help: "Henries — comma separated" }],
    outputs: COMBO_OUT("inductance_h", "Total inductance", "H"),
  },
  {
    id: "ohms_law.inductors_parallel",
    name: "Inductors in Parallel",
    group: "Ohm's Law",
    blurb: "Equivalent inductance of inductors in parallel.",
    fields: [{ kind: "list", key: "values", label: "Inductances", dim: "plain", default: "10e-6, 10e-6", help: "Henries — comma separated" }],
    outputs: COMBO_OUT("inductance_h", "Equivalent inductance", "H"),
  },

  // ================= Crystal / PPM =================
  {
    id: "ppm.hz_to_ppm",
    name: "Hz to PPM",
    group: "Crystal & PPM",
    blurb: "Frequency deviation expressed in parts per million.",
    fields: [
      { kind: "number", key: "center_hz", label: "Center frequency", dim: "freq", default: "32000" },
      { kind: "number", key: "max_hz", label: "Maximum frequency", dim: "freq", default: "32001" },
    ],
    outputs: [
      { key: "ppm", label: "Deviation", unit: "ppm", primary: true },
      { key: "variation_hz", label: "Variation", unit: "Hz" },
    ],
  },
  {
    id: "ppm.ppm_to_hz",
    name: "PPM to Hz",
    group: "Crystal & PPM",
    blurb: "Frequency limits implied by a PPM tolerance.",
    fields: [
      { kind: "number", key: "center_hz", label: "Center frequency", dim: "freq", default: "50MHz" },
      { kind: "number", key: "ppm", label: "Tolerance", dim: "plain", default: "25", help: "ppm" },
    ],
    outputs: [
      { key: "variation_hz", label: "Variation", unit: "Hz", primary: true },
      { key: "max_hz", label: "Maximum frequency", unit: "Hz" },
      { key: "min_hz", label: "Minimum frequency", unit: "Hz" },
    ],
  },
  {
    id: "ppm.xtal_load",
    name: "Crystal Load Capacitance",
    group: "Crystal & PPM",
    blurb: "Load capacitance from the two load caps plus stray capacitance.",
    fields: [
      { kind: "number", key: "c1_f", label: "C1", dim: "cap", default: "14pF" },
      { kind: "number", key: "c2_f", label: "C2", dim: "cap", default: "14pF" },
      { kind: "number", key: "c_stray_f", label: "Stray capacitance", dim: "cap", default: "3pF" },
    ],
    outputs: [
      { key: "c_load_calc_f", label: "Calculated load", unit: "F", primary: true },
      { key: "c_load_rule_of_thumb_f", label: "Rule of thumb", unit: "F" },
    ],
  },

  // ================= Reference =================
  {
    id: "wire_gauge",
    name: "Wire Gauge (AWG)",
    group: "Reference",
    blurb: "Standard AWG wire properties, 4/0 through 40.",
    fields: [
      {
        kind: "select",
        key: "awg",
        label: "AWG gauge",
        default: "22",
        // Populated at runtime from the backend; these are a static fallback.
        options: [{ value: "22", label: "22" }],
      },
    ],
    outputs: [
      { key: "diameter_mils", label: "Diameter", unit: "mil", primary: true },
      { key: "diameter_in", label: "Diameter", unit: "in", precision: 6 },
      { key: "area_circular_mils", label: "Cross section", unit: "circular mil" },
      { key: "resistance_ohm_per_kft", label: "Resistance", unit: "Ω/1000 ft", precision: 6 },
      { key: "area_saturn", label: "Saturn display area" },
    ],
  },
];

export const GROUPS = Array.from(new Set(CALCULATORS.map((c) => c.group)));

export function findCalculator(id: string): Calculator | undefined {
  return CALCULATORS.find((c) => c.id === id);
}
