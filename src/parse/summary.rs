use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::{Read, Result, Error, ErrorKind};
use book::bookitem::{BookItem, Chapter};

pub fn construct_bookitems(path: &PathBuf) -> Result<Vec<BookItem>> {
    debug!("[fn]: construct_bookitems");
    let mut summary = String::new();
    File::open(path)?.read_to_string(&mut summary)?;

    debug!("[*]: Parse SUMMARY.md");
    let top_items = parse_level(path.parent().unwrap(), &mut summary.split('\n').collect(), 0)?;
    debug!("[*]: Done parsing SUMMARY.md");
    Ok(top_items)
}

// TODO: Simplify SUMMARY.md. Name all chapters by the first header in their file. If a directory is given, go by dir/index.md.
// TODO: Subsections have their own SUMMARY.md that gets read by the root one to generate the full TOC.
// TODO: use a slice instead of popping from the front of a vector (eww, atl. use a deque).
fn parse_level(path: &Path, summary: &mut Vec<&str>, current_level: i32) -> Result<Vec<BookItem>> {
    debug!("[fn]: parse_level");
    let mut items: Vec<BookItem> = vec![];

    // Construct the book recursively
    while !summary.is_empty() {
        // Indentation level of the line to parse
        let level = level(summary[0], 4)?;

        // if level < current_level we remove the last digit of section, exit the current function,
        // and return the parsed level to the calling function.
        if level < current_level {
            break;
        }

        // if level > current_level we call ourselves to go one level deeper
        if level > current_level {
            debug!("[*]: Summary; parsing deeper at {}", level);
            // Level can not be root level !!
            let last = items.pop().expect("There should be at least one item since this can't be the root level");

            if let BookItem::Chapter(mut ch) = last {
                ch.sub_items.append(&mut parse_level(path, summary, level)?);
                items.push(BookItem::Chapter(ch));
                continue;
            } else {
                return Err(Error::new(ErrorKind::Other,
                                      "Your summary.md is messed up\n\n
                        Prefix, \
                                       Suffix and Spacer elements can only exist on the root level.\n
                        \
                                       Prefix elements can only exist before any chapter and there can be \
                                       no chapters after suffix elements."));
            };

        } else {
            // level and current_level are the same, parse the line
            if let Some(parsed_item) = parse_line(path, summary[0]) {
                // Eliminate possible errors and set section to -1 after suffix
                let item = match parsed_item {
                    // error if level != 0 and BookItem is != Chapter
                    BookItem::Affix(_) | BookItem::Spacer if level > 0 => {
                        return Err(Error::new(ErrorKind::Other,
                                              "Your summary.md is messed up\n\n
                        \
                                       Prefix, Suffix and Spacer elements can only exist on the \
                                       root level.\n
                        Prefix \
                                       elements can only exist before any chapter and there can be \
                                       no chapters after suffix elements."))
                    },

                    // error if BookItem == Chapter and section == -1
                    // TODO: Make sure this can be safely removed. This should be an unreachable case.
                    // BookItem::Chapter(_) if section[0] == -1 => {
                    //     return Err(Error::new(ErrorKind::Other,
                    //                           "Your summary.md is messed up\n\n
                    //     \
                    //                    Prefix, Suffix and Spacer elements can only exist on the \
                    //                    root level.\n
                    //     Prefix \
                    //                    elements can only exist before any chapter and there can be \
                    //                    no chapters after suffix elements."))
                    // },

                    x => x,
                };

                summary.remove(0);
                items.push(item)
            } else {
                // If parse_line does not return Some(_) continue...
                summary.remove(0);
            };
        }
    }
    debug!("[*]: Level: {:?}", items);
    Ok(items)
}


fn level(line: &str, spaces_in_tab: i32) -> Result<i32> {
    debug!("[fn]: level");
    let mut spaces = 0;
    let mut level = 0;

    for ch in line.chars() {
        match ch {
            ' ' => spaces += 1,
            '\t' => level += 1,
            _ => break,
        }
        if spaces >= spaces_in_tab {
            level += 1;
            spaces = 0;
        }
    }

    // If there are spaces left, there is an indentation error
    if spaces > 0 {
        debug!("[SUMMARY.md]:");
        debug!("\t[line]: {}", line);
        debug!("[*]: There is an indentation error on this line. Indentation should be {} spaces", spaces_in_tab);
        return Err(Error::new(ErrorKind::Other, format!("Indentation error on line:\n\n{}", line)));
    }

    Ok(level)
}


fn parse_line(root: &Path, l: &str) -> Option<BookItem> {
    debug!("[fn]: parse_line");

    // Remove leading and trailing spaces or tabs
    let line = l.trim_matches(|c: char| c == ' ' || c == '\t').trim();

    // Spacers are "------"
    if line.starts_with("--") {
        debug!("[*]: Line is spacer");
        return Some(BookItem::Spacer);
    }

    line.chars().nth(0).map(|c| {
        match c {
            // List item
            '-' | '*' => {
                debug!("[*]: Line is list element");
                let line = line.split_at(1).1.trim();
                let (name, path) = read_link(line).unwrap_or_else(move || {
                    (String::new(), PathBuf::from(line))
                });

                // TODO: Read the file's first header for the chapter name.
                let mut full_path = PathBuf::from(root.join(&path));
                println!("looking for chapter {:?}", full_path);
                let mut chap = Chapter::new(name.to_owned(), path.clone());
                if full_path.is_dir() {
                    // Directory, so use the index.md in that folder after parsing its SUMMARY.md for items.
                    // NOTE: Assumes that the base chapter file is index.md.

                    match construct_bookitems(&full_path.join("SUMMARY.md").into()) {
                        Ok(mut items) => {
                            for book_item in &mut items {
                                book_item.prepend(&chap);
                            }
                            chap.sub_items = items;
                        },
                        Err(_) => (),
                    }

                    chap.path.push("index.md");
                    full_path.push("index.md");

                } else {
                    // Simple file, use that.
                    chap.path.set_extension("md");
                    full_path.set_extension("md");
                }

                // Now we need to get the chapter name from the file.
                if full_path.is_file() && chap.name.is_empty() {
                    let file = File::open(full_path).unwrap();
                    let mut heading = false;
                    for c in file.chars() {
                        if heading {
                            match c {
                                Ok(c) => match c {
                                    '\n' => break,
                                    c => chap.name.push(c),
                                },
                                _ => break
                            }
                        } else {
                            match c {
                                Ok(c) => match c {
                                    '#' => heading = true,
                                    _ => continue,
                                },
                                _ => break
                            }
                        }
                    }
                }
                Some(BookItem::Chapter(chap))
            }
            // Non-list element
//            '[' => {
//                debug!("[*]: Line is a link element");
//
//                if let Some((name, path)) = read_link(line) {
//                    return Some(BookItem::Affix(Chapter::new(name, path)));
//                } else {
//                    return None;
//                }
//            },
            '#' => None,
            _ => {
                debug!("[*]: Line is a link/plain element");

                if let Some((name, path)) = read_link(line) {
                    Some(BookItem::Affix(Chapter::new(name, path)))
                } else {
                    // TODO: genericize the item chapter code for affix chapters here.
                    None
                }
            }
        }
    }).unwrap_or_default()
}

fn read_link(line: &str) -> Option<(String, PathBuf)> {
    let mut start_delimitor;
    let mut end_delimitor;

    // In the future, support for list item that is not a link
    // Not sure if I should error on line I can't parse or just ignore them...
    if let Some(i) = line.find('[') {
        start_delimitor = i;
    } else {
        debug!("[*]: '[' not found, this line is not a link. Ignoring...");
        return None;
    }

    if let Some(i) = line[start_delimitor..].find("](") {
        end_delimitor = start_delimitor + i;
    } else {
        debug!("[*]: '](' not found, this line is not a link. Ignoring...");
        return None;
    }

    let name = line[start_delimitor + 1..end_delimitor].to_owned();

    start_delimitor = end_delimitor + 1;
    if let Some(i) = line[start_delimitor..].find(')') {
        end_delimitor = start_delimitor + i;
    } else {
        debug!("[*]: ')' not found, this line is not a link. Ignoring...");
        return None;
    }

    let path = PathBuf::from(line[start_delimitor + 1..end_delimitor].to_owned());

    Some((name, path))
}
