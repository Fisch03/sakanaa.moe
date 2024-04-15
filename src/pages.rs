use maud::{html, Markup};

use crate::components::sections::*;
use crate::components::*;
use fishnet::{c, page::Page};

use tracing::{info, instrument};

fn column_spacer() -> Markup {
    html! {
        div class="column-spacer" {}
    }
}

#[instrument]
pub fn root_page() -> Page {
    info!("preparing page content");

    Page::new("root").with_content(|| {
        html! {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { "sakanaa :)" }
                link rel="stylesheet" href="css/style.css";
            }

            body style="background-image: url('assets/dither/bgdither.png')" class="ditherbg onex" {
                (c!(colorfilter()))
                (big_waifu("assets/Yuuko.png"))
                div id="Content" {
                    div class="column" {
                        (c!(LiveActivityComponent::new()))
                        (c!(navigation(vec![
                            ("about me", "AboutMe"),
                            ("music", "Music"),
                            ("microblogging", "Microblogging"),
                            ("hardware", "Hardware"),
                            ("uptime", "Uptime"),
                        ])))
                        (column_spacer())
                        (site_controls(c!(Zerox20ButtonComponentState::new())))
                    }
                    div class="column" {
                        (about_me())
                        //(c(MusicComponent::new()))
                        //(c(MicrobloggingComponent::new()))
                        (hardware())
                    }
                }
            }
        }
    })
}
