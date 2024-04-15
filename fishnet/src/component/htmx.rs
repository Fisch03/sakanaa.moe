//! Prebuild components for adding HTMX to your page.

use super::prelude::*;
use axum::routing::get;
use futures::{executor::block_on, future::BoxFuture};

pub struct HTMXComponent;
pub struct HTMXComponentState<ST>
where
    ST: Send + Sync + Clone + 'static,
{
    renderer: Box<dyn Fn(ST) -> BoxFuture<'static, Markup> + Send + Sync>,
    used_state: ST,
}
impl<ST> HTMXComponentState<ST>
where
    ST: Send + Sync + Clone + 'static,
{
    pub fn render(&self) -> BoxFuture<'static, Markup> {
        (self.renderer)(self.used_state.clone())
    }
}

impl HTMXComponent {
    async fn get<ST>(state: Extension<ComponentState<Arc<HTMXComponentState<ST>>>>) -> Markup
    where
        ST: Send + Sync + Clone + 'static,
    {
        (state.renderer)(state.used_state.clone()).await
    }

    pub fn new<F>(trigger: &str, renderer: F) -> impl BuildableComponent
    where
        F: Fn(()) -> BoxFuture<'static, Markup> + Send + Sync + 'static,
    {
        let render = block_on(renderer(()));
        let trigger = trigger.to_string();

        let state = Arc::new(HTMXComponentState::<()> {
            renderer: Box::new(renderer),
            used_state: (),
        });

        Component::new("htmx_component")
            .with_state(state.clone())
            .route("/", get(Self::get::<()>))
            .render(move |state| {
                let endpoint = state.endpoint();
                html! {
                    div hx-get=(endpoint) hx-trigger=(trigger) {
                        (render)
                    }
                }
            })
    }

    pub fn new_with_state<F, ST>(trigger: &str, state: ST, renderer: F) -> impl BuildableComponent
    where
        ST: Send + Sync + Clone + 'static,
        F: Fn(ST) -> BoxFuture<'static, Markup> + Send + Sync + 'static,
    {
        let render = block_on(renderer(state.clone()));
        dbg!(&render);
        let trigger = trigger.to_string();

        let state = Arc::new(HTMXComponentState::<ST> {
            renderer: Box::new(renderer),
            used_state: state,
        });
        Component::new("htmx_component")
            .with_state(state.clone())
            .route("/", get(Self::get::<ST>))
            .render(move |state| {
                let endpoint = state.endpoint();
                html! {
                    div hx-get=(endpoint) hx-trigger=(trigger) {
                        (render)
                    }
                }
            })
    }
}

#[macro_export]
macro_rules! htmx {
    ($trigger:literal, $endpoint:expr) => {
        $crate::c!(
            $crate::component::Component::new("htmx_component").render_dynamic(move |_| {
                $crate::component::prelude::html! {
                    div hx-get=($endpoint) hx-trigger=($trigger) {}
                }
            })
        )
    };
    /* ($trigger:literal, $render:expr) => {
        $crate::c!($crate::component::htmx::HTMXComponent::new(
            $trigger,
            move |_| async { $render }.boxed()
        ))
    }; */
    ($trigger:literal, $state:expr, $handler:expr) => {{
        $crate::c!($crate::component::htmx::HTMXComponent::new_with_state(
            $trigger, $state, $handler
        ))
    }};
}
