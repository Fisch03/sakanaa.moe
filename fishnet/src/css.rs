pub struct StyleFragment(&'static str);

impl StyleFragment {
    pub fn new(input: &'static str) -> Self {
        Self(input)
    }

    pub fn render(&self, toplevel_class: &str) -> String {
        self.0.to_string().replace("&", toplevel_class)
    }
}
