use futures::future::FutureExt;

use crate::components::sections::*;
use crate::components::*;
use fishnet::{c, html, Page};

use tracing::{info, instrument};

#[instrument]
pub fn root_page() -> Page {
    info!("preparing page content");

    Page::new("root")
        .with_head(html! {
            meta charset="utf-8";
            meta name="viewport" content="width=device-width, initial-scale=1";
            title { "sakanaa :)" }
            link rel="stylesheet" href="css/style.css";
        })
        .with_body(|| {
            async {
                html! {
                    body style="background-image: url('assets/dither/bgdither.png')" class="ditherbg onex" {
                        (c!(colorfilter()))
                        (c!(big_waifu("assets/Yuuko.png")))
                        (content(
                            html! {
                                (c!(LiveActivityComponent::new()))
                                (c!(navigation(vec![
                                    ("about me", "AboutMe"),
                                    ("music", "Music"),
                                    ("microblogging", "Microblogging"),
                                    ("hardware", "Hardware"),
                                    ("uptime", "Uptime"),
                                ])))
                                (site_controls(c!(Zerox20ButtonComponentState::new())).await)
                            },
                            html! {
                                (about_me().await)
                                //(c(MusicComponent::new()))
                                //(c(MicrobloggingComponent::new()))
                                (hardware().await)
                            }
                        ).await)
                    }
                }
    }.boxed()
        })
}
