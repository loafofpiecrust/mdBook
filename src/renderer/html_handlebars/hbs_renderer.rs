use renderer::html_handlebars::helpers;
use renderer::Renderer;
use book::MDBook;
use book::bookitem::BookItem;
use {utils, theme};
use regex::{Regex, Captures};

use std::ascii::AsciiExt;
use std::path::{Path, PathBuf};
use std::fs::{self, File};
use std::error::Error;
use std::io::{self, Read};
use std::collections::HashMap;

use handlebars::Handlebars;

use serde_json;


#[derive(Default)]
pub struct HtmlHandlebars;

impl HtmlHandlebars {
    pub fn new() -> Self {
        HtmlHandlebars
    }
}

impl Renderer for HtmlHandlebars {
    fn render(&self, book: &MDBook) -> Result<(), Box<Error>> {
        debug!("[fn]: render");
        let mut handlebars = Handlebars::new();

        // Load theme
        let theme = theme::Theme::new(book.get_theme_path());

        // Register template
        debug!("[*]: Register handlebars template");
        handlebars.register_template_string("index", try!(String::from_utf8(theme.index)))?;
        handlebars.register_template_string("chapter", try!(String::from_utf8(theme.chapter)))?;

        // Register helpers
        debug!("[*]: Register handlebars helpers");
        handlebars.register_helper("toc", Box::new(helpers::toc::RenderToc));
        handlebars.register_helper("previous", Box::new(helpers::navigation::previous));
        handlebars.register_helper("next", Box::new(helpers::navigation::next));
        handlebars.register_helper("camelCase", Box::new(helpers::text::camel_case));
        handlebars.register_helper("jsonify", Box::new(helpers::text::jsonify));

        let mut data = make_data(book)?;

        // Print version
        let mut print_content: String = String::new();

        // Check if dest directory exists
        debug!("[*]: Check if destination directory exists");
        if fs::create_dir_all(book.get_dest()).is_err() {
            return Err(Box::new(io::Error::new(io::ErrorKind::Other,
                                               "Unexpected error when constructing destination path")));
        }

        // Render the index file (optionally a title page)
        // TODO: Allow an index.md that's inserted as data.content
        let index = handlebars.render("index", &data)?;
        book.write_file(book.get_dest().join("index.html"), &index.into_bytes())?;


        // Render a file for every entry in the book
        // let mut index = true;
        for item in book.iter() {

            match *item {
                BookItem::Chapter(ref ch) |
                BookItem::Affix(ref ch) => {
                    if ch.path != PathBuf::new() {

                        let mut path = book.get_src().join(&ch.path);

                        // path may be:
                        // - Full link: [title](file.md)
                        // - Partial link: [title](file)
                        // - Full name: file.md
                        // - Partial name: file

                        // target of path may be:
                        // - file.md
                        // - directory (no .md)

                        // presuming it's a file, the full file name.
                        let md = path.with_extension("md");

                        if md.is_file() {
                            // Just use this file.
                            path = md;
                        } else if path.is_dir() {
                            // read index.md
                            path = path.join("index.md");
                        } else {
                            // It doesn't exist.
                        }

                        debug!("[*]: Opening file: {:?}", path);
                        let mut f = File::open(&path)?;
                        let mut content = String::new();

                        debug!("[*]: Reading file");
                        f.read_to_string(&mut content)?;

                        // Parse for playpen links
                        if let Some(p) = path.parent() {
                            content = helpers::playpen::render_playpen(&content, p);
                        }

                        // TODO: Make rendering more generic, allowing multiple input formats.
                        // Render markdown using the pulldown-cmark crate
                        content = utils::render_markdown(&content);
                        print_content.push_str(&content);

                        // Update the context with data for this file
                        // NOTE: Removes the extension. May or may not be wanted.
                        let path = ch.path.with_extension("");
                        let path = if book.page_extension {
                            ch.path.to_str().unwrap()
                        } else {
                            path.to_str().unwrap()
                        };
                        // let path = path.to_str().ok_or_else(||
                        //     io::Error::new(io::ErrorKind::Other, "Could not convert path to str"))?;
                        let slug = ch.path.file_stem().ok_or_else(||
                            io::Error::new(io::ErrorKind::Other, "Couldn't get file name"))?.to_str().unwrap();
                        data.insert("path".to_owned(), json!(path));
                        data.insert("slug".to_owned(), json!(slug));
                        data.insert("content".to_owned(), json!(content));
                        data.insert("chapter_title".to_owned(), json!(ch.name));
                        data.insert("path_to_root".to_owned(), json!(utils::fs::path_to_root(&ch.path)));

                        // Render the handlebars template with the data
                        debug!("[*]: Render template");
                        let rendered = handlebars.render("chapter", &data)?;

                        let filename = Path::new(&ch.path).with_extension("html");

                        // Do several kinds of post-processing
                        let rendered = build_header_links(rendered, path);
                        let rendered = fix_anchor_links(rendered, path);
                        let rendered = fix_code_blocks(rendered);
                        let rendered = add_playpen_pre(rendered);

                        // Write to file
                        info!("[*] Creating {:?} ✓", filename.display());
                        try!(book.write_file(filename, &rendered.into_bytes()));

                        // Create an index.html from the first element in SUMMARY.md
                        // if index {
                        //     debug!("[*]: index.html");
                        //
                        //     let mut content = String::new();
                        //     let _source = try!(File::open(book.get_dest().join(&ch.path.with_extension("html"))))
                        //         .read_to_string(&mut content);
                        //
                        //     // This could cause a problem when someone displays code containing <base href=...>
                        //     // on the front page, however this case should be very very rare...
                        //     content = content.lines()
                        //         .filter(|line| !line.contains("<base href="))
                        //         .collect::<Vec<&str>>()
                        //         .join("\n");
                        //
                        //     try!(book.write_file("index.html", content.as_bytes()));
                        //
                        //     info!("[*] Creating index.html from {:?} ✓",
                        //           book.get_dest().join(&ch.path.with_extension("html")));
                        //     index = false;
                        // }
                    }
                },
                _ => {},
            }
        }

        // Print version

        // Update the context with data for this file
        data.insert("path".to_owned(), json!("print.md"));
        data.insert("content".to_owned(), json!(print_content));
        data.insert("path_to_root".to_owned(), json!(utils::fs::path_to_root(Path::new("print.md"))));

        // Render the handlebars template with the data
        debug!("[*]: Render template");

        let rendered = try!(handlebars.render("index", &data));

        // do several kinds of post-processing
        let rendered = build_header_links(rendered, "print.html");
        let rendered = fix_anchor_links(rendered, "print.html");
        let rendered = fix_code_blocks(rendered);
        let rendered = add_playpen_pre(rendered);

        try!(book.write_file(Path::new("print").with_extension("html"), &rendered.into_bytes()));
        info!("[*] Creating print.html ✓");

        // Copy static files (js, css, images, ...)

        debug!("[*] Copy static files");
        try!(book.write_file("book.js", &theme.js));
        try!(book.write_file("book.css", &theme.css));
        try!(book.write_file("favicon.png", &theme.favicon));
        try!(book.write_file("jquery.js", &theme.jquery));
        try!(book.write_file("highlight.css", &theme.highlight_css));
        try!(book.write_file("tomorrow-night.css", &theme.tomorrow_night_css));
        try!(book.write_file("highlight.js", &theme.highlight_js));
        try!(book.write_file("_FontAwesome/css/font-awesome.css", theme::FONT_AWESOME));
        try!(book.write_file("_FontAwesome/fonts/fontawesome-webfont.eot", theme::FONT_AWESOME_EOT));
        try!(book.write_file("_FontAwesome/fonts/fontawesome-webfont.svg", theme::FONT_AWESOME_SVG));
        try!(book.write_file("_FontAwesome/fonts/fontawesome-webfont.ttf", theme::FONT_AWESOME_TTF));
        try!(book.write_file("_FontAwesome/fonts/fontawesome-webfont.woff", theme::FONT_AWESOME_WOFF));
        try!(book.write_file("_FontAwesome/fonts/fontawesome-webfont.woff2", theme::FONT_AWESOME_WOFF2));
        try!(book.write_file("_FontAwesome/fonts/FontAwesome.ttf", theme::FONT_AWESOME_TTF));

        // Copy all remaining files
        try!(utils::fs::copy_files_except_ext(book.get_src(), book.get_dest(), true, &["md"]));

        Ok(())
    }
}

