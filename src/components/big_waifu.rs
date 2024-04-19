use fishnet::component::prelude::*;

pub fn big_waifu(src: &str) -> impl BuildableComponent {
    let src = src.to_string();

    component!(BigWaifu)
        .style(css! {
            position: fixed;
            top: 0;
            left: 0;
            width: 100%;
            height: 100%;

            display: flex;
            justify-content: left;
            align-items: center;

            z-index: 100;

            pointer-events: none;

            > img {
              position: fixed;
              right: calc(10vw - 15vh);
              height: 100%;
              image-rendering: auto;

              pointer-events: none;
            }
        })
        .render(move |_| {
            html! {
                img src=(src) class="shadow paletteimg music_reactive" {}
            }
        })
}
