import { useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

import { CALCULATORS, GROUPS, type Calculator } from "./schema/calculators";
import { CalcForm, type Params } from "./components/CalcForm";
import { Results } from "./components/Results";

type Material = { name: string; er: number; tg: number | null };

export default function App() {
  const [activeId, setActiveId] = useState(CALCULATORS[0].id);
  const [query, setQuery] = useState("");
  const [params, setParams] = useState<Params | null>(null);
  const [result, setResult] = useState<Record<string, unknown> | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [pending, setPending] = useState(false);
  const [materials, setMaterials] = useState<Material[]>([]);
  const [awgList, setAwgList] = useState<string[]>([]);
  const [theme, setTheme] = useState<"dark" | "light">(
    () => (localStorage.getItem("pcb-theme") as "dark" | "light") ?? "dark",
  );

  const calc = useMemo(
    () => CALCULATORS.find((c) => c.id === activeId) ?? CALCULATORS[0],
    [activeId],
  );

  // Clear the previous calculator's state on switch. Without this, the effect below can
  // fire once with the new id and the old parameters before the form reports its defaults.
  const selectCalculator = (id: string) => {
    setActiveId(id);
    setParams(null);
    setResult(null);
    setError(null);
  };

  useEffect(() => {
    document.documentElement.dataset.theme = theme;
    try {
      localStorage.setItem("pcb-theme", theme);
    } catch {
      /* storage may be unavailable */
    }
  }, [theme]);

  // One-time reference data for the dropdowns.
  useEffect(() => {
    invoke<Material[]>("list_materials").then(setMaterials).catch(() => setMaterials([]));
    invoke<string[]>("list_awg").then(setAwgList).catch(() => setAwgList([]));
  }, []);

  // Recompute whenever the parameters change. A request id guards against an older
  // in-flight call resolving after a newer one.
  const reqId = useRef(0);
  useEffect(() => {
    if (!params) {
      setResult(null);
      setError(null);
      return;
    }
    const id = ++reqId.current;
    setPending(true);
    invoke<Record<string, unknown>>("calculate", { id: calc.id, params })
      .then((r) => {
        if (id !== reqId.current) return;
        setResult(r);
        setError(null);
      })
      .catch((e) => {
        if (id !== reqId.current) return;
        setResult(null);
        setError(typeof e === "string" ? e : String(e));
      })
      .finally(() => {
        if (id === reqId.current) setPending(false);
      });
  }, [calc.id, params]);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return CALCULATORS;
    return CALCULATORS.filter(
      (c) =>
        c.name.toLowerCase().includes(q) ||
        c.group.toLowerCase().includes(q) ||
        c.blurb.toLowerCase().includes(q),
    );
  }, [query]);

  const groups = GROUPS.filter((g) => filtered.some((c) => c.group === g));

  return (
    <div className="app">
      <aside className="sidebar">
        <div className="brand">
          <div className="brand-mark" aria-hidden="true" />
          <div>
            <div className="brand-name">PCB Toolkit</div>
            <div className="brand-sub">{CALCULATORS.length} calculators</div>
          </div>
        </div>

        <input
          className="search"
          type="search"
          placeholder="Search calculators…"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />

        <nav className="nav">
          {groups.map((group) => (
            <div className="nav-group" key={group}>
              <div className="nav-group-title">{group}</div>
              {filtered
                .filter((c) => c.group === group)
                .map((c) => (
                  <button
                    key={c.id}
                    type="button"
                    className={`nav-item${c.id === activeId ? " active" : ""}`}
                    onClick={() => selectCalculator(c.id)}
                  >
                    {c.name}
                  </button>
                ))}
            </div>
          ))}
          {filtered.length === 0 && <div className="nav-empty">No matches</div>}
        </nav>

        <button
          type="button"
          className="theme-toggle"
          onClick={() => setTheme(theme === "dark" ? "light" : "dark")}
        >
          {theme === "dark" ? "Light mode" : "Dark mode"}
        </button>
      </aside>

      <main className="main">
        <Header calc={calc} />
        <div className="panels">
          <section className="panel panel-inputs">
            <h2>Inputs</h2>
            <CalcForm
              calc={calc}
              materials={materials}
              awgList={awgList}
              onChange={(p) => setParams(p)}
            />
          </section>
          <section className="panel panel-results">
            <Results calc={calc} result={result} error={error} pending={pending} />
          </section>
        </div>
      </main>
    </div>
  );
}

function Header({ calc }: { calc: Calculator }) {
  return (
    <header className="header">
      <div className="header-eyebrow">{calc.group}</div>
      <h1>{calc.name}</h1>
      <p>{calc.blurb}</p>
    </header>
  );
}
