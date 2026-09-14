import { useState } from "react";
import type { Calculator } from "../schema/calculators";
import { formatNumber } from "../units";

type Props = {
  calc: Calculator;
  result: Record<string, unknown> | null;
  error: string | null;
  pending: boolean;
};

export function Results({ calc, result, error, pending }: Props) {
  const [copied, setCopied] = useState(false);

  if (error) {
    return (
      <div className="results">
        <div className="result-error">
          <strong>Cannot compute</strong>
          <p>{error}</p>
        </div>
      </div>
    );
  }

  if (!result) {
    return (
      <div className="results">
        <div className="result-empty">
          {pending ? "Calculating…" : "Enter values to see results."}
        </div>
      </div>
    );
  }

  // Outputs the backend actually returned. Optional values (Xc, f_res) come back null
  // when their inputs were omitted, so they are skipped rather than shown as "null".
  const rows = calc.outputs
    .map((o) => ({ ...o, value: result[o.key] }))
    .filter((o) => o.value !== undefined && o.value !== null);

  const primary = rows.filter((r) => r.primary);
  const secondary = rows.filter((r) => !r.primary);

  const copy = async () => {
    const text = rows
      .map((r) => `${r.label}: ${render(r.value, r.precision)}${r.unit ? ` ${r.unit}` : ""}`)
      .join("\n");
    try {
      await navigator.clipboard.writeText(`${calc.name}\n${text}`);
      setCopied(true);
      setTimeout(() => setCopied(false), 1400);
    } catch {
      /* clipboard unavailable — ignore */
    }
  };

  return (
    <div className="results">
      <div className="results-head">
        <h2>Results</h2>
        <button type="button" className="copy-btn" onClick={copy}>
          {copied ? "Copied" : "Copy"}
        </button>
      </div>

      {primary.length > 0 && (
        <div className="result-primary">
          {primary.map((r) => (
            <div className="result-hero" key={r.key}>
              <span className="hero-label">{r.label}</span>
              <span className="hero-value">
                {render(r.value, r.precision)}
                {r.unit && <span className="hero-unit">{r.unit}</span>}
              </span>
            </div>
          ))}
        </div>
      )}

      {secondary.length > 0 && (
        <dl className="result-grid">
          {secondary.map((r) => (
            <div className="result-row" key={r.key}>
              <dt>{r.label}</dt>
              <dd>
                {render(r.value, r.precision)}
                {r.unit && <span className="row-unit">{r.unit}</span>}
              </dd>
            </div>
          ))}
        </dl>
      )}
    </div>
  );
}

function render(value: unknown, precision?: number): string {
  if (typeof value === "number") return formatNumber(value, precision ?? 4);
  if (typeof value === "string") return value;
  if (typeof value === "boolean") return value ? "yes" : "no";
  return String(value);
}
