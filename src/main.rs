extern crate katex;

use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

use clap::{App, Arg, ArgMatches, SubCommand};
use mdbook::book::{Book, BookItem};
use mdbook::errors::Error;
use mdbook::preprocess::{CmdPreprocessor, Preprocessor, PreprocessorContext};
use std::io;
use std::process;

pub fn make_app() -> App<'static, 'static> {
    App::new("mdbook-katex")
        .about("A preprocessor that converts KaTex equations into html.")
        .subcommand(
            SubCommand::with_name("supports")
                .arg(Arg::with_name("renderer").required(true))
                .about("Check whether a renderer is supported by this preprocessor"),
        )
        .arg(
            Arg::from_usage("--macros=[FILE]")
                .takes_value(true)
                .required(false),
        )
        .about("Add the path to user-defined KaTex macros.")
}

fn main() {
    let matches = make_app().get_matches();
    let macros_path;
    if let Some(path) = matches.value_of("macros") {
        macros_path = Some(String::from(path));
    } else {
        macros_path = None;
    }
    let preprocessor = KatexProcessor::new(macros_path);
    if let Some(sub_args) = matches.subcommand_matches("supports") {
        handle_supports(&preprocessor, sub_args);
    }
    let result = handle_preprocessing(&preprocessor);
    if let Err(e) = result {
        eprintln!("{}", e);
    }
}

fn handle_preprocessing(pre: &dyn Preprocessor) -> Result<(), Error> {
    let (ctx, book) = CmdPreprocessor::parse_input(io::stdin())?;
    let processed_book = pre.run(&ctx, book)?;
    serde_json::to_writer(io::stdout(), &processed_book)?;
    Ok(())
}

fn handle_supports(pre: &dyn Preprocessor, sub_args: &ArgMatches) -> ! {
    let renderer = sub_args.value_of("renderer").expect("Required argument");
    let supported = pre.supports_renderer(&renderer);
    // Signal whether the renderer is supported by exiting with 1 or 0.
    if supported {
        process::exit(0);
    } else {
        process::exit(1);
    }
}

struct KatexProcessor {
    macros_path: Option<String>,
}

impl KatexProcessor {
    fn new(macros_path: Option<String>) -> Self {
        Self { macros_path }
    }

    // Take as input the content of a Chapter, and returns a String corresponding to the new content.
    fn process(&self, content: &str) -> String {
        let macros = self.load_macros();
        self.render(&content, macros)
    }

    fn load_macros(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        if let Some(path) = &self.macros_path {
            let macro_str = load_as_string(&path);
            for couple in macro_str.split("\n") {
                match couple.chars().next() {
                    Some(c) => {
                        if c == '\\' {
                            let couple: Vec<&str> = couple.split(":").collect();
                            map.insert(String::from(couple[0]), String::from(couple[1]));
                        } else {
                            ();
                        }
                    }
                    None => (),
                }
            }
        }
        map
    }

    fn render(&self, content: &str, macros: HashMap<String, String>) -> String {
        let header = r#"<link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/katex@0.12.0/dist/katex.min.css" integrity="sha384-AfEj0r4/OFrOo5t7NnNe46zW/tFgW6x/bCJG8FqQCEo3+Aro6EYUG4+cU+KJWu/X" crossorigin="anonymous">"#;
        let mut html = String::from(header);
        html.push_str("\n\n");
        let content = self.render_separator(content, "$$", true, macros.clone());
        let content = self.render_separator(&content, "$", false, macros.clone());
        html.push_str(&content);
        html
    }

    fn render_separator(
        &self,
        string: &str,
        separator: &str,
        display: bool,
        macros: HashMap<String, String>,
    ) -> String {
        let mut html = String::new();
        let mut k = 0;
        for item in string.split(separator) {
            if k % 2 == 1 {
                let ops = katex::Opts::builder()
                    .display_mode(display)
                    .output_type(katex::OutputType::Html)
                    .macros(macros.clone())
                    .build()
                    .unwrap();
                let result = katex::render_with_opts(&item, ops);
                if let Ok(rendered) = result {
                    html.push_str(&rendered)
                } else {
                    html.push_str(&item)
                }
            } else {
                html.push_str(&item)
            }
            k += 1;
        }
        html
    }
}

impl Preprocessor for KatexProcessor {
    fn name(&self) -> &str {
        "katex"
    }

    fn run(&self, ctx: &PreprocessorContext, book: Book) -> Result<Book, Error> {
        let mut new_book = book.clone();
        new_book.for_each_mut(|item| {
            if let BookItem::Chapter(chapter) = item {
                chapter.content = self.process(&chapter.content)
            }
        });
        Ok(new_book)
    }

    fn supports_renderer(&self, renderer: &str) -> bool {
        renderer == "html"
    }
}

fn load_as_string(path: &str) -> String {
    // Create a path to the desired file
    let path = Path::new(path);
    let display = path.display();

    // Open the path in read-only mode, returns `io::Result<File>`
    let mut file = match File::open(&path) {
        Err(why) => panic!("couldn't open {}: {}", display, why),
        Ok(file) => file,
    };

    // Read the file contents into a string, returns `io::Result<usize>`
    let mut string = String::new();
    match file.read_to_string(&mut string) {
        Err(why) => panic!("couldn't read {}: {}", display, why),
        Ok(_) => (),
    };
    string
}