use std::error::Error;
use std::fmt::{Display, Error as FmtError, Formatter};
use std::fs::File;
use std::io::Read;
use std::iter::{FromIterator, IntoIterator};
use std::path::Path;

use pulldown_cmark::{Alignment, Event, Parser, Tag};


#[derive(Debug)]
pub struct MarkdownFragment {
    elements: Vec<MarkdownElement>,
}
impl MarkdownFragment {
    pub fn new<E: IntoIterator<Item = MarkdownElement>>(elements: E) -> MarkdownFragment {
        MarkdownFragment {
            elements: Vec::from_iter(elements),
        }
    }
    accessor_and_mut!(elements, elements_mut, Vec<MarkdownElement>);
}

#[derive(Debug)]
pub enum MarkdownFormat {
    Emphasis,
    Strong,
    Strikethrough,
}

#[derive(Debug)]
pub struct MarkdownTable {
    alignments: Vec<char>,
    header_rows: Vec<Vec<MarkdownFragment>>,
    body_rows: Vec<Vec<MarkdownFragment>>,
}
impl MarkdownTable {
    pub fn new<
            A: IntoIterator<Item = char>,
            HR: IntoIterator<Item = HC>, HC: IntoIterator<Item = MarkdownFragment>,
            BR: IntoIterator<Item = BC>, BC: IntoIterator<Item = MarkdownFragment>,
    >(alignments: A, header_rows: HR, body_rows: BR) -> MarkdownTable {
        MarkdownTable {
            alignments: alignments.into_iter().collect(),
            header_rows: header_rows.into_iter().map(|cols| cols.into_iter().collect()).collect(),
            body_rows: body_rows.into_iter().map(|cols| cols.into_iter().collect()).collect(),
        }
    }

    accessor_and_mut!(alignments, alignments_mut, Vec<char>);
    accessor_and_mut!(header_rows, header_rows_mut, Vec<Vec<MarkdownFragment>>);
    accessor_and_mut!(body_rows, body_rows_mut, Vec<Vec<MarkdownFragment>>);
}

#[derive(Debug)]
pub enum MarkdownElement {
    Text(String),
    Heading(u32, MarkdownFragment),
    Paragraph(MarkdownFragment),
    List(Vec<MarkdownFragment>),
    Link(String, MarkdownFragment),
    Image(String, MarkdownFragment),
    Code(String),
    BlockQuote(MarkdownFragment),
    CodeBlock(MarkdownFragment),
    Formatting(MarkdownFormat, MarkdownFragment),
    Table(MarkdownTable),
}

#[derive(Debug)]
pub struct ASTError {
    message: String,
}
impl ASTError {
    pub fn new<M: AsRef<str>>(message: M) -> ASTError {
        ASTError {
            message: message.as_ref().to_owned(),
        }
    }
}
impl Display for ASTError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result<(), FmtError> {
        write!(formatter, "{}", self.message)
    }
}
impl Error for ASTError {}


fn parse_table_row<'a>(mut parser: &mut Parser<'a>) -> Result<Vec<MarkdownFragment>, ASTError> {
    let mut vals = Vec::new();
    while let Some(event) = parser.next() {
        match event {
            Event::End(Tag::TableRow) | Event::End(Tag::TableHead) => {
                break;
            },
            Event::Start(Tag::TableCell) => {
                let val = parse_until_end_event(&mut parser)?;
                vals.push(val);
            },
            _ => {
                return Err(ASTError::new(format!("unhandled table row parser event {:?}", event)));
            },
        }
    }
    Ok(vals)
}

fn parse_table<'a>(mut parser: &mut Parser<'a>, align_chars: Vec<char>) -> Result<MarkdownTable, ASTError> {
    let mut header_rows = Vec::new();
    let mut body_rows = Vec::new();
    while let Some(event) = parser.next() {
        match event {
            Event::End(Tag::Table(_)) => {
                break;
            },
            Event::Start(Tag::TableHead) => {
                let row = parse_table_row(&mut parser)?;
                header_rows.push(row);
            },
            Event::Start(Tag::TableRow) => {
                let row = parse_table_row(&mut parser)?;
                body_rows.push(row);
            },
            _ => {
                return Err(ASTError::new(format!("unhandled table parser event {:?}", event)));
            },
        }
    }
    Ok(MarkdownTable::new(align_chars, header_rows, body_rows))
}


