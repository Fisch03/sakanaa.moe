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

fn column_spacer() -> Markup {
    html! {
        div class="column-spacer" {}
    }
}

pub async fn root_page(website: &mut Website) -> Markup {
    let live_activity = website.add_dynamic_component("live_activity", LiveActivityComponent::new);
    //let microblogging = website.add_dynamic_component("microblogging", MicrobloggingComponent::new);
    let music = website.add_dynamic_component("music", MusicComponent::new);

    let zerox20 = website.add_dynamic_component("0x20", Zerox20ButtonComponent::new);

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
                (column_spacer())
                (site_controls(use_dyn!(zerox20)))
            }
            div class="column" {
                (about_me())
                (use_dyn!(music))
                //(use_dyn!(microblogging))
                (hardware())
            }
        }
    }
}
