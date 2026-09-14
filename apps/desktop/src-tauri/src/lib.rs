//! Tauri backend for the PCB Toolkit desktop app.
//!
//! The whole GUI is schema-driven: the frontend owns a declarative description of every
//! calculator (fields, units, defaults) and sends `{ id, params }` to a single
//! [`calculate`] command. This keeps adding a calculator to a data change on both sides
//! rather than a new command plus new plumbing.
//!
//! Input structs in `pcb-toolkit` deliberately do not derive `Deserialize`, and several
//! entry points take positional arguments rather than a struct, so this layer reads
//! parameters out of a JSON object by name via [`Params`]. Every result type *does*
//! derive `Serialize`, so results go back untouched.

use serde::Serialize;
use serde_json::{Value, json};

use pcb_toolkit::copper::{CopperWeight, EtchFactor, PlatingThickness};
use pcb_toolkit::inductor::SpiralShape;
use pcb_toolkit::spacing::DeviceType;
use pcb_toolkit::wire_gauge::Awg;
use pcb_toolkit::{
    crosstalk, current, differential, fusing, impedance, inductor, materials, ohms_law, padstack,
    pdn, ppm, reactance, spacing, thermal, via, wavelength, wire_gauge,
};

/// Typed accessor over the JSON parameter object sent by the frontend.
struct Params(Value);

impl Params {
    fn get(&self, key: &str) -> Option<&Value> {
        self.0.get(key)
    }

    /// Required number. Accepts a JSON number, or a numeric string (the inputs are text
    /// fields, and an empty one is reported as a missing value rather than a parse error).
    fn f64(&self, key: &str) -> Result<f64, String> {
        match self.get(key) {
            Some(Value::Number(n)) => n.as_f64().ok_or_else(|| format!("`{key}` is not a number")),
            Some(Value::String(s)) if !s.trim().is_empty() => s
                .trim()
                .parse::<f64>()
                .map_err(|_| format!("`{key}`: `{s}` is not a number")),
            _ => Err(format!("`{key}` is required")),
        }
    }

    fn f64_or(&self, key: &str, default: f64) -> f64 {
        self.f64(key).unwrap_or(default)
    }

    /// Optional number — absent, null, or blank all mean "not supplied".
    fn opt_f64(&self, key: &str) -> Option<f64> {
        self.f64(key).ok()
    }

    fn u32(&self, key: &str) -> Result<u32, String> {
        let v = self.f64(key)?;
        if v < 0.0 || v.fract() != 0.0 {
            return Err(format!("`{key}` must be a whole number"));
        }
        Ok(v as u32)
    }

    fn bool_or(&self, key: &str, default: bool) -> bool {
        match self.get(key) {
            Some(Value::Bool(b)) => *b,
            Some(Value::String(s)) => s == "true",
            _ => default,
        }
    }

    fn str_or<'a>(&'a self, key: &str, default: &'a str) -> &'a str {
        match self.get(key) {
            Some(Value::String(s)) if !s.is_empty() => s.as_str(),
            _ => default,
        }
    }

    /// A list of numbers, accepted either as a JSON array or as a comma/whitespace
    /// separated string (the component-list inputs are free text).
    fn f64_list(&self, key: &str) -> Result<Vec<f64>, String> {
        let parse_str = |s: &str| -> Result<Vec<f64>, String> {
            let out: Result<Vec<f64>, String> = s
                .split(|c: char| c == ',' || c.is_whitespace())
                .filter(|t| !t.is_empty())
                .map(|t| {
                    t.parse::<f64>()
                        .map_err(|_| format!("`{t}` is not a number"))
                })
                .collect();
            out
        };
        match self.get(key) {
            Some(Value::Array(a)) => a
                .iter()
                .map(|v| match v {
                    Value::Number(n) => n.as_f64().ok_or_else(|| "not a number".to_string()),
                    Value::String(s) => s
                        .trim()
                        .parse::<f64>()
                        .map_err(|_| format!("`{s}` is not a number")),
                    _ => Err("not a number".to_string()),
                })
                .collect(),
            Some(Value::String(s)) => parse_str(s),
            _ => Err(format!("`{key}` is required")),
        }
    }
}

