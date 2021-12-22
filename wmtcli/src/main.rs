use anyhow::Error;
use clap::{AppSettings, Parser, Subcommand};
use tracing::Level;
use wmtlib::{DependencyCheck, Questions};

#[derive(Subcommand, Debug)]
enum Commands {
    /// Commands related to questions. Shows the available question or a specific one.
    #[clap(setting(AppSettings::ArgRequiredElseHelp))]
    Question {
        /// Get a specific question
        #[clap(help="Describe a specific question",
        conflicts_with="list-questions",
        possible_values=["1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12"])]
        number: Option<String>,
        /// Get the list of questions
        #[clap(long = "list-questions", short = 'l', help = "List the questions")]
        list_questions: bool,
    },

    ///Run a check for a dependency or a list of dependencies.
    #[clap(setting(AppSettings::ArgRequiredElseHelp))]
    Check {
        #[clap(
            help = "The name of a dependency to check. You can check multiple dependencies by passing the path to a manifest file, a github url",
            multiple_values = true,
            required = true
        )]
        dependencies: Vec<String>,
        #[clap(help="Check a specific test",
        long = "question", short='q',
        possible_values=["1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12"])]
        question: Option<String>,
    },
}

/// An implementation of Adam Johnson's "The Well-Maintained Test".

/// For a given Rust project/crate/module, wmt checks if the project dependencies pass all 12 tests (or a specific one).
/// The basic syntax is "wmt check [CRATE_NAME]" to run checks for a Cargo crate.

/// For more information see <https://github.com/olamyy/wmt>

#[derive(Parser, Debug)]
#[clap(version = clap::crate_version!(), max_term_width = 100, name="wmt")]
struct WMTCli {
    #[clap(subcommand)]
    command: Commands,
    /// Show the result as json
    #[clap(
        long = "json",
        short = 'j',
        help = "Output the result in JSON",
        global = true
    )]
    json: bool,
    /// Verbosity
    #[clap(
        long = "verbose",
        short = 'v',
        help = "Use verbose output",
        global = true
    )]
    verbose: bool,
}

pub fn run() -> Result<(), Error> {
    let wmt_cli = WMTCli::parse();

    let json: bool = wmt_cli.json;
    let verbose: bool = wmt_cli.verbose;

    match verbose {
        true => setup_logger(),
        false => {}
    }

    match &wmt_cli.command {
        Commands::Check {
            dependencies,
            question,
        } => {
            let dependency_checker = DependencyCheck::new(dependencies.to_owned(), verbose);
            let results = dependency_checker.run_checks(question.to_owned());
            dependency_checker.show_results(json, results)
        }

        Commands::Question {
            number,
            list_questions,
        } => {
            let questions = Questions { verbose };

            if *list_questions {
                let question_list = questions.list();
                questions.show_results(json, question_list.questions);
            }

            match number.as_deref() {
                None => {}
                Some(value) => match Questions::question_numbers().contains(&value.to_string()) {
                    true => {
                        let question = questions.describe(value.to_string());
                        questions.show_results(json, question);
                    }
                    false => {}
                },
            }
        }
    }

    Ok(())
}

fn setup_logger() {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();
}

fn main() {
    match run() {
        Ok(()) => {}
        Err(error) => {
            eprintln!("{}", error);
            std::process::exit(1);
        }
    }
}
