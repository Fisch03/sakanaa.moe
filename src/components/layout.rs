use fishnet::component::prelude::*;

pub async fn content(first_column: Markup, second_column: Markup) -> Markup {
    style!(
        "content",
        css! {
            position: absolute;

            top: 0;
            left: calc(min(6rem, 3vw) + 2.5rem + var(--sidecolumn-width));
            right: calc(min(6rem, 3vw));

            padding: calc(min(3rem, 5vh)) 0;

            width: calc((85vw - 31vh) - var(--sidecolumn-width) - 2.5rem);

            .column {
                display: flex;
                flex-direction: column;
                flex-flow: space-between;
                justify-content: flex-start;
                align-items: flex-start;
                left: 0;
                gap: 2rem;
                height: 100%;
            }

            .column:nth-child(1) {
              position: fixed;
              left: calc(min(6rem, 3vw));
              width: var(--sidecolumn-width);
              height: calc(100vh - 2*min(3rem, 5vh));
            }

            .column > div {
              width: 100%;
            }

            @media screen and (max-width: 1550px) {
                left: calc(min(6rem, 10vw));
                width: calc(65vw - 2.5rem);

                .column:nth-child(1) {
                    position: unset;
                    left: unset;
                    height: unset;
                    width: 100%;
                    margin-bottom: 2rem;
                }
            }

            @media screen and (((orientation: portrait) and (max-width: 1550px)) or (max-width: 750px)) {
                flex-direction: column;
                left: calc(min(6rem, 10vw));
                right: calc(min(6rem, 10vw));
                width: unset;

                .column:nth-child(1) {
                    position: unset;
                    left: unset;
                    height: unset;
                    width: 100%;
                    margin-bottom: 2rem;
                }
            }
        }
    );

    html! {
        div class="content" {
            div class="column" { (first_column)  }
            div class="column" { (second_column) }
        }
    }
}
