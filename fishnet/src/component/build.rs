use axum::{Extension, Router};
use futures::future::{BoxFuture, FutureExt};
use maud::{html, Markup};
use std::any::TypeId;
use std::sync::Arc;
use tracing::{debug, instrument, trace};

use super::{
    render::{ComponentStyle, ContentType, StatefulContentRenderer},
    Component, ComponentRoute, ComponentState, HasRenderer,
};
use crate::js::ScriptType;
use crate::page::render_context;

#[derive(Debug, Clone)]
pub struct BuiltComponent {
    type_id: TypeId,

    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    id: String,

    content: Arc<ContentType>,
}

pub struct ComponentBuildResult {
    pub built_component: BuiltComponent,

    pub runner: Option<BoxFuture<'static, ()>>,
    pub router: Option<(ComponentRoute, Router)>,
}

pub struct ComponentGlobals {
    pub scripts: Vec<ScriptType>,
    pub style: Option<ComponentStyle>,
}

impl BuiltComponent {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn type_id(&self) -> TypeId {
        self.type_id
    }

    pub fn render(&self) -> Markup {
        html! {
            div class=(self.name) { (self.content.render()) }
        }
    }

    pub fn is_dynamic(&self) -> bool {
        match self.content.as_ref() {
            ContentType::Static(_) => false,
            _ => true,
        }
    }
}

pub trait BuildableComponent {
    fn name(&self) -> &str;
    fn id(&self) -> &str;

    fn build(self: Self, base_route: &str) -> ComponentBuildResult;
}

impl<ST, S> BuildableComponent for Component<HasRenderer, S, ST>
where
    ST: Clone + Send + Sync + 'static,
{
    fn name(&self) -> &str {
        &self.name
    }
    fn id(&self) -> &str {
        &self.id
    }

    #[instrument(name = "build_component", skip_all, fields(name = %self.name))]
    fn build(self, base_route: &str) -> ComponentBuildResult {
        trace!("building state");
        let api_route = ComponentRoute::new(base_route, &self.name, &self.id);
        let state = ComponentState {
            api_route: api_route.clone(),
            state: self.state,
        };

        let router = self.router.map(|r| r.layer(Extension(state.clone())));

        let renderer = self.renderer.unwrap();

        let runner = self.runner.map(|runner| {
            let runner = (runner)(state.clone());
            runner.boxed()
        });

        let content;

        if !self.is_dynamic {
            trace!("pre-rendering static component");
            render_context::enter_temporary_render();
            let render = renderer(state.clone());
            if !render_context::exit_temporary_render() {
                debug!("detected dynamic child, making self dynamic");
                content = ContentType::Dynamic(StatefulContentRenderer::new(renderer, state));
            } else {
                content = ContentType::Static(render);
            }
        } else {
            content = ContentType::Dynamic(StatefulContentRenderer::new(renderer, state));
        }

        trace!("rendering component style");
        let style = self.style.map(|style| style.render(&self.name));

        render_context::global_store().add(self.type_id, || ComponentGlobals {
            scripts: self.scripts,
            style,
        });

        debug!("built component");
        ComponentBuildResult {
            built_component: BuiltComponent {
                type_id: self.type_id,
                name: self.name,
                id: self.id,
                content: Arc::new(content),
            },
            runner,
            router: router.map(|r| (api_route, r)),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::component::prelude::*;
    use crate::component::render::ContentType;

    #[test]
    fn test_build_minimal() {
        let result = component!(TestComponent)
            .render(|_| html! { "test" })
            .build("/");

        assert_eq!(result.built_component.name(), "TestComponent");
        assert!(matches!(
            result.built_component.content.as_ref(),
            ContentType::Static(_)
        ));

        assert!(result.runner.is_none());
        assert!(result.router.is_none());
    }

    #[test]
    fn test_build_minimal_dynamic() {
        let result = component!(TestComponent)
            .render_dynamic(|_| html! { "test" })
            .build("/");

        assert_eq!(result.built_component.name(), "TestComponent");
        assert!(matches!(
            result.built_component.content.as_ref(),
            ContentType::Dynamic(_)
        ));

        assert!(result.runner.is_none());
        assert!(result.router.is_none());
    }

    #[test]
    fn test_build() {
        use axum::routing::get;

        let result = component!(TestComponent)
            .add_script(ScriptType::External("test.js".to_string()))
            .route("/route", get(|| async { "test" }))
            .render(|_| html! { "test" })
            .build("/");

        assert_eq!(result.built_component.name(), "TestComponent");
        assert!(matches!(
            result.built_component.content.as_ref(),
            ContentType::Static(_)
        ));
        assert!(result.router.is_some());

        /* it would be nice to verify the global store here
        assert_eq!(result.scripts.len(), 1);
        assert_eq!(
            result.scripts[0],
            ScriptType::External("test.js".to_string())
        );
        */
    }

    /*
    #[test]
    fn test_nested_dynamic() {
        todo!();
    }
    */
}
