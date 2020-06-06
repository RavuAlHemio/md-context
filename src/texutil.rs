use lazy_static::lazy_static;
use regex::{Captures, Regex};

use crate::md_ast::{MarkdownElement, MarkdownFormat, MarkdownFragment};


lazy_static! {
    static ref EDUCATED_QUOTE_RE: Regex = Regex::new("(?m)(^|.)\"").unwrap();
}


pub fn escape_tex(text: &str) -> String {
    let mut ret = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\\' | '~' | '{' | '}' | '#' | '%' | '$' => ret.push_str(&format!("\\char`\\{}", c)),
            '\u{FFFD}' => {
                let c_int: u32 = c.into();
                ret.push_str(&format!("\\char\"{:04X}", c_int));
            },
            other => ret.push(other),
        }
    }
    ret
}


pub fn educate_tex_quotes(text: &str) -> String {
    let rep1res = EDUCATED_QUOTE_RE.replace_all(
        text,
        |caps: &Captures| {
            let pre_char = caps.get(1).unwrap().as_str();
            if pre_char.is_empty() || pre_char.chars().all(|c| c.is_whitespace()) {
                // beginning of line or following whitespace => opening quote
                format!("{}\u{201C}", pre_char)
            } else {
                // closing quote
                format!("{}\u{201D}", pre_char)
            }
        },
    );
    rep1res.into_owned()
}


#[derive(PartialEq, Eq)]
enum TypingState {
    Closed,
    Braces,
    Plusses,
}

pub fn to_typing(s: &str) -> String {
    let mut state = TypingState::Closed;
    let mut ret = String::new();
    for c in s.chars() {
        match c {
            '{' | '}' => {
                if state == TypingState::Braces {
                    ret.push_str("}");
                    state = TypingState::Closed;
                }

                if state == TypingState::Closed {
                    ret.push_str("\\type+");
                    state = TypingState::Plusses;
                }
            },
            _ => {
                if state == TypingState::Plusses {
                    ret.push_str("+");
                    state = TypingState::Closed;
                }

                if state == TypingState::Closed {
                    ret.push_str("\\type{");
                    state = TypingState::Braces;
                }
            },
        }
        ret.push(c);
    }

    let final_str: &str = match state {
        TypingState::Braces => "}",
        TypingState::Plusses => "+",
        TypingState::Closed => "",
    };
    ret.push_str(final_str);

    ret
}

pub fn frag_to_collected_text(frag: &MarkdownFragment) -> Result<String, String> {
    let mut ret = String::new();
    for elem in frag.elements() {
        match elem {
            MarkdownElement::Text(text) => {
                ret.push_str(text);
            },
            _ => {
                return Err(format!("unknown element type {:?} when collecting text", elem));
            }
        }
    }
    Ok(ret)
}

pub fn frag_to_tex(frag: &MarkdownFragment) -> Result<String, String> {
    let mut ret = String::new();
    for elem in frag.elements() {
        match elem {
            MarkdownElement::BlockQuote(subfrag) => {
                let subtex = frag_to_tex(subfrag)?;
                ret.push_str("\\startblockquote\n");
                ret.push_str(&subtex);
                ret.push_str("\\stopblockquote\n\n");
            },
            MarkdownElement::Code(subfrag) => {
                // special handling for curly braces
                let subfrag_escaped = to_typing(subfrag);
                ret.push_str(&subfrag_escaped);
            },
            MarkdownElement::CodeBlock(subfrag) => {
                let subtex = frag_to_collected_text(subfrag)?;
                // FIXME: write to file and use \typefile instead?
                ret.push_str("\\starttyping\n");
                ret.push_str(&subtex);
                ret.push_str("\\stoptyping\n\n");
            },
            MarkdownElement::Formatting(fmt, subfrag) => {
                let subtex = frag_to_tex(subfrag)?;
                match fmt {
                    MarkdownFormat::Strikethrough => {
                        ret.push_str("\\overstrike{");
                        ret.push_str(&subtex);
                        ret.push_str("}");
                    },
                    _ => {
                        ret.push_str("{");
                        match fmt {
                            MarkdownFormat::Emphasis => ret.push_str("\\it "),
                            MarkdownFormat::Strong => ret.push_str("\\bf "),
                            _ => {
                                return Err(format!("unexpected formatting type: {:?}", fmt));
                            },
                        }
                        ret.push_str(&subtex);
                        ret.push_str("}");
                    },
                }
            },
            MarkdownElement::Heading(level, subfrag) => {
                if *level == 1 {
                    // the heading of this level is already output as part of descending the ToC
                    continue;
                }

                let subtex = frag_to_tex(subfrag)?;

                ret.push_str("\\");
                let sub_count = level - 1;
                for _ in 0..sub_count {
                    ret.push_str("sub");
                }
                ret.push_str("section{");
                ret.push_str(&subtex);
                ret.push_str("}\n");
            },
            MarkdownElement::Link(url, subfrag) => {
                let subtex = frag_to_tex(subfrag)?;

                ret.push_str("\\goto{");
                ret.push_str(&subtex);
                ret.push_str("}[url(");
                ret.push_str(url);
                ret.push_str(")]");
            },
            MarkdownElement::Image(url, _subfrag) => {
                //let subtex = frag_to_tex(subfrag)?;

                ret.push_str("\\externalfigure[");
                ret.push_str(url);
                ret.push_str("]");
            },
            MarkdownElement::List(items) => {
                ret.push_str("\n\\startitemize\n");
                for item in items {
                    let subtex = frag_to_tex(item)?;

                    ret.push_str("\\item ");
                    ret.push_str(&subtex);
                    ret.push_str("\n");
                }
                ret.push_str("\\stopitemize\n");
            },
            MarkdownElement::Paragraph(subfrag) => {
                let subtex = frag_to_tex(subfrag)?;

                ret.push_str(&subtex);
                ret.push_str("\n\n");
            },
            MarkdownElement::Table(table) => {
                for (i, alignment) in table.alignments().iter().enumerate() {
                    let align_keyword = match alignment {
                        'l' => "flushleft",
                        'r' => "flushright",
                        'c' => "middle",
                        _ => "",
                    };
                    if align_keyword.is_empty() {
                        continue;
                    }
                    ret.push_str(&format!("\\setupTABLE[c][{}][align={}]\n", i+1, align_keyword));
                }
                ret.push_str("\\bTABLE\n");
                let types_rows = vec![
                    ("TH", table.header_rows()),
                    ("TD", table.body_rows()),
                ];
                for (t, rows) in types_rows {
                    for row in rows {
                        ret.push_str("\\bTR\n");
                        for col in row {
                            ret.push_str(&format!("\\b{} ", t));
                            let coltex = frag_to_tex(col)?;
                            ret.push_str(&coltex);
                            ret.push_str(&format!(" \\e{}\n", t));
                        }
                        ret.push_str("\\eTR\n");
                    }
                }
                ret.push_str("\\eTABLE\n\n");
            },
            MarkdownElement::Text(text) => {
                let text = educate_tex_quotes(&escape_tex(&text));
                ret.push_str(&text);
            },
            MarkdownElement::HtmlFragment(html) => {
                let mut mod_html = html.replace("\n", "\n% ");
                mod_html.insert_str(0, "% ");
                mod_html.push_str("\n");
                ret.push_str(&mod_html);
            },
            MarkdownElement::FootnoteRef(foot_name) => {
                ret.push_str("\\note[");
                ret.push_str(&foot_name);
                ret.push_str("]");
            },
            _ => {
                return Err(format!("unknown element type {:?}", elem));
            },
        }
    }
    Ok(ret)
}
