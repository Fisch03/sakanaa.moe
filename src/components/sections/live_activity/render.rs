use maud::{html, Markup};

use super::types::*;
use crate::api::discord::*;
use crate::components::*;

fn bouncing_note() -> Markup {
    html! {
        img style="display: display: inline-block; width: 20px; margin-right:4px;"
            class="bounce colorfilter"
            src="/assets/notespin.png"
        {}
    }
}

fn activity_img(url: &str) -> Markup {
    html! {
        img style="width: 10rem; height:10rem; border-radius: 1rem; margin-bottom: 10px; margin-left: 5px;"
            class="shadow-box colorfilter"
            src=(url)
        {}
    }
}

fn avatar_img(user: &DiscordUser) -> Markup {
    let avatar_src = if let Some(avatar) = &user.avatar {
        format!(
            "https://cdn.discordapp.com/avatars/{}/{}.png",
            user.id, avatar
        )
    } else {
        "https://cdn.discordapp.com/embed/avatars/0.png".to_string()
    };

    avatar_image(&avatar_src)
}

impl LiveActivity {
    fn render_music(music: &MusicActivity) -> Markup {
        html! {
            @if let Some(url) = &music.album_art {
                (activity_img(url))
            }
            div {
                h3 { (bouncing_note()) "im listening to" }
                h2 style="max-width: 20rem; margin-bottom: 5px"
                   { (music.song_title.as_str()) }
                @if let Some(artist) = &music.artist {
                    h4 style="max-width: 20rem; margin-top: 0; margin-bottom: 0"
                        { "by " (artist.as_str()) }
                }
                @if let Some(album) = &music.album {
                    h4 style="max-width: 20rem; margin-top: 0; margin-bottom: 0"
                        { "on " (album.as_str()) }
                }
            }
        }
    }

    fn render_activity(activity: &DiscordActivity) -> Markup {
        let has_name = activity.name.is_some();
        let title = if let Some(title) = &activity.custom_title {
            title.as_str()
        } else {
            "playing"
        };

        let title = if has_name {
            html!(h3 style="margin-bottom: 10px" { "im " (title) })
        } else {
            html!(h2 style="margin-bottom: 10px" { "im playing" })
        };

        html! {
            @if let Some(url) = &activity.assets.as_ref().and_then(|a| a.large_image.as_ref()) {
                @if let Some(application_id) = &activity.application_id {
                    (activity_img(&discord_img_url(url, application_id)))
                }
            }
            div {
                (title)
                @if let Some(name) = &activity.name {
                    h2 style="max-width: 20rem; margin-bottom: 5px; margin-top: 0"
                       { (name.as_str()) }
                }
                @if let Some(details) = &activity.details {
                    h4 style="max-width: 20rem; margin-top: 0; margin-bottom: 0"
                        { (details.as_str()) }
                }
                @if let Some(state) = &activity.state {
                    h4 style="max-width: 20rem; margin-top: 0;"
                        { (state.as_str()) }
                }
            }
        }
    }

    fn render_no_activity() -> Markup {
        html! {
            div style="width: 100%; height: 90%; display:flex; justify-content: center; align-items:center;"
                { h3 { "looks like i'm not doing anything right now..." } }
        }
    }

    fn render_relevant_activity(&self, custom_filters: &[CustomActivityFilter]) -> Markup {
        if let Some(music) = &self.music_activity {
            Self::render_music(&music)
        } else if let Some(activity) = &self.discord_activities.first() {
            if let Some(filtered_activity) = custom_filters.iter().find_map(|f| f.apply(activity)) {
                Self::render_activity(&filtered_activity)
            } else {
                Self::render_activity(&activity)
            }
        } else {
            Self::render_no_activity()
        }
    }

    pub fn render(&self, custom_filter: &[CustomActivityFilter]) -> Markup {
        let header = if let Some(user) = &self.discord_user {
            html!(
                div class="inv-border avatar-border" { (avatar_img(user)) }
                (section_header(user.username.as_str()))
            )
        } else {
            section_header("what i'm doing right now!")
        };

        section_inner(header, self.render_relevant_activity(custom_filter))
    }
}
