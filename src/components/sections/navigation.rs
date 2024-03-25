use maud::html;

use crate::components::*;
use crate::dyn_component::JSComponent;

pub fn navigation(entries: Vec<(&str, &str)>) -> JSComponent {
    let render = section(
        "navigation",
        html! {
            div class="tabs vertical" {
                @for (name, target_id) in entries {
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
    );

    JSComponent::new(render, vec!["js/tabs.js".into()])
}
