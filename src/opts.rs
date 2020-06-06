use clap::Clap;

#[derive(Clap)]
#[clap()]
pub(crate) struct Opts {
    #[clap(default_value = "src", about = "The directory from which to load the book.")]
    pub directory: String,

    #[clap(default_value = "book.tex", about = "The output TeX file.")]
    pub out_file: String,
}
