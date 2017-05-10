use std::path::Path;
use std::collections::{VecDeque};

use serde_json;
use handlebars::{Handlebars, HelperDef, RenderError, RenderContext, Helper};
// use pulldown_cmark::{Parser, html, Event, Tag};
use book::BookItem;

// Handlebars helper to construct TOC
#[derive(Clone, Copy)]
pub struct RenderToc;

impl HelperDef for RenderToc {
    fn call(&self, _h: &Helper, _: &Handlebars, rc: &mut RenderContext) -> Result<(), RenderError> {

        // get value from context data
        // rc.get_path() is current json parent path, you should always use it like this
        // param is the key of value you want to display
        let chapters = rc.context().navigate(rc.get_path(), &VecDeque::new(), "chapters").to_owned();
        let current = rc.context().navigate(rc.get_path(), &VecDeque::new(), "path").to_string().replace("\"", "");
        //rc.writer.write_all("<ul class=\"chapter\">".as_bytes())?;


        let items: Vec<BookItem> = serde_json::from_str(&chapters.to_string()).unwrap();
        //println!("yay got bookitems!: {:?}", items);

        //let mut current_level = 1;
        let curr = Path::new(&current);
        write!(rc.writer, "<ol class=\"chapter\">")?;
        for item in items {
            item_to_li(item, &curr, rc)?;
        }
        write!(rc.writer, "</ol>")?;

        Ok(())
    }
}

fn item_to_li(item: BookItem, current: &Path, rc: &mut RenderContext) -> Result<(), RenderError> {
    match item {
        BookItem::Spacer => write!(rc.writer, "<li class=\"spacer\"></li>")?,
        BookItem::Affix(ch) => {
            write!(rc.writer, "<li class=\"affix\"><a href=\"/{}\"", ch.path.to_str().unwrap())?;
            if ch.path == current {
                write!(rc.writer, " class=\"active\"")?;
            }
            write!(rc.writer, ">{}</a></li>", ch.name)?;
        },
        BookItem::Chapter(ch) => {
            let slug = ch.path.file_stem().unwrap().to_str().unwrap();
            write!(rc.writer, "<li class=\"item\"><a name=\"{}\" href=\"/{}\"", slug, ch.path.to_str().unwrap())?;
            if ch.path == current {
                write!(rc.writer, " class=\"active\"")?;
            }
            write!(rc.writer, ">{}</a>", ch.name)?;
            if !ch.sub_items.is_empty() {
                write!(rc.writer, "<ol class=\"section\">")?;
                for sub in ch.sub_items {
                    item_to_li(sub, current, rc)?;
                }
                write!(rc.writer, "</ol>")?;
            }
            write!(rc.writer, "</li>")?;
        }
    }
    Ok(())
}