fn make_data(book: &MDBook) -> Result<serde_json::Map<String, serde_json::Value>, Box<Error>> {
    debug!("[fn]: make_data");

    let mut data = serde_json::Map::new();
    data.insert("language".to_owned(), json!("en"));
    data.insert("title".to_owned(), json!(book.get_title()));
    data.insert("description".to_owned(), json!(book.get_description()));
    data.insert("favicon".to_owned(), json!("favicon.png"));
    if let Some(livereload) = book.get_livereload() {
        data.insert("livereload".to_owned(), json!(livereload));
    }

    // NOTE: Simply use the serde default serialization. Why not?
    let chapters: Vec<_> = book.iter().collect();
    println!("inserting chap data: {:?}", json!(chapters));
    data.insert("chapters".to_owned(), json!(chapters));

    debug!("[*]: JSON constructed");
    Ok(data)
}

fn build_header_links(html: String, filename: &str) -> String {
    let regex = Regex::new(r"<h(\d)>(.*?)</h\d>").unwrap();
    let mut id_counter = HashMap::new();

    regex.replace_all(&html, |caps: &Captures| {
        let level = &caps[1];
        let text = &caps[2];
        let mut id = text.to_string();
        let repl_sub = vec!["<em>", "</em>", "<code>", "</code>",
                            "<strong>", "</strong>",
                            "&lt;", "&gt;", "&amp;", "&#39;", "&quot;"];
        for sub in repl_sub {
            id = id.replace(sub, "");
        }
        let id = id.chars().filter_map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                if c.is_ascii() {
                    Some(c.to_ascii_lowercase())
                } else {
                    Some(c)
                }
            } else if c.is_whitespace() && c.is_ascii() {
                Some('-')
            } else {
                None
            }
        }).collect::<String>();

        let id_count = *id_counter.get(&id).unwrap_or(&0);
        id_counter.insert(id.clone(), id_count + 1);

        let id = if id_count > 0 {
            format!("{}-{}", id, id_count)
        } else {
            id
        };

        format!("<a class=\"header\" href=\"{filename}#{id}\" id=\"{id}\"><h{level}>{text}</h{level}></a>",
            level=level, id=id, text=text, filename=filename)
    }).into_owned()
}

