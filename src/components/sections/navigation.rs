use crate::components::*;
use fishnet::component::prelude::*;

pub fn navigation(entries: Vec<(&str, &str)>) -> impl BuildableComponent {
    let entries = entries
        .into_iter()
        .map(|(name, target_id)| (name.to_string(), target_id.to_string()))
        .collect::<Vec<_>>();

    Component::new("navigation")
        .add_script(ScriptType::External("js/tabs.js".into()))
        .render(move |_| {
            section(
                "navigation",
                html! {
                    div class="tabs vertical" {
                        @for (name, target_id) in &entries {
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
        })
}