fn copper_weight(s: &str) -> Result<CopperWeight, String> {
    CopperWeight::from_str_oz(s).map_err(|e| e.to_string())
}

fn plating(s: &str) -> Result<PlatingThickness, String> {
    let k = s.trim().to_ascii_lowercase();
    let k = k.strip_suffix("oz").unwrap_or(&k).trim();
    Ok(match k {
        "bare" | "0" => PlatingThickness::Bare,
        "0.5" => PlatingThickness::Oz05,
        "1" => PlatingThickness::Oz1,
        "1.5" => PlatingThickness::Oz15,
        "2" => PlatingThickness::Oz2,
        "2.5" => PlatingThickness::Oz25,
        "3" => PlatingThickness::Oz3,
        other => return Err(format!("unknown plating thickness `{other}`")),
    })
}

fn etch_factor(s: &str) -> Result<EtchFactor, String> {
    Ok(match s.trim().to_ascii_lowercase().as_str() {
        "none" | "0" => EtchFactor::None,
        "1:1" | "1to1" => EtchFactor::OneToOne,
        "2:1" | "2to1" => EtchFactor::TwoToOne,
        other => return Err(format!("unknown etch factor `{other}`")),
    })
}

fn spiral_shape(s: &str) -> Result<SpiralShape, String> {
    Ok(match s.trim().to_ascii_lowercase().as_str() {
        "square" => SpiralShape::Square,
        "hexagonal" => SpiralShape::Hexagonal,
        "octagonal" => SpiralShape::Octagonal,
        "circle" | "circular" => SpiralShape::Circle,
        other => return Err(format!("unknown spiral shape `{other}`")),
    })
}

fn device_type(s: &str) -> Result<DeviceType, String> {
    Ok(match s.trim().to_ascii_lowercase().as_str() {
        "b1" => DeviceType::B1,
        "b2" => DeviceType::B2,
        "b3" => DeviceType::B3,
        "b4" => DeviceType::B4,
        "b5" => DeviceType::B5,
        "a6" => DeviceType::A6,
        "a7" => DeviceType::A7,
        "a8" => DeviceType::A8,
        other => return Err(format!("unknown device type `{other}`")),
    })
}

fn awg(s: &str) -> Result<Awg, String> {
    let k = s.trim().to_ascii_lowercase();
    let k = k.strip_prefix("awg").unwrap_or(&k).trim();
    Awg::all()
        .iter()
        .copied()
        .find(|a| wire_gauge::lookup(*a).awg_label.eq_ignore_ascii_case(k))
        .ok_or_else(|| format!("unknown AWG gauge `{s}`"))
}

/// Serialize a library result, mapping any serialization failure to a string error.
fn ok<T: Serialize>(value: T) -> Result<Value, String> {
    serde_json::to_value(value).map_err(|e| e.to_string())
}

