use serde::{Serialize, Serializer, Deserialize};
use serde::ser::SerializeStruct;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "t", content = "c")]
pub enum BookItem {
    Chapter(String, Chapter), // String = section
    Affix(Chapter),
    Spacer,
}

impl BookItem {
    pub fn prepend(&mut self, section: String, pre: &Chapter) {
        use self::BookItem::*;
        match *self {
            Chapter(ref mut idx, ref mut ch) => {
                ch.path = pre.path.join(&ch.path);
                *idx = section.clone() + &idx;
                for item in &mut ch.sub_items {
                    item.prepend(section.clone(), pre);
                }
            },
            Affix(..) => (),
            _ => (),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chapter {
    pub name: String,
    pub path: PathBuf,
    pub sub_items: Vec<BookItem>,
}

#[derive(Debug, Clone)]
pub struct BookItems<'a> {
    pub items: &'a [BookItem],
    pub current_index: usize,
    pub stack: Vec<(&'a [BookItem], usize)>,
}

impl Chapter {
    pub fn new(name: String, path: PathBuf) -> Self {
        Chapter {
            name: name,
            path: path,
            sub_items: vec![],
        }
    }
}

//
//impl Serialize for Chapter {
//    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
//        let mut struct_ = serializer.serialize_struct("Chapter", 2)?;
//        struct_.serialize_field("name", &self.name)?;
//        struct_.serialize_field("path", &self.path)?;
//        struct_.end()
//    }
//}
//


// Shamelessly copied from Rustbook
// (https://github.com/rust-lang/rust/blob/master/src/rustbook/book.rs)
impl<'a> Iterator for BookItems<'a> {
    type Item = &'a BookItem;

    fn next(&mut self) -> Option<&'a BookItem> {
        loop {
            if self.current_index >= self.items.len() {
                match self.stack.pop() {
                    None => return None,
                    Some((parent_items, parent_idx)) => {
                        self.items = parent_items;
                        self.current_index = parent_idx + 1;
                    },
                }
            } else {
                let cur = &self.items[self.current_index];

                match *cur {
                    BookItem::Chapter(_, ref ch) | BookItem::Affix(ref ch) => {
                        self.stack.push((self.items, self.current_index));
                        self.items = &ch.sub_items[..];
                        self.current_index = 0;
                    },
                    BookItem::Spacer => {
                        self.current_index += 1;
                    },
                }

                return Some(cur);
            }
        }
    }
}
