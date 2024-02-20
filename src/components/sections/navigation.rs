use maud::{html, Markup};

use crate::components::*;

pub fn navigation(entries: Vec<(&str, &str)>) -> Markup {
    section(
        "navigation",
        html! {
            div class="sectioncontent tabs vertical" {
                @for (name, target_id) in entries {
                    button data-target=(target_id) { (name) }
                }
            }
            script src="js/tabs.js" {}
        },
        &SectionConfig {
            id: Some("Navigation"),
            is_vertical: true,
            hidden_on_mobile: true,
            ..Default::default()
        },
    )
}
