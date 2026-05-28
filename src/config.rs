use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Position {
    Top,
    Bottom,
    Left,
    Right,
    Body,
}

#[derive(Deserialize, Clone, Debug)]
pub struct PanelSpec {
    pub name: String,
    pub position: Position,
    /// Height in terminal rows — used for Top and Bottom panels.
    #[serde(default)]
    pub height: Option<u16>,
    /// Width as a percentage of the body zone — used for Left and Right panels.
    /// All panels in the same column carry the same value; the LayoutEngine
    /// reads only the first panel's value to determine column width.
    #[serde(default)]
    pub width_pct: Option<u16>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct PresetConfig {
    pub panels: Vec<PanelSpec>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct LayoutConfig {
    pub active_preset: String,
    pub presets: HashMap<String, PresetConfig>,
}

impl LayoutConfig {
    pub fn default_config() -> Self {
        use Position::*;
        let minimal = PresetConfig {
            panels: vec![
                PanelSpec { name: "header".into(),    position: Top,    height: Some(3), width_pct: None },
                PanelSpec { name: "telemetry".into(), position: Body,   height: None,    width_pct: None },
                PanelSpec { name: "gains".into(),     position: Right,  height: None,    width_pct: Some(50) },
                PanelSpec { name: "log".into(),       position: Bottom, height: Some(9), width_pct: None },
                PanelSpec { name: "footer".into(),    position: Bottom, height: Some(3), width_pct: None },
            ],
        };
        let monitoring = PresetConfig {
            panels: vec![
                PanelSpec { name: "header".into(),           position: Top,    height: Some(3), width_pct: None     },
                PanelSpec { name: "hardware_health".into(),  position: Left,   height: None,    width_pct: Some(50) },
                PanelSpec { name: "iq_diagnostics".into(),   position: Left,   height: None,    width_pct: Some(50) },
                PanelSpec { name: "telemetry".into(),        position: Right,  height: None,    width_pct: Some(50) },
                PanelSpec { name: "system_resources".into(), position: Right,  height: None,    width_pct: Some(50) },
                PanelSpec { name: "log".into(),              position: Bottom, height: Some(7), width_pct: None     },
                PanelSpec { name: "footer".into(),           position: Bottom, height: Some(3), width_pct: None     },
            ],
        };
        let spectrum = PresetConfig {
            panels: vec![
                PanelSpec { name: "header".into(),   position: Top,    height: Some(3), width_pct: None },
                PanelSpec { name: "spectrum".into(),  position: Body,   height: None,    width_pct: None },
                PanelSpec { name: "log".into(),       position: Bottom, height: Some(5), width_pct: None },
                PanelSpec { name: "footer".into(),    position: Bottom, height: Some(3), width_pct: None },
            ],
        };
        let mut presets = HashMap::new();
        presets.insert("minimal".into(), minimal);
        presets.insert("monitoring".into(), monitoring);
        presets.insert("spectrum".into(), spectrum);
        Self { active_preset: "minimal".into(), presets }
    }

    pub fn active_panels(&self) -> &[PanelSpec] {
        self.presets
            .get(&self.active_preset)
            .map(|p| p.panels.as_slice())
            .unwrap_or(&[])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_minimal_preset() {
        let cfg = LayoutConfig::default_config();
        assert_eq!(cfg.active_preset, "minimal");
        assert!(!cfg.active_panels().is_empty());
    }

    #[test]
    fn active_panels_returns_correct_names() {
        let cfg = LayoutConfig::default_config();
        let names: Vec<&str> = cfg.active_panels().iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"header"));
        assert!(names.contains(&"footer"));
        assert!(names.contains(&"telemetry"));
    }

    #[test]
    fn deserialize_from_toml() {
        let raw = r#"
            active_preset = "minimal"
            [presets.minimal]
            panels = [
              { name = "header", position = "top", height = 3 },
              { name = "footer", position = "bottom", height = 3 },
            ]
        "#;
        let cfg: LayoutConfig = toml::from_str(raw).unwrap();
        assert_eq!(cfg.active_panels().len(), 2);
    }
}
