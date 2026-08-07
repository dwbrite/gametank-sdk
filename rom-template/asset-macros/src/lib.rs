extern crate proc_macro;

use proc_macro::TokenStream;

use quote::quote;

use serde::{Deserialize, Serialize};
use syn::{parse_macro_input, LitStr};

use proc_macro2::{Ident, Span, TokenStream as TokenStream2};


mod bmp;


#[derive(Serialize, Deserialize, Debug)]
struct Frame {
    frame: FrameFrame,
    #[serde(rename = "spriteSourceSize")]
    sprite_source_size: FrameFrame,
}

#[derive(Serialize, Deserialize, Debug)]
struct FrameFrame {
    x: u8,
    y: u8,
    w: u8,
    h: u8,
}

#[derive(Serialize, Deserialize, Debug)]
struct FrameData {
    frames: Vec<Frame>
}


#[derive(Debug)]
struct Sprite {
    sheet_x: u8,
    sheet_y: u8,
    width: u8,
    height: u8,
    x_offset: u8,
    y_offset: u8,
    // TODO: We'll probably want more at some point. Maybe.
}

struct Inputs {
    static_name: proc_macro2::Ident,
    bmp_path: String,
    json_path: String
}

fn process_input(input: TokenStream) -> Inputs {
    let input = TokenStream2::from(input);

    // Parse the identifier (static name)
    let mut iter = input.into_iter();
    let static_name = match iter.next().expect("Expected identifier for static name").into() {
        proc_macro2::TokenTree::Ident(ident) => ident,
        _ => panic!("Expected an identifier for the static name"),
    };

    // Parse the first comma
    match iter.next().expect("Expected a comma after static name") {
        proc_macro2::TokenTree::Punct(punct) if punct.as_char() == ',' => {},
        _ => panic!("Expected a comma after static name"),
    };

    // Parse the first string literal (bmp path)
    let bmp_path = match iter.next().expect("Expected string literal for bmp path") {
        proc_macro2::TokenTree::Literal(lit) => {
            // Create a new TokenStream and append the Literal to it
            let mut token_stream = proc_macro2::TokenStream::new();
            token_stream.extend(std::iter::once(proc_macro2::TokenTree::Literal(lit)));

            // Parse the TokenStream as a LitStr
            syn::parse2::<LitStr>(token_stream)
                .expect("Expected valid string literal")
                .value()
        },
        _ => panic!("Expected a string literal for bmp path"),
    };


    // Parse the second comma
    match iter.next().expect("Expected a comma after bmp path") {
        proc_macro2::TokenTree::Punct(punct) if punct.as_char() == ',' => {},
        _ => panic!("Expected a comma after bmp path"),
    };

    // Parse the second string literal (json path)
    let json_path = match iter.next().expect("Expected string literal for json path") {
        proc_macro2::TokenTree::Literal(lit) => {
            // Create a new TokenStream and append the Literal to it
            let mut token_stream = proc_macro2::TokenStream::new();
            token_stream.extend(std::iter::once(proc_macro2::TokenTree::Literal(lit)));

            // Parse the TokenStream as a LitStr
            syn::parse2::<LitStr>(token_stream)
                .expect("Expected valid string literal")
                .value()
        },
        _ => panic!("Expected a string literal for json path"),
    };

    Inputs {
        static_name,
        bmp_path,
        json_path,
    }
}


