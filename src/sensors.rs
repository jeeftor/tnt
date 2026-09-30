//! Parsing and display grouping for `lm-sensors` JSON output.

use serde_json::Value;

/// Display section assigned to a fan or temperature reading.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Group {
    Fans,
    Cpu,
    System,
    Drives,
}

impl Group {
    /// Return the label shown for this group in the sensor list.
    pub fn name(self) -> &'static str {
        match self {
            Self::Fans => "Fans",
            Self::Cpu => "CPU",
            Self::System => "System",
            Self::Drives => "Drives",
        }
    }
}

/// One numeric input field from a sensor feature.
#[derive(Clone, Debug)]
pub struct Reading {
    /// Stable key used to retain history and selection across samples.
    pub id: String,
    /// Feature label shown in the UI, or the drive chip suffix for drives.
    pub name: String,
    /// Chip key from the `sensors -j` output.
    pub chip: String,
    pub group: Group,
    pub value: f64,
    pub unit: &'static str,
    /// Positive critical value, or maximum when no critical value is present.
    pub limit: Option<f64>,
}

/// Parse finite temperature and fan input values from `sensors -j` output.
///
/// Other fields and nonnumeric values are skipped. Readings are sorted by
/// group, display name, then chip so the list stays stable across samples.
/// Invalid JSON returns an error; valid JSON without usable inputs returns an
/// empty list.
pub fn parse_sensors(input: &[u8]) -> Result<Vec<Reading>, serde_json::Error> {
    let root: Value = serde_json::from_slice(input)?;
    let mut readings = Vec::new();
    if let Some(chips) = root.as_object() {
        for (chip, features) in chips {
            let Some(features) = features.as_object() else {
                continue;
            };
            for (name, fields) in features {
                let Some(fields) = fields.as_object() else {
                    continue;
                };
                for (field, value) in fields {
                    let Some(prefix) = field.strip_suffix("_input") else {
                        continue;
                    };
                    let Some(value) = value.as_f64().filter(|v| v.is_finite()) else {
                        continue;
                    };
                    let (group, unit) = if prefix.starts_with("fan") {
                        (Group::Fans, "RPM")
                    } else if prefix.starts_with("temp") {
                        let group = if chip.starts_with("drivetemp") {
                            Group::Drives
                        } else if chip.starts_with("coretemp") || name == "CPU" {
                            Group::Cpu
                        } else {
                            Group::System
                        };
                        (group, "°C")
                    } else {
                        continue;
                    };
                    let limit = fields
                        .get(&format!("{prefix}_crit"))
                        .or_else(|| fields.get(&format!("{prefix}_max")))
                        .and_then(Value::as_f64)
                        .filter(|v| v.is_finite() && *v > 0.0);
                    let display_name = if group == Group::Drives {
                        chip.strip_prefix("drivetemp-").unwrap_or(chip).to_owned()
                    } else {
                        name.clone()
                    };
                    readings.push(Reading {
                        id: format!("{chip}/{name}/{field}"),
                        name: display_name,
                        chip: chip.clone(),
                        group,
                        value,
                        unit,
                        limit,
                    });
                }
            }
        }
    }
    readings.sort_by(|a, b| {
        (a.group, a.name.as_str(), a.chip.as_str()).cmp(&(
            b.group,
            b.name.as_str(),
            b.chip.as_str(),
        ))
    });
    Ok(readings)
}

#[cfg(test)]
mod tests {
    use super::{Group, parse_sensors};

    #[test]
    fn parses_truenas_fans_and_temperatures() {
        let sample = br#"{
          "dell_smm-isa-00de": {
            "Adapter": "ISA adapter",
            "Processor Fan": {"fan1_input": 933.0, "fan1_min": 0.0, "fan1_max": 3700.0},
            "Motherboard Fan": {"fan2_input": 1166.0, "fan2_max": 2400.0},
            "Ambient": {"temp1_input": 27.0},
            "pwm1": {"pwm1_enable": 2.0}
          },
          "coretemp-isa-0000": {
            "Package id 0": {"temp1_input": 29.0, "temp1_crit": 100.0}
          },
          "drivetemp-scsi-0-40": {
            "temp1": {"temp1_input": 46.0, "temp1_crit": 70.0}
          }
        }"#;
        let readings = parse_sensors(sample).unwrap();
        assert_eq!(readings.len(), 5);
        assert_eq!(readings[0].name, "Motherboard Fan");
        assert_eq!(readings[1].value, 933.0);
        assert_eq!(readings[2].group, Group::Cpu);
        assert_eq!(readings[3].name, "Ambient");
        assert_eq!(readings[4].group, Group::Drives);
        assert_eq!(readings[4].limit, Some(70.0));
    }
}
