use common::gfx::{DitherImage, DitherPattern, DitherSettings, Palette};
use maud::{html, Markup};

pub fn shared() -> Markup {
    html! {
        meta charset="utf-8";
        meta name="viewport" content="width=device-width, initial-scale=1.0";

        (color_style(Palette::random()))
        link rel="stylesheet" href="/style/main.css";
    }
}

pub fn color_style(palette: Palette) -> Markup {
    let pattern = DitherPattern::default();

    #[rustfmt::skip]
    const BG_LEVELS: &[(&str, usize)] = &[
        ("primary",   4), 
        ("secondary", 2), 
        // ("tertiary",  1), 
    ];

    let color_levels = [
        ("primary", palette.primary),
        ("secondary", palette.secondary),
    ];

    html! {
        style data-hmr-ignore {
            ":root {"
                // "--pattern-size: " (pattern.size()) "px;"

                @for (level, color) in color_levels {
                    "--" (level) "-col: rgb(" (color[0]) ", " (color[1]) ", " (color[2]) ");"
                }


                @for (name, level) in BG_LEVELS {
                    @let settings = DitherSettings::new(pattern, *level);
                    @let img = DitherImage::new(palette, settings);
                    @let url = img.to_data_url();

                    "--" (name) "-bg: url('" (url) "');"
                }

            "}"
        }
    }
}
