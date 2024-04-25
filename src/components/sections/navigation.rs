use crate::components::*;
use fishnet::component::prelude::*;

#[component]
pub fn navigation(entries: Vec<(&str, &str)>) {
    let entries = state_init!(Arc::new(
        entries
            .into_iter()
            .map(|(name, target_id)| (name.to_string(), target_id.to_string()))
            .collect::<Vec<_>>()
    ));

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
