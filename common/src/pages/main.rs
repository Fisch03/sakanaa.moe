use crate::components;
use maud::{DOCTYPE, Markup, html};

pub async fn main() -> Markup {
    html! {
        (DOCTYPE)
        html {
            head {
                title { "sakanaa!" }
                (components::head::shared())
            }
            body {
                #main {
                    (components::app::selector())
                    (components::app::container())
                    (components::widget::container())
                }
                // (components::app::float_target())
            }
        }
    }
}
