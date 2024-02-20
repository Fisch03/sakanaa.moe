use crate::components::*;
use maud::{html, Markup};

pub fn music() -> Markup {
    section(
        "music",
        html! {
            p { "i would generally say that i enjoy all sorts of music. the genres i enjoy the most though are" }
            ul {
                li { "vocaloid" }
                li { "jpop" }
                li { "eurobeat" }
                li { "hardcore" }
                li { "math rock" }
                li { "various subgenres of breakcore, especially"
                    ul {
                        li { "lolicore" }
                        li { "speedcore" }
                        li { "mashcore/dancecore" }
                    }
                }
                li { "hyperflip (or dariacore, whatever you wanna call it)" }
                li { "jazz-fusion" }
                li { "bossa nova" }
            }
        },
        &SectionConfig {
            id: Some("Music"),
            ..Default::default()
        },
    )
}
