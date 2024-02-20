use super::types::*;
use maud::{html, Markup, Render};

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

impl LiveActivity {
    fn avatar_img(&self) -> Markup {
        let avatar_src = if let Some(avatar) = &self.discord_user.avatar {
            format!(
                "https://cdn.discordapp.com/avatars/{}/{}.png",
                self.discord_user.id, avatar
            )
        } else {
            "https://cdn.discordapp.com/embed/avatars/0.png".to_string()
        };

        avatar_image(&avatar_src)
    }

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
        html! {
            @if let Some(url) = &activity.assets.as_ref().and_then(|a| a.large_image.as_ref()) {
                @if let Some(application_id) = &activity.application_id {
                    (activity_img(&discord_img_url(url, application_id)))
                }
            }
            div {
                h3 style="margin-bottom: 10px" { "im playing" }
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

    fn render_relevant_activity(&self) -> Markup {
        if let Some(music) = &self.music_activity {
            Self::render_music(&music)
        } else if let Some(activity) = &self.discord_activities.first() {
            Self::render_activity(&activity)
        } else {
            Self::render_no_activity()
        }
    }
}

impl Render for OptionalLiveActivity {
    fn render(&self) -> Markup {
        match &self.0 {
            Some(status) => section_inner(
                html! {
                    div class="inv-border avatar-border" { (status.avatar_img()) }
                    (section_header(status.discord_user.username.as_str()))
                },
                status.render_relevant_activity(),
            ),
            None => LiveActivity::render_no_activity(),
        }
    }
}