fn parse_until_end_event<'a>(mut parser: &mut Parser<'a>) -> Result<MarkdownFragment, ASTError> {
    let mut elements = Vec::new();
    while let Some(event) = parser.next() {
        match event {
            Event::End(_) => {
                break;
            },
            Event::Text(body) => {
                elements.push(MarkdownElement::Text(body.as_ref().to_owned()));
            },
            Event::Code(code) => {
                elements.push(MarkdownElement::Code(code.as_ref().to_owned()));
            },
            Event::SoftBreak => {
                elements.push(MarkdownElement::Text("\n".to_owned()));
            },
            Event::Start(Tag::Paragraph) => {
                let subfrag = parse_until_end_event(&mut parser)?;
                elements.push(MarkdownElement::Paragraph(subfrag));
            },
            Event::Start(Tag::Heading(level)) => {
                let subfrag = parse_until_end_event(&mut parser)?;
                elements.push(MarkdownElement::Heading(level, subfrag));
            },
            Event::Start(Tag::List(_)) => {
                let items = parse_list_items(&mut parser)?;
                elements.push(MarkdownElement::List(items));
            },
            Event::Start(Tag::BlockQuote) => {
                let subfrag = parse_until_end_event(&mut parser)?;
                elements.push(MarkdownElement::BlockQuote(subfrag));
            },
            Event::Start(Tag::CodeBlock(_)) => {
                let subfrag = parse_until_end_event(&mut parser)?;
                elements.push(MarkdownElement::CodeBlock(subfrag));
            },
            Event::Start(Tag::Emphasis) | Event::Start(Tag::Strong) | Event::Start(Tag::Strikethrough) => {
                let format: MarkdownFormat = match event {
                    Event::Start(Tag::Emphasis) => MarkdownFormat::Emphasis,
                    Event::Start(Tag::Strong) => MarkdownFormat::Strong,
                    Event::Start(Tag::Strikethrough) => MarkdownFormat::Strikethrough,
                    _ => return Err(ASTError::new(format!("incorrectly handled formatting tag {:?}", event))),
                };
                let subfrag = parse_until_end_event(&mut parser)?;
                elements.push(MarkdownElement::Formatting(format, subfrag));
            },
            Event::Start(Tag::Link(link_type, dest, title)) => {
                // FIXME: don't ignore the title
                let subfrag = parse_until_end_event(&mut parser)?;
                elements.push(MarkdownElement::Link(dest.as_ref().to_owned(), subfrag));
            },
            Event::Start(Tag::Image(link_type, dest, title)) => {
                // FIXME: don't ignore the title
                let subfrag = parse_until_end_event(&mut parser)?;
                elements.push(MarkdownElement::Image(dest.as_ref().to_owned(), subfrag));
            },
            Event::Start(Tag::Table(alignments)) => {
                let align_chars: Vec<char> = alignments.iter().map(|al| match al {
                    Alignment::None => ' ',
                    Alignment::Left => 'l',
                    Alignment::Center => 'c',
                    Alignment::Right => 'r',
                }).collect();
                let table = parse_table(&mut parser, align_chars)?;
                elements.push(MarkdownElement::Table(table));
            },
            _ => {
                return Err(ASTError::new(format!("unhandled parser event {:?}", event)));
            },
        }
    }
    Ok(MarkdownFragment::new(elements))
}

fn parse_list_items<'a>(mut parser: &mut Parser<'a>) -> Result<Vec<MarkdownFragment>, ASTError> {
    let mut items: Vec<MarkdownFragment> = Vec::new();
    while let Some(event) = parser.next() {
        match event {
            Event::End(Tag::List(_)) => {
                break;
            },
            Event::Start(Tag::Item) => {
                let item_frag = parse_until_end_event(&mut parser)?;
                items.push(item_frag);
            },
            evt => {
                return Err(ASTError::new(format!("unexpected event {:?} while parsing list items", evt)));
            }
        }
    }
    Ok(items)
}

pub fn parse<'a>(mut parser: &mut Parser<'a>) -> Result<MarkdownFragment, ASTError> {
    let mut elements: Vec<MarkdownElement> = Vec::new();
    loop {
        let mut subfrag = parse_until_end_event(&mut parser)?;
        if subfrag.elements().is_empty() {
            break;
        }
        elements.append(&mut subfrag.elements);
    }
    Ok(MarkdownFragment::new(elements))
}

pub fn load(path: &Path) -> Result<MarkdownFragment, ASTError> {
    let mut md_file: File = match File::open(&path) {
        Ok(f) => f,
        Err(err) => {
            return Err(ASTError::new(format!(
                "failed to open Markdown file {:?}: {}", path, err,
            )));
        },
    };
    let mut md_string: String = String::new();
    if let Err(err) = md_file.read_to_string(&mut md_string) {
        return Err(ASTError::new(format!(
            "failed to read Markdown file {:?}: {}", path, err,
        )));
    };

    let mut options = pulldown_cmark::Options::empty();
    options.insert(pulldown_cmark::Options::ENABLE_STRIKETHROUGH);
    options.insert(pulldown_cmark::Options::ENABLE_TABLES);
    let mut md_parser = pulldown_cmark::Parser::new_ext(&md_string, options);
    let md_frag = match parse(&mut md_parser) {
        Ok(ast) => ast,
        Err(err) => return Err(ASTError::new(format!(
            "failed to parse Markdown file {:?}: {}", path, err,
        ))),
    };

    Ok(md_frag)
}
