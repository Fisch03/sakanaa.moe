use maud::{html, Markup};

pub fn big_waifu(src: &str) -> Markup {
    html! {
        div id="BigWaifu" {
            img src=(src) class="shadow paletteimg" {}
        }
    }
}
