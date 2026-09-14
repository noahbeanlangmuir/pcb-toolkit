# pcb-toolkit-cli

Command-line interface for PCB design calculations. Wraps the
[pcb-toolkit](https://crates.io/crates/pcb-toolkit) library.

Part of the [pcb-toolkit](https://github.com/akiselev/pcb-toolkit) workspace.

## Installation

```
cargo install pcb-toolkit-cli
```

The binary is called `pcb-toolkit`.

## Usage

### Microstrip Impedance

```
pcb-toolkit impedance microstrip -w 10 --height 5 --er 4.6
```

```
Microstrip Impedance
────────────────────
  Zo      = 44.8322 Ω
  Er_eff  = 3.3075
  Tpd     = 154.0839 ps/in
  Lo      = 6.9079 nH/in
  Co      = 3.4369 pF/in
```

Options:

| Flag | Description | Default |
|---|---|---|
| `-w`, `--width` | Conductor width (mils) | Required |
| `--height` | Dielectric height (mils) | Required |
| `-t`, `--thickness` | Conductor thickness (mils) | 1.4 (1 oz copper) |
| `--er` | Substrate relative permittivity | 4.6 (FR-4) |
| `-f`, `--freq-mhz` | Frequency for dispersion correction (MHz) | 0 (static) |

### JSON Output

All subcommands support the `--json` flag for machine-readable output:

```
pcb-toolkit impedance microstrip -w 10 --height 5 --er 4.6 --json
```

```json
{
  "zo": 44.83223672585572,
  "er_eff": 3.307496138618307,
  "tpd_ps_per_in": 154.08390124056288,
  "lo_nh_per_in": 6.907925936060289,
  "co_pf_per_in": 3.436899706404774
}
```

### Help

```
pcb-toolkit --help
pcb-toolkit impedance --help
pcb-toolkit impedance microstrip --help
```

## Available Subcommands

| Subcommand | Description | Status |
|---|---|---|
| `impedance microstrip` | Microstrip impedance calculation | Available |

Additional subcommands will be added as the underlying library modules are
implemented.

## License

Licensed under either of [Apache License, Version 2.0](../../LICENSE-APACHE) or
[MIT License](../../LICENSE-MIT) at your option.
