use proc_macro_error::{abort, proc_macro_error};
use proc_macro2::{TokenStream, TokenTree};
use quote::quote;
use std::path::{Path, PathBuf};

#[proc_macro]
#[proc_macro_error]
pub fn include_palettes(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input: TokenStream = input.into();

    let mut palettes = Vec::new();

    for token in input.into_iter() {
        match token {
            TokenTree::Literal(ref literal) => {
                let path = literal.to_string();
                let path = path.trim_matches('"');
                match collect_palettes(&path, &mut palettes) {
                    Ok(_) => {}
                    Err(e) => abort!(
                        token,
                        "failed to collect palettes from '{}': {}",
                        path.to_string(),
                        e
                    ),
                }
            }
            _ => abort!(token, "expected path to palette images"),
        }
    }

    quote! {
        &[
            #(#palettes),*
        ]
    }
    .into()
}

fn relative_path<P: AsRef<Path>>(path: P) -> anyhow::Result<PathBuf> {
    let path = path.as_ref();
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let manifest_dir = Path::new(&manifest_dir);
        manifest_dir.join(path)
    };

    Ok(path.canonicalize()?)
}

fn collect_palettes<P: AsRef<Path>>(
    path: P,
    palettes: &mut Vec<TokenStream>,
) -> anyhow::Result<()> {
    let path = relative_path(path)?;

    for entry in std::fs::read_dir(path)? {
        let entry = entry.unwrap();
        if entry.file_type()?.is_dir() {
            collect_palettes(entry.path(), palettes)?;
            continue;
        }

        let path = entry.path();
        let name = {
            let name = path.file_stem().unwrap();
            let name = name.to_string_lossy();
            let name = name.trim_end_matches("-1x");
            name.replace("-", " ")
        };

        let image = image::open(entry.path())?;

        let image = image.to_rgb8();

        let mut primary = image.get_pixel(0, 0).0;
        let mut secondary = image.get_pixel(1, 0).0;

        if luminance(primary) > luminance(secondary) {
            primary.swap_with_slice(&mut secondary);
        }

        palettes.push(quote! {
            Palette {
                name: #name,
                primary: image::Rgb([#(#primary),*]),
                secondary: image::Rgb([#(#secondary),*]),
            }
        });
    }

    Ok(())
}

fn luminance([r, g, b]: [u8; 3]) -> f32 {
    (0.2126 * r as f32) + 0.7152 * g as f32 + 0.0722 * b as f32
}

