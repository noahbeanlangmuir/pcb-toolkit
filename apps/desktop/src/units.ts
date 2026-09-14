/**
 * Unit-aware parsing and formatting.
 *
 * The library computes in canonical units (mils, Hz, Farads, Henries), and the CLI lets
 * you type `0.254mm` or `1GHz` and converts at the boundary. The GUI does the same, so a
 * field accepts either a bare number in its canonical unit or a value with a suffix.
 */

export type Dimension =
  | "length"
  | "freq"
  | "cap"
  | "ind"
  | "temp"
  | "time"
  | "plain";

/** Suffix -> multiplier into the canonical unit for that dimension. */
const FACTORS: Record<Exclude<Dimension, "temp" | "plain">, Record<string, number>> = {
  // canonical: mils
  length: {
    "": 1,
    mil: 1,
    mils: 1,
    th: 1,
    mm: 1 / 0.0254,
    cm: 10 / 0.0254,
    m: 1000 / 0.0254,
    in: 1000,
    inch: 1000,
    inches: 1000,
    um: 1 / 25.4,
    "µm": 1 / 25.4,
    micron: 1 / 25.4,
  },
  // canonical: Hz
  freq: { "": 1, hz: 1, khz: 1e3, mhz: 1e6, ghz: 1e9, thz: 1e12 },
  // canonical: Farads
  cap: { "": 1, f: 1, mf: 1e-3, uf: 1e-6, "µf": 1e-6, nf: 1e-9, pf: 1e-12 },
  // canonical: Henries
  ind: { "": 1, h: 1, mh: 1e-3, uh: 1e-6, "µh": 1e-6, nh: 1e-9, ph: 1e-12 },
  // canonical: seconds
  time: { "": 1, s: 1, sec: 1, ms: 1e-3, us: 1e-6, "µs": 1e-6, ns: 1e-9, ps: 1e-12 },
};

/** The unit a bare (suffix-less) number is interpreted as. */
export const CANONICAL_UNIT: Record<Dimension, string> = {
  length: "mil",
  freq: "Hz",
  cap: "F",
  ind: "H",
  temp: "°C",
  time: "s",
  plain: "",
};

export class UnitError extends Error {}

/** Split "0.254mm" into [0.254, "mm"]. Handles signs and scientific notation. */
function split(raw: string): [number, string] {
  const s = raw.trim();
  if (!s) throw new UnitError("value is required");

  const m = s.match(/^([+-]?(?:\d+\.?\d*|\.\d+)(?:[eE][+-]?\d+)?)\s*(.*)$/);
  if (!m) throw new UnitError(`"${raw}" is not a number`);

  const value = Number(m[1]);
  if (!Number.isFinite(value)) throw new UnitError(`"${raw}" is not a finite number`);
  return [value, m[2].trim()];
}

/**
 * Parse a user-entered value into the canonical unit for `dim`.
 * Throws {@link UnitError} with a message suitable for showing next to the field.
 */
export function parseValue(raw: string, dim: Dimension = "plain"): number {
  const [value, suffix] = split(raw);

  if (dim === "plain") {
    if (suffix) throw new UnitError(`"${suffix}" is not expected here`);
    return value;
  }

  if (dim === "temp") {
    const u = suffix.replace(/^°/, "").toLowerCase();
    if (u === "" || u === "c" || u === "degc") return value;
    if (u === "f" || u === "degf") return (value - 32) / 1.8;
    if (u === "k") return value - 273.15;
    throw new UnitError(`unknown temperature unit "${suffix}"`);
  }

  const table = FACTORS[dim];
  const key = suffix.toLowerCase().replace(/µ/g, "µ");
  const factor = table[key] ?? table[suffix.toLowerCase()];
  if (factor === undefined) {
    throw new UnitError(`unknown unit "${suffix}" for a ${dim} value`);
  }
  return value * factor;
}

/** Format a number for display: fixed decimals, but falls back to exponent when tiny/huge. */
export function formatNumber(v: number, precision = 4): string {
  if (!Number.isFinite(v)) return String(v);
  if (v === 0) return "0";
  const abs = Math.abs(v);
  if (abs >= 1e6 || abs < 1e-4) return v.toExponential(precision);
  return v.toLocaleString(undefined, {
    minimumFractionDigits: 0,
    maximumFractionDigits: precision,
  });
}
