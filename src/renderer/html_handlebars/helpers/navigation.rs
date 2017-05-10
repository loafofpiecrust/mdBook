use std::path::Path;
use std::collections::{VecDeque, BTreeMap};

use serde_json;
use handlebars::{Handlebars, RenderError, RenderContext, Helper, Renderable};

use book::bookitem::{Chapter, BookItem};


// Handlebars helper for navigation
pub fn previous(_h: &Helper, r: &Handlebars, rc: &mut RenderContext) -> Result<(), RenderError> {

    debug!("[fn]: previous (handlebars helper)");

    debug!("[*]: Get data from context");
    // get value from context data
    // rc.get_path() is current json parent path, you should always use it like this
    // param is the key of value you want to display
    let chapters = rc.context().navigate(rc.get_path(), &VecDeque::new(), "chapters").to_owned();

    let current = rc.context().navigate(rc.get_path(), &VecDeque::new(), "path")
        .to_string()
        .replace("\"", "");


    debug!("[*]: Decode chapters from JSON");
    // Decode json format
    let decoded: Vec<BookItem> = match serde_json::from_str(&chapters.to_string()) {
        Ok(data) => data,
        Err(_) => return Err(RenderError::new("Could not decode the JSON data")),
    };
    let mut previous: Option<Chapter> = None;

    for item in decoded {
        match item {
            BookItem::Chapter(ch) | BookItem::Affix(ch) => {
                if ch.path != Path::new("") {
                    if ch.path == Path::new(&current) {
                        match previous {
                            Some(ref mut prev) => {
                                prev.path.set_extension("");

                                debug!("[*]: Inject in context");
                                let mut data = BTreeMap::new();
                                data.insert("title".to_owned(), json!(prev.name));
                                data.insert("link".to_owned(), json!(prev.path.to_str().unwrap()));

                                // Inject in current context
                                let updated_context = rc.context().extend(&data);

                                debug!("[*]: Render template");
                                // Render template
                                match _h.template() {
                                    Some(t) => {
                                        *rc.context_mut() = updated_context;
                                        t.render(r, rc)?;
                                    },
                                    None => return Err(RenderError::new("Error with the handlebars template")),
                                }
                            },
                            _ => (),
                        }
                        break;
                    } else {
                        previous = Some(ch);
                    }
                }
            },
            _ => continue
        }
    }
    Ok(())
}


pub fn next(_h: &Helper, r: &Handlebars, rc: &mut RenderContext) -> Result<(), RenderError> {
    debug!("[fn]: next (handlebars helper)");

    debug!("[*]: Get data from context");
    // get value from context data
    // rc.get_path() is current json parent path, you should always use it like this
    // param is the key of value you want to display
    let chapters = rc.context().navigate(rc.get_path(), &VecDeque::new(), "chapters").to_owned();

    let current = rc.context().navigate(rc.get_path(), &VecDeque::new(), "path")
        .to_string()
        .replace("\"", "");

    debug!("[*]: Decode chapters from JSON");
    // Decode json format
    let decoded: Vec<BookItem> = match serde_json::from_str(&chapters.to_string()) {
        Ok(data) => data,
        Err(_) => return Err(RenderError::new("Could not decode the JSON data")),
    };
    let mut next: Option<Chapter> = None;

    for item in decoded.into_iter().rev() {
        match item {
            BookItem::Chapter(ch) | BookItem::Affix(ch) => {
                if ch.path != Path::new("") {
                    if ch.path == Path::new(&current) {
                        match next {
                            Some(ref mut prev) => {
                                prev.path.set_extension("");

                                debug!("[*]: Inject in context");
                                let mut data = BTreeMap::new();
                                data.insert("title".to_owned(), json!(prev.name));
                                data.insert("link".to_owned(), json!(prev.path.to_str().unwrap()));

                                // Inject in current context
                                let updated_context = rc.context().extend(&data);

                                debug!("[*]: Render template");
                                // Render template
                                match _h.template() {
                                    Some(t) => {
                                        *rc.context_mut() = updated_context;
                                        t.render(r, rc)?;
                                    },
                                    None => return Err(RenderError::new("Error with the handlebars template")),
                                }
                            },
                            _ => (),
                        }
                        break;
                    } else {
                        next = Some(ch);
                    }
                }
            },
            _ => continue
        }
    }
    Ok(())
}
