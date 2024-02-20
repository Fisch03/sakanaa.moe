use maud::{html, Markup};

use crate::components::sections::*;
use crate::components::*;
use crate::dyn_component::DynamicComponent;
use crate::website::Website;

macro_rules! use_dyn {
    ($dyn:ident) => {
        match $dyn {
            Ok($dyn) => $dyn.lock().await.render(),
            Err(e) => {
                eprintln!("Error rendering dynamic component: {}", e);
                html! {}
            }
        }
    };
}

pub async fn root_page(website: &mut Website) -> Markup {
    let live_activity = website.add_dynamic_component("live_activity", LiveActivityComponent::new);

    html! {
        (big_waifu("assets/Yuuko.png"))
        div id="Content" {
            div class="column" {
                (use_dyn!(live_activity))
                (navigation(vec![
                    ("about me", "AboutMe"),
                    ("music", "Music"),
                    ("microblogging", "Microblogging"),
                    ("hardware", "Hardware"),
                    ("uptime", "Uptime"),
                ]))
            }
            div class="column" {
                (about_me())
                div class="columnsection" {
                    (music())
                }
                (hardware())
            }
        }
    }
}
