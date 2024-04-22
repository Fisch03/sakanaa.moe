use fishnet::component::prelude::*;

pub fn big_waifu(src: &str) -> impl BuildableComponent {
    component!(BigWaifu)
        .with_state(Arc::new(src.to_string()))
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
            
            @media screen and (max-width: 1550px) {
                > img {
                    transform: translate(60vw, 0);
                }
            }

            @media screen and (((orientation: portrait) and (max-width: 1550px)) or (max-width: 750px)) {
                display: none;
            }
        })
        .render(|src| {
            async move {
                html! {
                    img src=(src.as_ref()) class="shadow paletteimg music_reactive" {}
                }
            }
            .boxed()
        })
}
