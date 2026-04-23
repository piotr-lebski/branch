use clap::Parser;

fn main() {
    let cli = branch::cli::Cli::parse();
    if let Err(e) = branch::app::run(cli) {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
