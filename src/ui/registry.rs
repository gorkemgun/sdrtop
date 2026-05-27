use std::collections::HashMap;
use ratatui::{layout::Rect, Frame};
use crate::state::SdrMetrics;
use super::panel::Panel;

pub struct PanelRegistry {
    panels: HashMap<&'static str, Box<dyn Panel>>,
}

impl PanelRegistry {
    pub fn new() -> Self {
        Self { panels: HashMap::new() }
    }

    pub fn register(&mut self, panel: impl Panel + 'static) {
        self.panels.insert(panel.name(), Box::new(panel));
    }

    pub fn get(&self, name: &str) -> Option<&dyn Panel> {
        self.panels.get(name).map(|p| p.as_ref())
    }

    pub fn render_panel(&self, name: &str, f: &mut Frame, area: Rect, state: &SdrMetrics) {
        if let Some(panel) = self.get(name) {
            panel.render(f, area, state);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::panel::Panel;

    struct NamedPanel(&'static str);

    impl Panel for NamedPanel {
        fn name(&self) -> &'static str { self.0 }
        fn min_size(&self) -> (u16, u16) { (0, 0) }
        fn render(&self, _f: &mut Frame, _area: Rect, _state: &SdrMetrics) {}
    }

    #[test]
    fn register_and_retrieve() {
        let mut reg = PanelRegistry::new();
        reg.register(NamedPanel("alpha"));
        reg.register(NamedPanel("beta"));
        assert!(reg.get("alpha").is_some());
        assert!(reg.get("beta").is_some());
        assert!(reg.get("gamma").is_none());
    }
}
