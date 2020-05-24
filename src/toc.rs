use std::cmp::Ordering;
use std::error::Error;
use std::fmt::{Display, Error as FmtError, Formatter};
use std::path::{Path, PathBuf};

use crate::md_ast::{self, MarkdownElement};
use crate::texutil::frag_to_tex;


pub struct TableOfContents {
    title: String,
    front_matter_sections: Vec<TOCEntry>,
    body_matter_sections: Vec<TOCEntry>,
    appendix_sections: Vec<TOCEntry>,
    back_matter_sections: Vec<TOCEntry>,
}
impl TableOfContents {
    pub fn new(title: &str) -> TableOfContents {
        TableOfContents {
            title: title.to_owned(),
            front_matter_sections: Vec::new(),
            body_matter_sections: Vec::new(),
            appendix_sections: Vec::new(),
            back_matter_sections: Vec::new(),
        }
    }

    accessor!(title, str);
    accessor_and_mut!(front_matter_sections, front_matter_sections_mut, Vec<TOCEntry>);
    accessor_and_mut!(body_matter_sections, body_matter_sections_mut, Vec<TOCEntry>);
    accessor_and_mut!(appendix_sections, appendix_sections_mut, Vec<TOCEntry>);
    accessor_and_mut!(back_matter_sections, back_matter_sections_mut, Vec<TOCEntry>);
}

#[derive(Eq, Ord)]
pub enum TOCLevel {
    Part,
    Chapter,
    Section(u32),
}
impl TOCLevel {
    fn num_value(&self) -> u32 {
        match self {
            TOCLevel::Part => 0,
            TOCLevel::Chapter => 1,
            TOCLevel::Section(i) => 2 + i,
        }
    }

    pub fn tex_string(&self) -> String {
        match self {
            TOCLevel::Part => "part".to_owned(),
            TOCLevel::Chapter => "chapter".to_owned(),
            TOCLevel::Section(i) => {
                let mut sect = String::new();
                for _ in 0..*i {
                    sect.push_str("sub");
                }
                sect.push_str("section");
                sect
            },
        }
    }
}
impl PartialEq for TOCLevel {
    fn eq(&self, other: &TOCLevel) -> bool {
        self.num_value() == other.num_value()
    }
}
impl PartialOrd for TOCLevel {
    fn partial_cmp(&self, other: &TOCLevel) -> Option<Ordering> {
        self.num_value().partial_cmp(&other.num_value())
    }
}

pub struct TOCEntry {
    level: TOCLevel,
    title: String,
    path: PathBuf,
    child_entries: Vec<TOCEntry>,
}
impl TOCEntry {
    pub fn new<T: AsRef<str>, P: AsRef<Path>>(level: TOCLevel, title: T, path: P) -> TOCEntry {
        TOCEntry {
            level,
            title: title.as_ref().to_owned(),
            path: path.as_ref().to_path_buf(),
            child_entries: vec![],
        }
    }

    accessor!(level, TOCLevel);
    accessor!(title, str);
    accessor!(path, Path);
    accessor_and_mut!(child_entries, child_entries_mut, Vec<TOCEntry>);
}


#[derive(Debug)]
pub struct TOCLoadError {
    message: String,
}
impl TOCLoadError {
    pub fn new<M: AsRef<str>>(message: M) -> TOCLoadError {
        TOCLoadError {
            message: message.as_ref().to_owned(),
        }
    }
    accessor!(message, str);
}
impl Display for TOCLoadError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result<(), FmtError> {
        write!(formatter, "{}", self.message)
    }
}
impl Error for TOCLoadError {
}


fn links_to_toc<'a, E: IntoIterator<Item = &'a MarkdownElement>>(frag: E, section_level: u32) -> Result<Vec<TOCEntry>, String> {
    let mut entries = Vec::new();
    for elem in frag {
        match elem {
            MarkdownElement::Link(url, title_frag) => {
                let title_tex = frag_to_tex(&title_frag)?;
                entries.push(TOCEntry::new(
                    TOCLevel::Section(section_level),
                    title_tex,
                    url,
                ));
            },
            MarkdownElement::List(items) => {
                // last entry has subentries
                let last_entry = match entries.last_mut() {
                    Some(e) => e,
                    None => {
                        return Err("sublist without an entry".to_owned());
                    },
                };

                for subitem in items {
                    let mut sub_entries = links_to_toc(subitem.elements(), section_level + 1)?;
                    last_entry.child_entries_mut().append(&mut sub_entries);
                }
            },
            _ => {
                return Err(format!("unexpected TOC list item: {:?}", elem));
            },
        }
    }
    Ok(entries)
}


pub fn load_toc(book_path: &str) -> Result<TableOfContents, TOCLoadError> {
    // load the table of contents
    let mut toc_path: PathBuf = PathBuf::new();
    toc_path.push(book_path);
    toc_path.push("SUMMARY.md");
    let toc_frag = match md_ast::load(&toc_path) {
        Ok(ast) => ast,
        Err(err) => return Err(TOCLoadError::new(format!(
            "failed to parse TOC: {}", err,
        ))),
    };

    let mut title = String::new();
    let mut front_matter_done = false;
    let mut front_matter_sections = Vec::new();
    let mut body_sections = Vec::new();
    let mut back_matter_sections = Vec::new();
    for elem in toc_frag.elements() {
        match elem {
            MarkdownElement::Heading(1, frag) => {
                title = match frag_to_tex(&frag) {
                    Ok(t) => t,
                    Err(err) => {
                        return Err(TOCLoadError::new(format!(
                            "failed to parse title: {}", err,
                        )));
                    }
                };
            },
            MarkdownElement::Paragraph(frag) => {
                for parelem in frag.elements() {
                    let toc_elems_res = match parelem {
                        MarkdownElement::Link(_, _) => {
                            links_to_toc(
                                vec![parelem],
                                0
                            )
                        },
                        MarkdownElement::List(items) => {
                            links_to_toc(
                                items.iter().flat_map(|frag| frag.elements()),
                                0
                            )
                        },
                        _ => {
                            return Err(TOCLoadError::new(format!(
                                "unexpected TOC paragraph item: {:?}", elem,
                            )));
                        },
                    };
                    let mut toc_elems = match toc_elems_res {
                        Ok(els) => els,
                        Err(err) => {
                            return Err(TOCLoadError::new(format!(
                                "failed to parse paragraph TOC links: {}", err,
                            )));
                        }
                    };
                    if front_matter_done {
                        back_matter_sections.append(&mut toc_elems);
                    } else {
                        front_matter_sections.append(&mut toc_elems);
                    }
                }
            },
            MarkdownElement::List(entries) => {
                // front matter are paragraphs before the first list
                front_matter_done = true;

                for entry in entries {
                    let mut toc_elems = match links_to_toc(entry.elements(), 0) {
                        Ok(els) => els,
                        Err(err) => {
                            return Err(TOCLoadError::new(format!(
                                "failed to parse list TOC links: {}", err,
                            )));
                        }
                    };
                    body_sections.append(&mut toc_elems);
                }
            },
            _ => {
                return Err(TOCLoadError::new(format!(
                    "unexpected TOC item: {:?}", elem,
                )));
            }
        }
    }

    let mut toc = TableOfContents::new(&title);
    toc.front_matter_sections_mut().append(&mut front_matter_sections);
    toc.body_matter_sections_mut().append(&mut body_sections);
    toc.back_matter_sections_mut().append(&mut back_matter_sections);

    Ok(toc)
}
