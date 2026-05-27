use ratatui::{layout::Rect, Frame};
use crate::state::SdrMetrics;

pub trait Panel: Send + Sync {
    fn name(&self) -> &'static str;
    #[allow(dead_code)]
    fn min_size(&self) -> (u16, u16);
    fn render(&self, f: &mut Frame, area: Rect, state: &SdrMetrics);
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DummyPanel;

    impl Panel for DummyPanel {
        fn name(&self) -> &'static str { "dummy" }
        fn min_size(&self) -> (u16, u16) { (10, 3) }
        fn render(&self, _f: &mut Frame, _area: Rect, _state: &SdrMetrics) {}
    }

    #[test]
    fn panel_name_and_min_size() {
        let p = DummyPanel;
        assert_eq!(p.name(), "dummy");
        assert_eq!(p.min_size(), (10, 3));
    }
}
