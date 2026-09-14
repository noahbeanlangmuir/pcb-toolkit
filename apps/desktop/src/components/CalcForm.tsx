import { useEffect, useMemo, useState } from "react";
import type { Calculator, Field } from "../schema/calculators";
import { CANONICAL_UNIT, UnitError, parseValue } from "../units";

export type Params = Record<string, number | boolean | string | null>;

type Props = {
  calc: Calculator;
  materials: { name: string; er: number; tg: number | null }[];
  awgList: string[];
  onChange: (params: Params | null, errors: Record<string, string>) => void;
};

/** Initial raw (string/boolean) state for a calculator's fields. */
function initialState(calc: Calculator): Record<string, string | boolean> {
  const out: Record<string, string | boolean> = {};
  for (const f of calc.fields) {
    out[f.key] = f.kind === "bool" ? f.default : (f.default ?? "");
  }
  return out;
}

export function CalcForm({ calc, materials, awgList, onChange }: Props) {
  const [raw, setRaw] = useState<Record<string, string | boolean>>(() => initialState(calc));
  const [material, setMaterial] = useState("");

  // Reset when switching calculators.
  useEffect(() => {
    setRaw(initialState(calc));
    setMaterial("");
  }, [calc.id]);

  const { params, errors } = useMemo(() => {
    const p: Params = {};
    const errs: Record<string, string> = {};

    for (const f of calc.fields) {
      const v = raw[f.key];

      if (f.kind === "bool") {
        p[f.key] = Boolean(v);
        continue;
      }
      if (f.kind === "select") {
        p[f.key] = String(v ?? f.default);
        continue;
      }
      if (f.kind === "list") {
        const text = String(v ?? "").trim();
        if (!text) {
          errs[f.key] = "at least one value is required";
          continue;
        }
        p[f.key] = text;
        continue;
      }

      // number
      const text = String(v ?? "").trim();
      if (!text) {
        if (!f.optional) errs[f.key] = "required";
        else p[f.key] = null;
        continue;
      }
      try {
        p[f.key] = parseValue(text, f.dim ?? "plain");
      } catch (e) {
        errs[f.key] = e instanceof UnitError ? e.message : String(e);
      }
    }

    return { params: p, errors: errs };
  }, [calc, raw]);

  useEffect(() => {
    onChange(Object.keys(errors).length ? null : params, errors);
  }, [params, errors]);

  const set = (key: string, value: string | boolean) =>
    setRaw((prev) => ({ ...prev, [key]: value }));

  const applyMaterial = (name: string) => {
    setMaterial(name);
    const m = materials.find((x) => x.name === name);
    if (m && calc.erField) set(calc.erField, String(m.er));
  };

  return (
    <form className="calc-form" onSubmit={(e) => e.preventDefault()}>
      {calc.erField && materials.length > 0 && (
        <label className="field">
          <span className="field-label">Substrate material</span>
          <select
            className="input"
            value={material}
            onChange={(e) => applyMaterial(e.target.value)}
          >
            <option value="">Custom / manual Er…</option>
            {materials.map((m) => (
              <option key={m.name} value={m.name}>
                {m.name} — Er {m.er}
                {m.tg != null ? ` · Tg ${m.tg}°C` : ""}
              </option>
            ))}
          </select>
          <span className="field-help">Fills the dielectric constant below</span>
        </label>
      )}

      {calc.fields.map((f) => (
        <FieldInput
          key={f.key}
          field={f}
          value={raw[f.key]}
          error={errors[f.key]}
          awgList={awgList}
          onChange={(v) => set(f.key, v)}
        />
      ))}
    </form>
  );
}

function FieldInput({
  field,
  value,
  error,
  awgList,
  onChange,
}: {
  field: Field;
  value: string | boolean | undefined;
  error?: string;
  awgList: string[];
  onChange: (v: string | boolean) => void;
}) {
  if (field.kind === "bool") {
    return (
      <label className="field field-check">
        <input
          type="checkbox"
          checked={Boolean(value)}
          onChange={(e) => onChange(e.target.checked)}
        />
        <span className="field-label">{field.label}</span>
        {field.help && <span className="field-help">{field.help}</span>}
      </label>
    );
  }

  if (field.kind === "select") {
    // The AWG dropdown is populated from the backend at runtime.
    const options =
      field.key === "awg" && awgList.length
        ? awgList.map((a) => ({ value: a, label: `AWG ${a}` }))
        : field.options;
    return (
      <label className="field">
        <span className="field-label">{field.label}</span>
        <select
          className="input"
          value={String(value ?? field.default)}
          onChange={(e) => onChange(e.target.value)}
        >
          {options.map((o) => (
            <option key={o.value} value={o.value}>
              {o.label}
            </option>
          ))}
        </select>
        {field.help && <span className="field-help">{field.help}</span>}
      </label>
    );
  }

  const unit =
    field.kind === "number" ? CANONICAL_UNIT[field.dim ?? "plain"] : "";

  return (
    <label className={`field${error ? " has-error" : ""}`}>
      <span className="field-label">
        {field.label}
        {field.kind === "number" && field.optional && (
          <span className="field-optional">optional</span>
        )}
      </span>
      <div className="input-wrap">
        <input
          className="input"
          type="text"
          inputMode="decimal"
          spellCheck={false}
          value={String(value ?? "")}
          placeholder={field.kind === "list" ? "e.g. 100, 220, 470" : unit || "value"}
          onChange={(e) => onChange(e.target.value)}
        />
        {unit && field.kind === "number" && <span className="input-unit">{unit}</span>}
      </div>
      {error ? (
        <span className="field-error">{error}</span>
      ) : (
        field.help && <span className="field-help">{field.help}</span>
      )}
    </label>
  );
}
