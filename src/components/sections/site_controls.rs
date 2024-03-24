use maud::{html, Markup};

use crate::components::*;

pub fn site_controls(zerox20render: Markup) -> Markup {
    section(
        "controls",
        html! {
            button id="ColorBtn" { (filtered_image("assets/palette.png")) }
            (zerox20render)
        },
        &SectionConfig {
            id: Some("SiteControls"),
            is_vertical: true,
            at_end: true,
            hidden_on_mobile: true,
            ..Default::default()
        },
    )
}
