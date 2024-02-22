use crate::components::*;
use crate::config::CONFIG;
use chrono::prelude::*;
use date_component::date_component;
use maud::{html, Markup};

pub fn about_me() -> Markup {
    //yes, this is completely overkill :P
    let now = Utc::now();
    let config_date = CONFIG
        .get::<String>("page.birthday")
        .expect("page.birthday not set in config");

    let birthdate = config_date.split('-').collect::<Vec<&str>>();
    if birthdate.len() != 3 {
        panic!("Invalid date format in page.birthday config");
    }
    let year = birthdate[0].parse::<i32>().unwrap();
    let month = birthdate[1].parse::<u32>().unwrap();
    let day = birthdate[2].parse::<u32>().unwrap();
    let birthdate = DateTime::from_naive_utc_and_offset(
        NaiveDateTime::new(
            NaiveDate::from_ymd_opt(year, month, day)
                .expect("Invalid date in page.birthday config"),
            NaiveTime::default(),
        ),
        chrono::offset::Utc,
    );

    let interval = date_component::calculate(&birthdate, &now);
    let interval_years = interval.year;

    section(
        "about me",
        html! {
            p { "Hi! if you read this you somehow found a way to this random place on the interwebs, glad to have you here :)"}
            p { "im sakanaa, (formerly Fisch03), a " (interval_years) " year old CS student from germany."}
            p { "stuff i like:"
                ul {
                    li { "programming" }
                    li { "photography" }
                    li { "anime" }
                    li { "vtuber" }
                    li { "rythm games"}
                    li { "music" }
                }
            }
            p { "feel free to scroll down and check out more stuff about me! " span class="hideonmobile" { "(or just use the navigation buttons on the left)" } }
        },
        &SectionConfig {
            id: Some("AboutMe"),
            ..Default::default()
        },
    )
}