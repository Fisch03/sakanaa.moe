use fishnet::{css, style};
use maud::{html, Markup};

#[derive(Default)]
pub struct SectionConfig<'a> {
    pub id: Option<&'a str>,
    pub is_vertical: bool,
    pub hidden_on_mobile: bool,
    pub at_end: bool,
}

pub async fn section(header: &str, inner: Markup, config: &SectionConfig<'_>) -> Markup {
    section_raw(section_inner(section_header(header), inner).await, config).await
}

pub async fn section_raw(inner: Markup, config: &SectionConfig<'_>) -> Markup {
    style!(
        "section",
        css! {
            border-radius: 3rem;
            overflow: hidden;

            &.vertical {
                display: flex;
                align-items: stretch;
                min-height: 7rem;
                width: 100%;
            }

            &.vertical > .section-header {
                border-radius: 3rem 0 0 3rem;
                border-right: 1px solid var(--fg-color);
                min-height: 7rem;
                width: 5rem;
                padding: 0;
                height: unset;
                max-height: unset;
            }

            &.vertical > .section-header > h2 {
                transform: rotate(-90deg);
                transform-origin: 50% 50%;
            }

            &.vertical > .section-content {
                float: left;
                height: 100%;
                width: 100%;
                padding: 0;
            }
        }
    );

    let id = config.id.and_then(|id| Some(id.to_string()));

    let mut classes = vec!["section", "background", "inv-shadow", "border"];

    if config.is_vertical {
        classes.push("vertical");
    }

    if config.hidden_on_mobile {
        classes.push("hideonmobile");
    }

    if config.at_end {
        classes.push("atend");
    }

    let classes_str = classes.join(" ");

    html! {
        div class=(classes_str) id=[id] {
            (inner)
        }
    }
}

pub async fn section_inner(header: Markup, content: Markup) -> Markup {
    style!(
        "section-header",
        css! {
            /*border-radius: 3rem 3rem 0 0;*/
            border-bottom: 1px solid var(--fg-color);
            display: flex;
            justify-content: space-between;
            align-items: center;
            gap: 1rem;

            text-align: center;

            padding: .7rem;

            max-height: 5rem;
            min-height: 3rem;
            height: 10vh;

            :has(h2:nth-child(1)) {
                justify-content: space-evenly;
            }

            > h2 {
                border-radius: 0.5rem;
                padding: .4rem;
                height: fit-content;
                margin: 0;
            }
        }
    );

    style!(
        "section-content",
        css! {
            padding: .7rem 1.5rem;

            height: calc(100% - 9rem);
            hyphens: auto;
            position: relative;
        }
    );

    html! {
        div class="section-header ditherbg twox" style="background-image: url('assets/dither/bgdither2x.png');"  {
            (header)
        }
        div class="section-content" {
            (content)
        }
    }
}

pub fn section_header(header: &str) -> Markup {
    html! { h2 class="background" { (header) } }
}

pub fn split_section(sections: &[Markup]) -> Markup {
    html! {
        div class="split-section" {
            @for section in sections {
                (section)
            }
        }
    }
}
