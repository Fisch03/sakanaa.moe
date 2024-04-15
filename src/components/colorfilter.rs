use fishnet::component::prelude::*;

/*
<filter id="colorfilter" color-interpolation-filters="sRGB">
<feComponentTransfer>
    <feFuncR type="linear" slope="1" class="colorfilterbrightness" />
    <feFuncG type="linear" slope="1" class="colorfilterbrightness" />
    <feFuncB type="linear" slope="1" class="colorfilterbrightness" />
</feComponentTransfer>
<feComponentTransfer>
    <feFuncR type="table" tableValues="1 0" />
    <feFuncG type="table" tableValues="1 0" />
    <feFuncB type="table" tableValues="1 0" />
</feComponentTransfer>
<feColorMatrix id="colorfiltermatrix" type="matrix" values="1 0 0 0 0  0 1 0 0 0  0 0 1 0 0  0 0 0 1 0" />
</filter>
*/

pub fn colorfilter() -> impl BuildableComponent {
    Component::new("colorfilter")
        .add_script(ScriptType::External("js/colors.js".into()))
        .render(|_| {
            html! {
                svg xmlns="http://www.w3.org/2000/svg" {
                    defs {
                        filter id="colorfilter" color-interpolation-filters="sRGB" {
                            feComponentTransfer {
                                feFuncR type="linear" slope="1" class="colorfilterbrightness" {}
                                feFuncG type="linear" slope="1" class="colorfilterbrightness" {}
                                feFuncB type="linear" slope="1" class="colorfilterbrightness" {}
                            }
                            feComponentTransfer {
                                feFuncR type="table" tableValues="1 0" {}
                                feFuncG type="table" tableValues="1 0" {}
                                feFuncB type="table" tableValues="1 0" {}
                            }
                            feColorMatrix id="colorfiltermatrix" type="matrix" values="1 0 0 0 0  0 1 0 0 0  0 0 1 0 0  0 0 0 1 0" {}
                        }
                    }
                }
            }
        })
}