#[proc_macro]
pub fn include_spritesheet(input: TokenStream) -> TokenStream {
    let inputs = process_input(input);
    let static_name = inputs.static_name;
    let sprite_ident = Ident::new(&format!("{}_Sprite", static_name), Span::call_site());
    let spritesheet_ident = Ident::new(&format!("{}_SpriteSheet", static_name), Span::call_site());

    let json_file_contents = std::fs::read_to_string(inputs.json_path.clone())
        .expect(&format!("Failed to read JSON file {:?}", inputs.json_path.clone()));

    let frame_data: FrameData = serde_json::from_str(&json_file_contents).expect("Failed to parse JSON");

    let mut sprites = vec![];
    let num_sprites = frame_data.frames.len();

    for frame in frame_data.frames {
        sprites.push(Sprite {
            sheet_x: frame.frame.x,
            sheet_y: frame.frame.y,
            width: frame.frame.w,
            height: frame.frame.h,
            x_offset: frame.sprite_source_size.x,
            y_offset: frame.sprite_source_size.y,
        });
    }

    let sprite_tokens: Vec<_> = sprites.iter().map(|sprite| {
        let sheet_x = sprite.sheet_x;
        let sheet_y = sprite.sheet_y;
        let width = sprite.width;
        let height = sprite.height;
        let x_offset = sprite.x_offset;
        let y_offset = sprite.y_offset;

        quote! {
            #sprite_ident {
                sheet_x: #sheet_x,
                sheet_y: #sheet_y,
                width: #width,
                height: #height,
                x_offset: #x_offset,
                y_offset: #y_offset,
            }
        }
    }).collect();

    let spritesheetimage = bmp::SpriteSheetImage::load_spritesheet(inputs.bmp_path);

    let pixels_per_byte = spritesheetimage.pixels_per_byte;
    let width = spritesheetimage.width;
    let height = spritesheetimage.height;
    let palette = spritesheetimage.palette;
    let pixel_array = spritesheetimage.pixel_array;
    let pixel_array_size = pixel_array.len();
    let palette_size = palette.len();

    let output = quote! {
        #[derive(Debug, Copy, Clone)]
        pub struct #sprite_ident {
            pub sheet_x: u8,
            pub sheet_y: u8,
            pub width: u8,
            pub height: u8,
            pub x_offset: u8,
            pub y_offset: u8,
        }

        #[derive(Debug, Copy, Clone)]
        pub struct #spritesheet_ident {
            pub pixels_per_byte: u8,
            pub palette: [u8; #palette_size],
            pub width: u8,
            pub height: u8,
            pub sprite_data: [#sprite_ident; #num_sprites],
            pub pixel_array: [u8; #pixel_array_size],
        }

        pub static #static_name: #spritesheet_ident = #spritesheet_ident {
            pixels_per_byte: #pixels_per_byte,
            width: #width as u8,
            height: #height as u8,
            palette: [#(#palette),*],
            sprite_data: [#(#sprite_tokens),*],
            pixel_array: [#(#pixel_array),*],
        };
    };

    output.into()
}

/// Include a BMP file as a byte array.
/// Usage: `include_bmp!("path/to/file.bmp")`
/// 
/// Note: A 128x128 image is 16,384 bytes which exceeds a single 16KB bank.
/// For large images, consider splitting or using `include_bmp_banked!`.
#[proc_macro]
pub fn include_bmp(input: TokenStream) -> TokenStream {
    let path_lit = parse_macro_input!(input as LitStr);
    let path = path_lit.value();

    let pixels = bmp::load_bmp_raw(path);

    let output = quote! {
        [ #( #pixels ),* ]
    };

    output.into()
}

#[proc_macro]
pub fn string_to_indices(input: TokenStream) -> TokenStream {
    let input_string = parse_macro_input!(input as LitStr).value();
    let characters = " ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                      abcdefghijklmnopqrstuvwxyz\
                      1234567890\
                      !?.,;:/\\\"\"()[]{}<>"; // Removed the quote marks from here

    let mut quote_count = 0;
    let left_quote_index = characters.find("\"").unwrap();
    let right_quote_index = left_quote_index + 1; // Index for the right quote

    let indices: Vec<usize> = input_string.chars()
        .map(|c| {
            if c == '"' {
                quote_count += 1;
                if quote_count % 2 == 0 { right_quote_index } else { left_quote_index }
            } else {
                characters.find(c).unwrap_or_else(|| panic!("Character '{}' not found", c))
            }
        })
        .collect();

    let output = quote! {
        [ #( #indices ),* ]
    };

    output.into()
}