/// Run one calculator by id. Errors come back as plain strings for display.
#[tauri::command]
fn calculate(id: String, params: Value) -> Result<Value, String> {
    let p = Params(params);
    let e = |err: pcb_toolkit::CalcError| err.to_string();

    match id.as_str() {
        // ---- impedance ----------------------------------------------------------
        "impedance.microstrip" => ok(impedance::microstrip::calculate(
            &impedance::microstrip::MicrostripInput {
                width: p.f64("width")?,
                height: p.f64("height")?,
                thickness: p.f64_or("thickness", 1.4),
                er: p.f64_or("er", 4.6),
                frequency: p.f64_or("frequency", 0.0),
            },
        )
        .map_err(e)?),

        "impedance.stripline" => ok(impedance::stripline::calculate(
            &impedance::stripline::StriplineInput {
                width: p.f64("width")?,
                height: p.f64("height")?,
                thickness: p.f64_or("thickness", 1.4),
                er: p.f64_or("er", 4.6),
            },
        )
        .map_err(e)?),

        "impedance.embedded" => ok(impedance::embedded::calculate(
            &impedance::embedded::EmbeddedMicrostripInput {
                width: p.f64("width")?,
                height: p.f64("height")?,
                thickness: p.f64_or("thickness", 1.4),
                er: p.f64_or("er", 4.6),
                cover_height: p.f64("cover_height")?,
                frequency: p.f64_or("frequency", 0.0),
            },
        )
        .map_err(e)?),

        "impedance.coplanar" => ok(impedance::coplanar::calculate(
            &impedance::coplanar::CoplanarInput {
                width: p.f64("width")?,
                gap: p.f64("gap")?,
                height: p.f64("height")?,
                thickness: p.f64_or("thickness", 1.4),
                er: p.f64_or("er", 4.6),
            },
        )
        .map_err(e)?),

        // ---- differential -------------------------------------------------------
        "differential.edge_coupled_external" => ok(differential::edge_coupled_external::calculate(
            &differential::edge_coupled_external::EdgeCoupledExternalInput {
                width: p.f64("width")?,
                spacing: p.f64("spacing")?,
                height: p.f64("height")?,
                thickness: p.f64_or("thickness", 1.4),
                er: p.f64_or("er", 4.6),
            },
        )
        .map_err(e)?),

        "differential.edge_coupled_internal_sym" => {
            ok(differential::edge_coupled_internal_sym::calculate(
                &differential::edge_coupled_internal_sym::EdgeCoupledInternalSymInput {
                    width: p.f64("width")?,
                    spacing: p.f64("spacing")?,
                    height: p.f64("height")?,
                    thickness: p.f64_or("thickness", 1.4),
                    er: p.f64_or("er", 4.6),
                },
            )
            .map_err(e)?)
        }

        "differential.edge_coupled_internal_asym" => {
            ok(differential::edge_coupled_internal_asym::calculate(
                &differential::edge_coupled_internal_asym::EdgeCoupledInternalAsymInput {
                    width: p.f64("width")?,
                    spacing: p.f64("spacing")?,
                    height1: p.f64("height1")?,
                    height2: p.f64("height2")?,
                    thickness: p.f64_or("thickness", 1.4),
                    er: p.f64_or("er", 4.6),
                },
            )
            .map_err(e)?)
        }

        "differential.edge_coupled_embedded" => ok(differential::edge_coupled_embedded::calculate(
            &differential::edge_coupled_embedded::EdgeCoupledEmbeddedInput {
                width: p.f64("width")?,
                spacing: p.f64("spacing")?,
                height: p.f64("height")?,
                thickness: p.f64_or("thickness", 1.4),
                er: p.f64_or("er", 4.6),
                cover_height: p.f64("cover_height")?,
            },
        )
        .map_err(e)?),

        "differential.broadside_coupled" => ok(differential::broadside_coupled::calculate(
            &differential::broadside_coupled::BroadsideCoupledInput {
                width: p.f64("width")?,
                separation: p.f64("separation")?,
                height_total: p.f64("height_total")?,
                thickness: p.f64_or("thickness", 1.4),
                er: p.f64_or("er", 4.6),
                shielded: p.bool_or("shielded", true),
            },
        )
        .map_err(e)?),

        // ---- current ------------------------------------------------------------
        "current.ipc2221a" => ok(current::calculate(&current::CurrentInput {
            width: p.f64("width")?,
            thickness: p.f64_or("thickness", 1.4),
            length: p.f64("length")?,
            temperature_rise: p.f64_or("temperature_rise", 10.0),
            ambient_temp: p.f64_or("ambient_temp", 25.0),
            frequency: p.f64_or("frequency", 0.0),
            etch_factor: etch_factor(p.str_or("etch_factor", "none"))?,
            is_internal: p.bool_or("is_internal", false),
        })
        .map_err(e)?),

        "current.ipc2152" => ok(current::calculate_ipc2152(&current::Ipc2152Input {
            width: p.f64("width")?,
            thickness: p.f64_or("thickness", 1.4),
            length: p.f64("length")?,
            temperature_rise: p.f64_or("temperature_rise", 10.0),
            ambient_temp: p.f64_or("ambient_temp", 25.0),
            frequency: p.f64_or("frequency", 0.0),
            etch_factor: etch_factor(p.str_or("etch_factor", "none"))?,
            is_internal: p.bool_or("is_internal", false),
            board_thickness_mils: p.f64_or("board_thickness_mils", 62.0),
            has_copper_plane: p.bool_or("has_copper_plane", false),
            material_modifier: p.f64_or("material_modifier", 1.0),
            user_modifier: p.f64_or("user_modifier", 1.0),
        })
        .map_err(e)?),

        // ---- fusing -------------------------------------------------------------
        "fusing.trace" => ok(fusing::fusing_current_trace(
            p.f64("width_mils")?,
            copper_weight(p.str_or("base_copper", "1"))?,
            plating(p.str_or("plating", "bare"))?,
            etch_factor(p.str_or("etch_factor", "none"))?,
            p.f64("time_s")?,
            p.f64_or("ambient_c", 25.0),
        )
        .map_err(e)?),

        "fusing.area" => {
            let amps = fusing::fusing_current(
                p.f64("area_circular_mils")?,
                p.f64("time_s")?,
                p.f64_or("ambient_c", 25.0),
                p.f64_or("melting_temp_c", fusing::COPPER_MELTING_TEMP_C),
            )
            .map_err(e)?;
            Ok(json!({ "fusing_current_a": amps }))
        }

        // ---- via / inductor -----------------------------------------------------
        "via" => ok(via::calculate(&via::ViaInput {
            hole_diameter_mils: p.f64("hole_diameter_mils")?,
            pad_diameter_mils: p.f64("pad_diameter_mils")?,
            antipad_diameter_mils: p.f64("antipad_diameter_mils")?,
            height_mils: p.f64("height_mils")?,
            plating_thickness_mils: p.f64_or("plating_thickness_mils", 0.7),
            er: p.f64_or("er", 4.6),
        })
        .map_err(e)?),

        "inductor" => ok(inductor::planar_spiral(
            p.u32("n_turns")?,
            p.f64("width_mils")?,
            p.f64("spacing_mils")?,
            p.f64("dout_mils")?,
            spiral_shape(p.str_or("shape", "square"))?,
        )
        .map_err(e)?),

        // ---- reactance / wavelength ---------------------------------------------
        "reactance" => ok(reactance::reactance(
            p.f64("freq_hz")?,
            p.opt_f64("capacitance_f"),
            p.opt_f64("inductance_h"),
        )
        .map_err(e)?),

        "wavelength" => ok(
            wavelength::wavelength(p.f64("freq_hz")?, p.f64_or("er_eff", 1.0)).map_err(e)?,
        ),

        // ---- ohm's law ----------------------------------------------------------
        "ohms_law.eir" => ok(ohms_law::eir(
            p.opt_f64("voltage_v"),
            p.opt_f64("current_a"),
            p.opt_f64("resistance_ohm"),
        )
        .map_err(e)?),

        "ohms_law.led_bias" => ok(ohms_law::led_bias(
            p.f64("supply_v")?,
            p.f64("led_v")?,
            p.f64("led_current_a")?,
        )
        .map_err(e)?),

        "ohms_law.pi_pad" => ok(
            ohms_law::pi_pad(p.f64("attenuation_db")?, p.f64_or("z_ohm", 50.0)).map_err(e)?,
        ),
        "ohms_law.t_pad" => ok(
            ohms_law::t_pad(p.f64("attenuation_db")?, p.f64_or("z_ohm", 50.0)).map_err(e)?,
        ),

        "ohms_law.resistors_series" => {
            ok(ohms_law::resistors_series(&p.f64_list("values")?).map_err(e)?)
        }
        "ohms_law.resistors_parallel" => {
            ok(ohms_law::resistors_parallel(&p.f64_list("values")?).map_err(e)?)
        }
        "ohms_law.capacitors_series" => {
            ok(ohms_law::capacitors_series(&p.f64_list("values")?).map_err(e)?)
        }
        "ohms_law.capacitors_parallel" => {
            ok(ohms_law::capacitors_parallel(&p.f64_list("values")?).map_err(e)?)
        }
        "ohms_law.inductors_series" => {
            ok(ohms_law::inductors_series(&p.f64_list("values")?).map_err(e)?)
        }
        "ohms_law.inductors_parallel" => {
            ok(ohms_law::inductors_parallel(&p.f64_list("values")?).map_err(e)?)
        }

        // ---- ppm ----------------------------------------------------------------
        "ppm.hz_to_ppm" => ok(
            ppm::hz_to_ppm(p.f64("center_hz")?, p.f64("max_hz")?).map_err(e)?,
        ),
        "ppm.ppm_to_hz" => ok(ppm::ppm_to_hz(p.f64("center_hz")?, p.f64("ppm")?).map_err(e)?),
        "ppm.xtal_load" => ok(ppm::xtal_load(
            p.f64_or("c_stray_f", 0.0),
            p.f64("c1_f")?,
            p.f64("c2_f")?,
        )
        .map_err(e)?),

        // ---- padstack -----------------------------------------------------------
        "padstack.thru_hole" => ok(padstack::thru_hole(&padstack::ThruHoleInput {
            hole_diameter_mils: p.f64("hole_diameter_mils")?,
            annular_ring_mils: p.f64("annular_ring_mils")?,
            isolation_width_mils: p.f64("isolation_width_mils")?,
        })
        .map_err(e)?),

        "padstack.corner_to_corner" => {
            let d = padstack::corner_to_corner(p.f64("a_mils")?, p.f64("b_mils")?).map_err(e)?;
            Ok(json!({ "diagonal_mils": d }))
        }

        // ---- lookups ------------------------------------------------------------
        "spacing" => ok(spacing::spacing(&spacing::SpacingInput {
            voltage: p.f64("voltage")?,
            device_type: device_type(p.str_or("device_type", "b1"))?,
        })
        .map_err(e)?),

        "wire_gauge" => ok(wire_gauge::lookup(awg(p.str_or("awg", "22"))?)),

        // ---- pdn / thermal / crosstalk ------------------------------------------
        "pdn" => ok(pdn::calculate(&pdn::PdnInput {
            v_supply: p.f64("v_supply")?,
            i_max: p.f64("i_max")?,
            i_step_pct: p.f64("i_step_pct")?,
            v_ripple_pct: p.f64("v_ripple_pct")?,
            area_sq_in: p.f64("area_sq_in")?,
            er: p.f64_or("er", 4.6),
            d_mils: p.f64("d_mils")?,
            freq_mhz: p.f64_or("freq_mhz", 0.0),
        })
        .map_err(e)?),

        "thermal" => ok(thermal::calculate(&thermal::ThermalInput {
            r_theta_ja: p.f64("r_theta_ja")?,
            power_w: p.f64("power_w")?,
            t_ambient_c: p.f64_or("t_ambient_c", 25.0),
        })
        .map_err(e)?),

        "crosstalk" => ok(crosstalk::calculate(&crosstalk::CrosstalkInput {
            rise_time_ns: p.f64("rise_time_ns")?,
            voltage: p.f64("voltage")?,
            coupled_length_mils: p.f64("coupled_length_mils")?,
            spacing_mils: p.f64("spacing_mils")?,
            height_mils: p.f64("height_mils")?,
            er: p.f64_or("er", 4.6),
            trace_width_mils: p.f64("trace_width_mils")?,
        })
        .map_err(e)?),

        other => Err(format!("unknown calculator `{other}`")),
    }
}