// anchors to the same page (href="#anchor") do not work because of
// <base href="../"> pointing to the root folder. This function *fixes*
// that in a very inelegant way
fn fix_anchor_links(html: String, filename: &str) -> String {
    let regex = Regex::new(r##"<a([^>]+)href="#([^"]+)"([^>]*)>"##).unwrap();
    regex.replace_all(&html, |caps: &Captures| {
        let before = &caps[1];
        let anchor = &caps[2];
        let after = &caps[3];

        format!("<a{before}href=\"{filename}#{anchor}\"{after}>",
            before=before, filename=filename, anchor=anchor, after=after)
    }).into_owned()
}


// The rust book uses annotations for rustdoc to test code snippets, like the following:
// ```rust,should_panic
// fn main() {
//     // Code here
// }
// ```
// This function replaces all commas by spaces in the code block classes
fn fix_code_blocks(html: String) -> String {
    let regex = Regex::new(r##"<code([^>]+)class="([^"]+)"([^>]*)>"##).unwrap();
    regex.replace_all(&html, |caps: &Captures| {
        let before = &caps[1];
        let classes = &caps[2].replace(",", " ");
        let after = &caps[3];

        format!("<code{before}class=\"{classes}\"{after}>", before=before, classes=classes, after=after)
    }).into_owned()
}

fn add_playpen_pre(html: String) -> String {
    let regex = Regex::new(r##"((?s)<code[^>]?class="([^"]+)".*?>(.*?)</code>)"##).unwrap();
    regex.replace_all(&html, |caps: &Captures| {
        let text = &caps[1];
        let classes = &caps[2];
        let code = &caps[3];

        if classes.contains("language-rust") && !classes.contains("ignore") {
            // wrap the contents in an external pre block

            if text.contains("fn main") {
                format!("<pre class=\"playpen\">{}</pre>", text)
            } else {
                // we need to inject our own main
                let (attrs, code) = partition_source(code);
                format!("<pre class=\"playpen\"><code class=\"{}\"># #![allow(unused_variables)]
{}#fn main() {{
{}
#}}</code></pre>", classes, attrs, code)
            }
        } else {
            // not language-rust, so no-op
            format!("{}", text)
        }
    }).into_owned()
}

fn partition_source(s: &str) -> (String, String) {
    let mut after_header = false;
    let mut before = String::new();
    let mut after = String::new();

    for line in s.lines() {
        let trimline = line.trim();
        let header = trimline.chars().all(|c| c.is_whitespace()) ||
            trimline.starts_with("#![");
        if !header || after_header {
            after_header = true;
            after.push_str(line);
            after.push_str("\n");
        } else {
            before.push_str(line);
            before.push_str("\n");
        }
    }

    (before, after)
}
