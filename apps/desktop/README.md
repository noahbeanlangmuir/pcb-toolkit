# PCB Toolkit — Desktop

A desktop GUI for the [`pcb-toolkit`](../../crates/pcb-toolkit) calculator library, in the
spirit of Saturn PCB Toolkit but with a modern interface. Built with
[Tauri 2](https://tauri.app), React and TypeScript.

All 37 calculators are exposed, grouped into Impedance, Differential Pairs, Current &
Thermal, Via & Padstack, Signal Integrity, Ohm's Law, Crystal & PPM, and Reference.

## Running

```bash
cd apps/desktop
npm install
npm run tauri dev
```

> **Don't launch `target/debug/pcb-toolkit-desktop.exe` directly.** In a debug build Tauri
> loads the `devUrl` dev server rather than the embedded frontend, so the window opens on a
> "can't reach this page" error unless Vite is already running. Use `npm run tauri dev`,
> which starts both — or build in release mode, which embeds the bundle.

## Building installers

```bash
npm run tauri build
```

Artifacts land in `../../target/release/bundle/` — MSI and NSIS on Windows, `.deb`/AppImage
on Linux, `.dmg` on macOS.

### Prerequisites

Rust plus the platform WebView. Windows needs the WebView2 runtime (present by default on
Windows 11); Linux needs `libwebkit2gtk-4.1-dev` and friends — see the
`Install Tauri system dependencies` step in `.github/workflows/ci.yml` for the exact list.

## How it is wired

The GUI is **schema-driven**. Adding or changing a calculator is a data change in two
places rather than new components:

| File | Role |
| ---- | ---- |
| [`src/schema/calculators.ts`](src/schema/calculators.ts) | One entry per calculator: fields, units, defaults, outputs |
| [`src-tauri/src/lib.rs`](src-tauri/src/lib.rs) | One `match` arm per calculator id, calling the library |

The frontend sends `{ id, params }` to a single `calculate` command and renders whatever
comes back, so there is no per-calculator UI code. A calculator's `id` in the schema must
match its dispatch arm, and each field `key` must match the parameter name that arm reads.
A Rust test (`every_calculator_dispatches_and_returns_its_primary_output`) exercises all 37
arms and asserts each returns the field the UI reads, so drift between the two fails CI.

Two further commands supply reference data: `list_materials` (the 45-entry substrate
database, used by the Er picker) and `list_awg` (wire gauge dropdown).

## Units

Numeric fields accept a unit suffix, exactly like the CLI — `0.254mm`, `1GHz`, `10nF`,
`1.5in`. A bare number is read as the field's canonical unit, shown greyed inside the
input. Conversion happens in [`src/units.ts`](src/units.ts) before the value is sent, so
the backend always receives canonical units (mils, Hz, Farads, Henries).

## Accuracy caveats

The GUI presents every calculator identically. Some are less well validated than others —
notably broadside-coupled differential, crosstalk, the embedded-microstrip Er_eff blend,
and the IPC-2152 modifier approximations. See [`VALIDATION.md`](../../VALIDATION.md) at the
repository root for the per-module confidence assessment.
