use super::ComponentState;
use maud::Markup;

pub type ContentRenderer<ST> = Box<dyn Fn(ComponentState<ST>) -> Markup + Send + Sync>;

pub type ComponentStyle = String;

pub struct StatefulContentRenderer<ST>
where
    ST: Clone + Send + Sync,
{
    renderer: ContentRenderer<ST>,
    state: ComponentState<ST>,
}
impl<ST> StatefulContentRenderer<ST>
where
    ST: Clone + Send + Sync,
{
    pub fn new(renderer: ContentRenderer<ST>, state: ComponentState<ST>) -> Box<Self> {
        Box::new(Self { renderer, state })
    }
}

pub trait StatefulRenderer: Send + Sync {
    fn render(&self) -> Markup;
}
impl<ST> StatefulRenderer for StatefulContentRenderer<ST>
where
    ST: Clone + Send + Sync,
{
    fn render(&self) -> Markup {
        (self.renderer)(self.state.clone())
    }
}

pub enum ContentType {
    Dynamic(Box<dyn StatefulRenderer>),
    Static(Markup),
}
impl ContentType {
    pub fn render(&self) -> Markup {
        match self {
            ContentType::Dynamic(renderer) => renderer.render(),
            ContentType::Static(content) => content.clone(),
        }
    }
}
impl std::fmt::Debug for ContentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContentType::Dynamic(_) => write!(f, "Dynamic"),
            ContentType::Static(_) => write!(f, "Static"),
        }
    }
}
