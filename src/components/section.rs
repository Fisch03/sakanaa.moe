use maud::{html, Markup};

pub struct HTMXConfig<'a> {
    pub get: &'a str,
    pub trigger: &'a str,
}

#[derive(Default)]
pub struct SectionConfig<'a> {
    pub id: Option<&'a str>,
    pub htmx: Option<HTMXConfig<'a>>,
    pub is_vertical: bool,
    pub hidden_on_mobile: bool,
    pub at_end: bool,
}

pub fn section(header: &str, inner: Markup, config: &SectionConfig) -> Markup {
    section_raw(section_inner(section_header(header), inner), config)
}

pub fn section_raw(inner: Markup, config: &SectionConfig) -> Markup {
    let htmx_config = config.htmx.as_ref();
    let hx_get = htmx_config.and_then(|c| Some(c.get));
    let hx_trigger = htmx_config.and_then(|c| Some(c.trigger));
    let id = config.id.and_then(|id| Some(id.to_string()));

    let mut classes = vec!["container", "background", "inv-shadow", "border"];

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
        div class=(classes_str) id=[id] hx-get=[hx_get] hx-trigger=[hx_trigger] {
            (inner)
        }
    }
}

pub fn section_inner(header: Markup, content: Markup) -> Markup {
    html! {
        div class="sectionheader ditherbg twox" style="background-image: url('assets/dither/bgdither2x.png');"  {
            (header)
        }
        div class="sectioncontent" {
            (content)
        }
    }
}

pub fn section_header(header: &str) -> Markup {
    html! { h2 class="background" { (header) } }
}

pub fn split_section(sections: &[Markup]) -> Markup {
    html! {
        div class="columnsection" {
            @for section in sections {
                (section)
            }
        }
    }
}
