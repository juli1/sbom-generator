use std::env;
use std::process::exit;

use getopts::Options;
use sbom_generator::analyze::sbom_generate::analyze;
use sbom_generator::model::configuration::Configuration;

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} FILE [options]", program);
    print!("{}", opts.usage(&brief));
}

pub fn main() {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

    let mut opts = Options::new();
    opts.optflag("h", "help", "print this help");
    opts.optopt(
        "i",
        "directory",
        "directory to scan (valid existing directory)",
        "/path/to/code/to/analyze",
    );

    opts.optflag("d", "debug", "use debug mode");

    opts.optopt(
        "o",
        "output",
        "file to write the results",
        "/path/to/file.sbom",
    );

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => {
            panic!("error when parsing arguments: {}", f)
        }
    };

    if matches.opt_present("h") {
        print_usage(&program, opts);
        exit(0);
    }

    let directory_to_analyze_option = matches.opt_str("i");
    let output = matches.opt_str("o");

    if directory_to_analyze_option.is_none() {
        eprintln!("missing directory to analyze");
        print_usage(&program, opts);
        exit(1);
    }

    if output.is_none() {
        eprintln!("missing output file");
        print_usage(&program, opts);
        exit(1);
    }

    let configuration = Configuration {
        directory: directory_to_analyze_option.unwrap(),
        output: output.unwrap(),
        use_debug: matches.opt_present("d"),
    };

    analyze(&configuration).expect("error when generating SBOM");
}