#[derive(Serialize)]
struct MaterialEntry {
    name: &'static str,
    er: f64,
    tg: Option<f64>,
}

/// The built-in substrate database, for the Er picker.
#[tauri::command]
fn list_materials() -> Vec<MaterialEntry> {
    materials::MATERIALS
        .iter()
        .map(|m| MaterialEntry {
            name: m.name,
            er: m.er,
            tg: m.tg,
        })
        .collect()
}

/// All AWG gauge labels, largest to smallest, for the wire-gauge dropdown.
#[tauri::command]
fn list_awg() -> Vec<&'static str> {
    Awg::all()
        .iter()
        .map(|a| wire_gauge::lookup(*a).awg_label)
        .collect()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            calculate,
            list_materials,
            list_awg
        ])
        .run(tauri::generate_context!())
        .expect("error while running PCB Toolkit");
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Every id the frontend schema can send, with a representative parameter set.
    /// Keep in sync with `src/schema/calculators.ts`.
    fn cases() -> Vec<(&'static str, Value, &'static str)> {
        vec![
            ("impedance.microstrip", json!({"width":10,"height":5,"thickness":1.4,"er":4.6,"frequency":0}), "zo"),
            ("impedance.stripline", json!({"width":10,"height":20,"thickness":1.4,"er":4.6}), "zo"),
            ("impedance.embedded", json!({"width":10,"height":5,"thickness":1.4,"er":4.6,"cover_height":5,"frequency":0}), "zo"),
            ("impedance.coplanar", json!({"width":10,"gap":8,"height":10,"thickness":1.4,"er":4.6}), "zo"),
            ("differential.edge_coupled_external", json!({"width":10,"spacing":5,"height":15,"thickness":2.1,"er":4.6}), "zdiff"),
            ("differential.edge_coupled_internal_sym", json!({"width":10,"spacing":5,"height":15,"thickness":2.1,"er":4.6}), "zdiff"),
            ("differential.edge_coupled_internal_asym", json!({"width":10,"spacing":5,"height1":15,"height2":10,"thickness":2.1,"er":4.6}), "zdiff"),
            ("differential.edge_coupled_embedded", json!({"width":10,"spacing":5,"height":15,"thickness":2.1,"er":4.6,"cover_height":5}), "zdiff"),
            ("differential.broadside_coupled", json!({"width":10,"separation":5,"height_total":30,"thickness":2.1,"er":4.6,"shielded":true}), "zdiff"),
            ("current.ipc2221a", json!({"width":50,"thickness":1.4,"length":1000,"temperature_rise":10,"ambient_temp":25,"frequency":0,"etch_factor":"none","is_internal":false}), "current_capacity"),
            ("current.ipc2152", json!({"width":50,"thickness":1.4,"length":1000,"temperature_rise":10,"ambient_temp":25,"frequency":0,"etch_factor":"none","is_internal":false,"board_thickness_mils":62,"has_copper_plane":false,"material_modifier":1.0,"user_modifier":1.0}), "current_capacity"),
            ("fusing.trace", json!({"width_mils":10,"base_copper":"1","plating":"bare","etch_factor":"none","time_s":1,"ambient_c":25}), "fusing_current_a"),
            ("fusing.area", json!({"area_circular_mils":23.93,"time_s":1,"ambient_c":25,"melting_temp_c":1084.62}), "fusing_current_a"),
            ("via", json!({"hole_diameter_mils":10,"pad_diameter_mils":20,"antipad_diameter_mils":40,"height_mils":62,"plating_thickness_mils":1,"er":4.6}), "impedance_ohms"),
            ("inductor", json!({"n_turns":5,"width_mils":10,"spacing_mils":10,"dout_mils":350,"shape":"square"}), "inductance_nh"),
            ("reactance", json!({"freq_hz":1e6,"capacitance_f":1e-6,"inductance_h":1e-3}), "xc_ohms"),
            ("wavelength", json!({"freq_hz":1e8,"er_eff":4.0}), "lambda_inches"),
            ("ohms_law.eir", json!({"voltage_v":12,"current_a":1,"resistance_ohm":null}), "power_w"),
            ("ohms_law.led_bias", json!({"supply_v":12,"led_v":2,"led_current_a":0.01}), "resistance_ohm"),
            ("ohms_law.pi_pad", json!({"attenuation_db":6,"z_ohm":50}), "r_series_ohm"),
            ("ohms_law.t_pad", json!({"attenuation_db":6,"z_ohm":50}), "r_series_ohm"),
            ("ohms_law.resistors_series", json!({"values":"100, 220, 470"}), "resistance_ohm"),
            ("ohms_law.resistors_parallel", json!({"values":"100, 100"}), "resistance_ohm"),
            ("ohms_law.capacitors_series", json!({"values":"10e-9, 10e-9"}), "capacitance_f"),
            ("ohms_law.capacitors_parallel", json!({"values":"10e-9, 10e-9"}), "capacitance_f"),
            ("ohms_law.inductors_series", json!({"values":"10e-6, 10e-6"}), "inductance_h"),
            ("ohms_law.inductors_parallel", json!({"values":"10e-6, 10e-6"}), "inductance_h"),
            ("ppm.hz_to_ppm", json!({"center_hz":32000,"max_hz":32001}), "ppm"),
            ("ppm.ppm_to_hz", json!({"center_hz":50e6,"ppm":25}), "variation_hz"),
            ("ppm.xtal_load", json!({"c1_f":14e-12,"c2_f":14e-12,"c_stray_f":3e-12}), "c_load_calc_f"),
            ("padstack.thru_hole", json!({"hole_diameter_mils":32,"annular_ring_mils":12,"isolation_width_mils":12}), "pad_external_mils"),
            ("padstack.corner_to_corner", json!({"a_mils":100,"b_mils":100}), "diagonal_mils"),
            ("spacing", json!({"voltage":100,"device_type":"b1"}), "spacing_mils"),
            ("wire_gauge", json!({"awg":"22"}), "diameter_mils"),
            ("pdn", json!({"v_supply":5,"i_max":2,"i_step_pct":50,"v_ripple_pct":5,"area_sq_in":5,"er":4.6,"d_mils":2,"freq_mhz":1}), "z_target_ohms"),
            ("thermal", json!({"r_theta_ja":50,"power_w":2,"t_ambient_c":25}), "t_junction_c"),
            ("crosstalk", json!({"rise_time_ns":1,"voltage":5,"coupled_length_mils":250,"spacing_mils":10,"height_mils":30,"er":4.6,"trace_width_mils":10}), "crosstalk_db"),
        ]
    }

    /// Every dispatch arm resolves, runs, and returns the field the UI reads.
    #[test]
    fn every_calculator_dispatches_and_returns_its_primary_output() {
        for (id, params, key) in cases() {
            let out = calculate(id.to_string(), params.clone())
                .unwrap_or_else(|e| panic!("`{id}` failed: {e}"));
            let v = out
                .get(key)
                .unwrap_or_else(|| panic!("`{id}` result has no `{key}`: {out}"));
            assert!(
                v.is_number(),
                "`{id}` field `{key}` should be numeric, got {v}"
            );
        }
    }

    /// The schema in the frontend lists exactly these ids; guard against drift.
    #[test]
    fn covers_every_dispatch_arm() {
        assert_eq!(cases().len(), 37, "expected 37 calculators");
    }

    #[test]
    fn unknown_id_is_an_error() {
        assert!(calculate("nope".into(), json!({})).is_err());
    }

    #[test]
    fn missing_required_parameter_is_reported_by_name() {
        let err = calculate("impedance.microstrip".into(), json!({ "height": 5 })).unwrap_err();
        assert!(err.contains("width"), "error should name the field: {err}");
    }

    /// Library errors surface as readable strings rather than being swallowed.
    #[test]
    fn library_errors_propagate() {
        let err = calculate(
            "impedance.stripline".into(),
            json!({"width":100,"height":20,"thickness":1.4,"er":4.6}),
        )
        .unwrap_err();
        assert!(err.contains("unphysical"), "unexpected error: {err}");
    }

    /// Numeric strings are accepted, since the inputs are text fields.
    #[test]
    fn numeric_strings_are_accepted() {
        let out = calculate(
            "thermal".into(),
            json!({"r_theta_ja":"50","power_w":"2","t_ambient_c":"25"}),
        )
        .unwrap();
        assert_eq!(out["t_junction_c"], json!(125.0));
    }

    #[test]
    fn reference_lists_are_populated() {
        assert_eq!(list_materials().len(), 45);
        assert_eq!(list_awg().len(), 44);
        assert!(list_awg().contains(&"4/0"));
    }
}
