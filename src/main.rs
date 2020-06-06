mod macros;
mod md_ast;
mod texutil;
mod toc;

use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::process::exit;

use getopts;


fn usage(program_name: &str) {
    eprintln!("Usage: {} [DIRECTORY [OUTFILE]]", program_name);
    eprintln!();
    eprintln!("  DIRECTORY     The directory from which to load the book.");
    eprintln!("                The default is \"src\".");
    eprintln!("  OUTFILE       The output TeX file. The default is \"book.tex\".");
}

fn output_section(output_file: &mut File, section: &toc::TOCEntry, book_path: &str) -> i32 {
    if let Err(err) = write!(
        output_file,
        "\n\\{lvl}{ob}{t}{cb}\n",
        lvl = section.level().tex_string(),
        ob = '{',
        t = section.title(),
        cb = '}'
    ) {
        eprintln!("failed to output section heading: {}", err);
        return 1;
    }

    if let Some(sp) = section.path() {
        let mut section_path: PathBuf = PathBuf::new();
        section_path.push(book_path);
        section_path.push(sp);
        let section_frag = match md_ast::load(&section_path) {
            Ok(ast) => ast,
            Err(err) => {
                eprintln!("failed to parse section: {}", err);
                return 1;
            },
        };

        let section_tex = match texutil::frag_to_tex(&section_frag) {
            Ok(tex) => tex,
            Err(err) => {
                eprintln!("failed to transform section to TeX: {}", err);
                return 1;
            }
        };

        if let Err(err) = write!(output_file, "{}", section_tex) {
            eprintln!("failed to output section: {}", err);
            return 1;
        }
    }

    for child_section in section.child_entries() {
        let code = output_section(output_file, child_section, book_path);
        if code != 0 {
            return 1;
        }
    }

    0
}

fn output_tex(output_file: &mut File, toc: &toc::TableOfContents, book_path: &str) -> i32 {
    if let Err(err) = write!(
        output_file,
        "\\setupinteraction[title={ob}{t}{cb}]\n\n\\starttext\n\n\\mdcontextplacetoc\n\n",
        ob = '{', t = toc.title(), cb = '}',
    ) {
        eprintln!("error writing preamble: {}", err);
        return 1;
    }

    let sections = vec![
        ("frontmatter", toc.front_matter_sections()),
        ("bodymatter", toc.body_matter_sections()),
        ("appendices", toc.appendix_sections()),
        ("backmatter", toc.back_matter_sections()),
    ];
    for (matter_tex, matter_sections) in sections {
        if matter_sections.is_empty() {
            continue;
        }

        if let Err(err) = write!(output_file, "\n\\start{}\n", matter_tex) {
            eprintln!("error writing opening of {}: {}", matter_tex, err);
            return 1;
        }

        for section in matter_sections {
            let code = output_section(output_file, section, book_path);
            if code != 0 {
                return code;
            }
        }

        if let Err(err) = write!(output_file, "\n\\stop{}\n", matter_tex) {
            eprintln!("error writing end of {}: {}", matter_tex, err);
            return 1;
        }
    }

    if let Err(err) = write!(output_file, "\\stoptext\n") {
        eprintln!("error writing postamble: {}", err);
        return 1;
    }

    0
}

fn do_main() -> i32 {
    let args: Vec<String> = env::args().collect();
    let program_name: String = match args.get(0) {
        Some(pn) => pn,
        None => "md-context",
    }.to_owned();

    let opts = getopts::Options::new();
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(err) => {
            eprintln!("failed to parse arguments: {:?}", err);
            usage(&program_name);
            return 1;
        },
    };

    let book_path_str: String = match matches.free.get(0) {
        Some(bps) => bps,
        None => "src",
    }.to_owned();

    let output_path: String = match matches.free.get(1) {
        Some(op) => op,
        None => "book.tex",
    }.to_owned();
    let mut output_file = match File::create(&output_path) {
        Ok(f) => f,
        Err(err) => {
            eprintln!("failed to open output file {:?}: {:?}", output_path, err);
            return 1;
        },
    };

    let toc = match toc::load_toc(&book_path_str) {
        Err(err) => {
            eprintln!("failed to load TOC: {}", err);
            return 1;
        },
        Ok(t) => t,
    };

    output_tex(&mut output_file, &toc, &book_path_str)
}

fn main() {
    let exit_code = do_main();
    exit(exit_code);
}
