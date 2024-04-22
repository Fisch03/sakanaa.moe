use crate::components::*;
use fishnet::component::prelude::*;

pub fn navigation(entries: Vec<(&str, &str)>) -> impl BuildableComponent {
    let entries = entries
        .into_iter()
        .map(|(name, target_id)| (name.to_string(), target_id.to_string()))
        .collect::<Vec<_>>();

    component!(Navigation)
        .with_state(Arc::new(entries))
        .add_script(ScriptType::External("js/tabs.js".into()))
        .render(|entries| {
            async move {
                section(
                    "navigation",
                    html! {
                        div class="tabs vertical" {
                            @for (name, target_id) in entries.as_ref() {
                                button data-target=(target_id) { (name) }
                            }
                        }
                    },
                    &SectionConfig {
                        id: Some("Navigation"),
                        is_vertical: true,
                        hidden_on_mobile: true,
                        ..Default::default()
                    },
                )
                .await
            }
            .boxed()
        })
}
