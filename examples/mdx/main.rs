//! Compile `src/Guide.mdx` to a JavaScript module and print it.
//!
//! ```sh
//! cargo run --example mdx                 # plain JSX: <h1>, components must be imported
//! cargo run --example mdx -- --components # @mdx-js/mdx-style `_components` mapping
//! cargo run --example mdx -- --provider   # … plus `useMDXComponents` from @mdx-js/react
//! ```

use ironmark::{JsxOptions, ParseOptions, render_jsx_with};

const GUIDE: &str = include_str!("src/Guide.mdx");

fn main() {
    let mode = std::env::args().nth(1).unwrap_or_default();
    let jsx = JsxOptions {
        components: mode == "--components",
        provider_import_source: (mode == "--provider").then(|| "@mdx-js/react".to_string()),
    };
    match render_jsx_with(GUIDE, &ParseOptions::default(), &jsx) {
        Ok(out) => {
            print!("{}", out.to_module());
            if !out.references.is_empty() {
                eprintln!("// components to provide: {}", out.references.join(", "));
            }
        }
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    }
}
