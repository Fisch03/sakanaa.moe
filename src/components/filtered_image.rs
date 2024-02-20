use maud::{html, Markup};

pub fn filtered_image(src: &str) -> Markup {
    html! {
        img src=(src) class="colorfilter" {}
    }
}

pub fn avatar_image(src: &str) -> Markup {
    html! {
        img src=(src) class="avatar colorfilter" {}
    }
}
