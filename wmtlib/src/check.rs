use std::cmp::Ordering;
use std::collections::HashMap;
use std::string::String;
use std::vec::IntoIter;

use chrono::Utc;
use octocrab::models::issues::Issue;
use octocrab::models::workflows::WorkFlow;
use octocrab::params::State;
use serde::Serialize;

use crate::{CommandResult, log_if_verbose, Question, Questions};
use crate::cargo_crate::Crate;
use crate::constants::{
    MAX_DOWNLOAD_FOR_MINOR_VERSION, MISSING_FIELD_PLACEHOLDER, QUESTION_EXPLANATION_SUFFIX,
};
use crate::doc::{DocService, DocSource};
use crate::github::GithubService;
use crate::questions::CheckNames;
use crate::version::Version;

/// Represents a dependency
#[derive(Debug)]
pub struct CrateCheck {
    pub crates: Vec<Crate>,
    pub results: Option<Vec<Vec<CheckResult>>>,
    pub verbose: bool,
}

/// Represents the status of a dependency check.
/// GREEN ===> Completely passes the required test.
/// Yellow ====> Passes the required test but not completely.
/// RED =====> Does not pass the required test
#[derive(Debug, Serialize)]
pub enum Status {
    Green,
    Yellow,
    Red,
    Grey,
}

impl Status {
    pub fn value(&self) -> String {
        match *self {
            Status::Green => String::from("GREEN"),
            Status::Yellow => String::from("Yellow"),
            Status::Red => String::from("RED"),
            Status::Grey => String::from("GREY"),
        }
    }
}

///Represents the result of running checking a question
#[derive(Debug, Serialize)]
pub struct CheckResult {
    question: Option<String>,
    status: Status,
    explanation: String,
}

impl CheckResult {
    pub fn to_vec(&self) -> Vec<String> {
        return vec![
            self.question.to_owned().unwrap(),
            self.status.value(),
            self.explanation.to_string(),
        ];
    }

    pub fn from_error_message(message: String) -> Self {
        CheckResult {
            question: None,
            status: Status::Red,
            explanation: message,
        }
    }
}

impl CrateCheck {
    #[tracing::instrument]
    pub fn new(deps: Vec<String>, verbose: bool) -> Self {
        log_if_verbose(verbose, "Checking dependency source");
        let mut crates_to_check = Vec::new();
        for dep in deps {
            match dep.ends_with(".toml") {
                true => {
                    log_if_verbose(verbose, "Found manifest path. Extracting");
                    let dependencies_from_manifest = Crate::from_manifest(dep);
                    crates_to_check.extend(dependencies_from_manifest);
                }
                false => match dep.starts_with("https://") {
                    true => {
                        log_if_verbose(verbose, "Found source url. Querying");
                        let dependency = Crate::from_source(dep);
                        crates_to_check.push(dependency);
                    }
                    false => {
                        log_if_verbose(verbose, "Found crate name. Extracting crate information");
                        let dependency =
                            Crate::from_name(dep, Some(MISSING_FIELD_PLACEHOLDER.to_string()));
                        crates_to_check.push(dependency);
                    }
                },
            }
        }

        CrateCheck {
            crates: crates_to_check,
            results: Option::None,
            verbose,
        }
    }

    /// Show the command result
    pub fn show_results(&self, json: bool, data: Vec<Vec<CheckResult>>) {
        let command_result = CommandResult {
            as_json: json,
            headers: vec![
                String::from("Question"),
                String::from("Status"),
                String::from("Explanation"),
            ],
        };

        command_result.display_checks_result(data)
    }

    /// Run the checks on a question or list of questions
    pub fn run_checks(&self, question: Option<String>) -> Vec<Vec<CheckResult>> {
        let question = question.unwrap_or_else(|| "0".parse().unwrap());
        let questions = Questions {
            verbose: self.verbose,
        };

        let selected_question = match question.as_str() {
            "0" => {
                log_if_verbose(self.verbose, "Will check all questions");
                questions.list().questions
            }
            _ => {
                log_if_verbose(
                    self.verbose,
                    format!("Will only check for question {}", question).as_str(),
                );
                questions.describe(question)
            }
        };

        let mut results = Vec::new();

        for cargo_crate in &self.crates {
            match cargo_crate.source_url.is_some() {
                true => {
                    for question_to_check in &selected_question {
                        let question_results = self.check_question(cargo_crate, question_to_check);
                        results.push(question_results);
                    }
                }
                false => {
                    log_if_verbose(self.verbose, "Missing source url");
                    eprintln!(
                        "crates.io has no source url for {}. Will ignore.",
                        cargo_crate.name
                    );
                }
            }
        }
        results
    }

    fn no_support_result(&self) -> CheckResult {
        CheckResult {
            question: None,
            status: Status::Grey,
            explanation: "This is currently not supported".to_string(),
        }
    }

    fn check_question(&self, cargo_crate: &Crate, question: &Question) -> Vec<CheckResult> {
        let mut check_results: Vec<CheckResult> = Vec::new();

        let mut check_result = match question.name {
            CheckNames::ProductionReadiness => {
                log_if_verbose(self.verbose, "Checking for production readiness");
                self.check_production_readiness(cargo_crate)
            }
            CheckNames::Documentation => {
                log_if_verbose(self.verbose, "Checking for documentation");
                self.check_documentation(cargo_crate)
            }
            CheckNames::Changelog => {
                log_if_verbose(self.verbose, "Checking for changelog");
                self.check_changelog(cargo_crate)
            }
            CheckNames::Tests => {
                log_if_verbose(self.verbose, "Checking for tests");
                self.check_tests(cargo_crate)
            }
            CheckNames::BugReportResponse => {
                log_if_verbose(self.verbose, "Checking for bug response time");
                self.check_bug_response(cargo_crate)
            }
            CheckNames::TestsRunAgainstLatestLanguageVersion => {
                log_if_verbose(
                    self.verbose,
                    "Checking if the tests run with the latest <Language> version?",
                );
                self.check_runs_against_latest_language(cargo_crate)
            }
            CheckNames::TestsRunAgainstLatestIntegrationVersion => self.no_support_result(),
            CheckNames::ContinuousIntegrationConfiguration => {
                log_if_verbose(self.verbose, "Checking for Github workflows");
                self.check_continuous_integration(cargo_crate)
            }
            CheckNames::ContinuousIntegrationPasses => {
                log_if_verbose(self.verbose, "Checking for CI status");
                self.check_ci_status(cargo_crate)
            }
            CheckNames::Usage => {
                log_if_verbose(self.verbose, "Checking for usage data");
                self.check_usage(cargo_crate)
            }
            CheckNames::LatestCommits => {
                log_if_verbose(self.verbose, "Checking for latest commits");
                self.check_latest_commits(cargo_crate)
            }
            CheckNames::LatestRelease => {
                log_if_verbose(self.verbose, "Checking for latest release");
                self.check_latest_release(cargo_crate)
            }
        };

        check_result.question = Option::from(question.number.to_owned());
        check_results.push(check_result);

        check_results
    }

    ///To check the production readiness, wmt checks the following :
    /// 1. Does the crate have at least one major release?
    /// 2. Does the crate have at least 2 minor releases and a significant number of downloads?
    fn check_production_readiness(&self, cargo_crate: &Crate) -> CheckResult {
        let dependency_version = &cargo_crate.version.as_ref().unwrap();
        let remote_dependency_version = dependency_version.remote.as_ref().unwrap();
        let version = Version::from_version_text(remote_dependency_version);
        match version.at_least_one_major_release() {
            true => {
                log_if_verbose(self.verbose, "Passes AT_LEAST_ONE_MAJOR_RELEASE check");
                CheckResult {
                    question: None,
                    status: Status::Green,
                    explanation: format!(
                        "{} at least one major release",
                        QUESTION_EXPLANATION_SUFFIX
                    ),
                }
            }
            false => match version.at_least_one_minor_release() {
                true => {
                    let downloads = &cargo_crate.downloads;
                    match downloads.ge(&MAX_DOWNLOAD_FOR_MINOR_VERSION) {
                        true => {
                            log_if_verbose(
                                self.verbose,
                                "Passes AT_LEAST_TWO_MINOR_RELEASES check",
                            );
                            CheckResult {
                                question: None,
                                status: Status::Yellow,
                                explanation: format!(
                                    "{} at least two minor releases and at least 500 downloads",
                                    QUESTION_EXPLANATION_SUFFIX
                                ),
                            }
                        }
                        false => {
                            log_if_verbose(self.verbose, "Does not pass any of the checks.");
                            CheckResult {
                                question: None,
                                status: Status::Red,
                                explanation: format!(
                                    "{} no major or minor release",
                                    QUESTION_EXPLANATION_SUFFIX
                                ),
                            }
                        }
                    }
                }
                false => {
                    log_if_verbose(self.verbose, "Does not pass any of the checks.");
                    CheckResult {
                        question: None,
                        status: Status::Red,
                        explanation: format!(
                            "{} no major or minor release",
                            QUESTION_EXPLANATION_SUFFIX
                        ),
                    }
                }
            },
        }
    }

    ///To check the documentation, wmt checks the following :
    /// 1. Does the crate have either a README or a doc.rs page??
    /// 2. If it has a doc.rs page, was the build successful?
    /// 3. If it was, does it contain any information about the doc coverage?
    fn check_documentation(&self, cargo_crate: &Crate) -> CheckResult {
        let maybe_doc_link = &cargo_crate
            .documentation
            .as_deref()
            .unwrap_or_else(|| cargo_crate.source_url.as_deref().unwrap());
        let mut status = Status::Green;
        let mut explanation = String::new();
        let doc = DocService::new(cargo_crate.name.as_str(), maybe_doc_link);

        log_if_verbose(
            self.verbose,
            format!("Crate doc is hosted on {}", doc.doc_source.to_string()).as_str(),
        );

        if doc.check_doc_page_exists() {
            match doc.doc_source {
                DocSource::GithubReadMe => {
                    log_if_verbose(self.verbose, "Crate has a README.");
                    explanation.push_str("README exists. Can't guarantee the coverage.")
                }
                DocSource::RustDoc => {
                    if doc.has_successful_build() {
                        log_if_verbose(
                            self.verbose,
                            "Crate has a doc.rs page. Will check build status and coverage",
                        );
                        let doc_coverage_score = doc.get_rust_doc_coverage_score();
                        match doc_coverage_score {
                            Ok(value) => {
                                explanation.push_str(
                                    format!("{}% of the crate is documented", value).as_str(),
                                );

                                match value.cmp(&50) {
                                    Ordering::Less => {
                                        if value < 10 {
                                            log_if_verbose(
                                                self.verbose,
                                                "Crate has < 10% doc coverage",
                                            );
                                            status = Status::Red
                                        } else {
                                            log_if_verbose(
                                                self.verbose,
                                                "Crate has <50% but > 10% doc coverage",
                                            );
                                            status = Status::Yellow
                                        }
                                    }
                                    _ => {
                                        log_if_verbose(self.verbose, "Crate has >50% doc coverage");
                                        status = Status::Green
                                    }
                                }
                            }
                            Err(_) => {
                                log_if_verbose(
                                    self.verbose,
                                    "Crate has no doc coverage information",
                                );
                                status = Status::Yellow;
                                explanation.push_str("The crate is documented on doc.rs but there's no doc coverage information")
                            }
                        }
                    } else {
                        log_if_verbose(self.verbose, "Crate has a failing build");
                        status = Status::Red;
                        explanation.push_str("The crate has a failing documentation build")
                    }
                }
            }
        } else {
            status = Status::Red;
            explanation.push_str("The crate has no README or doc.rs page")
        }

        CheckResult {
            question: None,
            status,
            explanation,
        }
    }

    ///To check the changelog, wmt checks the following :
    /// 1. Does the crate source(github) have either a CHANGELOG.md or a release page with a changelog note?
    fn check_changelog(&self, cargo_crate: &Crate) -> CheckResult {
        let mut status = Status::Green;
        let mut explanation = String::new();

        let source_url = cargo_crate.source_url.as_deref().unwrap().to_string();
        let github_service = GithubService::new(source_url);
        let release_changelog_exists = github_service.release_changelog_exists();

        match github_service.changelog_note_exists() {
            true => explanation.push_str("The crate release has a CHANGELOG.md note"),
            false => match release_changelog_exists {
                Ok(_) => explanation.push_str("The crate has a release changelog"),
                Err(_) => {
                    status = Status::Red;
                    explanation.push_str("The crate has no release changelog or CHANGELOG.md")
                }
            },
        }

        CheckResult {
            question: None,
            status,
            explanation,
        }
    }

    fn check_tests(&self, cargo_crate: &Crate) -> CheckResult {
        let source_url = cargo_crate.source_url.as_deref().unwrap().to_string();
        let github_service = GithubService::new(source_url);

        let test_files = github_service.get_test_files();

        let mut explanation = String::new();
        let mut status = Status::Green;

        match test_files.is_empty() {
            true => {
                status = Status::Red;
                explanation.push_str("No test files found");
            }
            false => explanation.push_str("Test files found. Can't guarantee coverage"),
        }

        CheckResult {
            question: None,
            status,
            explanation,
        }
    }

    fn check_bug_response(&self, cargo_crate: &Crate) -> CheckResult {
        let source_url = cargo_crate.source_url.as_deref().unwrap().to_string();
        let github_service = GithubService::new(source_url);

        log_if_verbose(self.verbose, "Checking open bug");

        let open_bugs = github_service.get_bugs(State::Open).into_iter();

        let mut explanation = String::new();
        let mut status = Status::Green;

        match open_bugs.len() == 0 {
            true => {
                log_if_verbose(self.verbose, "No open bug found");
                explanation.push_str("There are no open bugs");
            }
            false => {
                log_if_verbose(self.verbose, "Checking comments on open bug");
                let open_bugs_with_comments = open_bugs
                    .filter(|bug| bug.comments > 1)
                    .collect::<Vec<Issue>>();
                match open_bugs_with_comments.is_empty() {
                    true => {
                        status = Status::Red;
                        explanation.push_str(&format!(
                            "There are {} open bugs with no response from the maintainer(s)",
                            open_bugs_with_comments.len()
                        ))
                    }
                    false => {
                        log_if_verbose(self.verbose, "All open bugs have responses");
                        status = Status::Green;
                        explanation.push_str("The maintainer(s) have responded to all open bugs");
                    }
                }
            }
        }

        CheckResult {
            question: None,
            status,
            explanation,
        }
    }

    fn check_runs_against_latest_language(&self, cargo_crate: &Crate) -> CheckResult {
        let mut check_tests = self.check_tests(cargo_crate);

        if let Status::Green = check_tests.status {
            check_tests.explanation = format!(
                "{} or if they run with latest version",
                check_tests.explanation
            )
        }

        check_tests
    }

    fn check_continuous_integration(&self, cargo_crate: &Crate) -> CheckResult {
        let source_url = cargo_crate.source_url.as_deref().unwrap().to_string();
        let github_service = GithubService::new(source_url);

        let mut explanation = String::new();
        let mut status = Status::Green;

        let workflows = github_service.get_workflows();
        let count = workflows.len();

        match count > 0 {
            true => {
                explanation.push_str(&format!("The crate has {} Github workflow(s)", count));
            }
            false => {
                status = Status::Red;
                explanation.push_str("The crate has no Github workflow");
            }
        }

        CheckResult {
            question: None,
            status,
            explanation,
        }
    }

    fn check_ci_status(&self, cargo_crate: &Crate) -> CheckResult {
        let source_url = cargo_crate.source_url.as_deref().unwrap().to_string();
        let github_service = GithubService::new(source_url);

        let mut explanation = String::new();
        let mut status = Status::Green;

        let workflows = github_service.get_workflows().into_iter();
        let count = workflows.len();

        match count > 0 {
            true => {
                let failing_workflows_count = self.get_failing_workflows(github_service, workflows);

                match failing_workflows_count > 0 {
                    true => explanation.push_str(&format!(
                        "{} of the workflows failed",
                        failing_workflows_count
                    )),
                    false => {
                        explanation.push_str("No failing workflow found");
                    }
                }
            }
            false => {
                status = Status::Red;
                explanation.push_str("The crate has no Github workflow");
            }
        }

        CheckResult {
            question: None,
            status,
            explanation,
        }
    }

    fn get_failing_workflows(
        &self,
        github_service: GithubService,
        workflows: IntoIter<WorkFlow>,
    ) -> u64 {
        let mut failing_workflows = HashMap::new();
        for workflow in workflows {
            let failing_runs = github_service.get_workflow_runs(workflow.id.to_string());
            failing_workflows.insert(workflow.id.to_string(), failing_runs.len() as u64);
        }

        let fw = failing_workflows
            .iter()
            .filter(|&(_, value)| value > &0u64)
            .into_iter()
            .count();

        fw as u64
    }

    fn check_usage(&self, cargo_crate: &Crate) -> CheckResult {
        let source_url = cargo_crate.source_url.as_deref().unwrap().to_string();
        let github_service = GithubService::new(source_url);

        let mut explanation = String::new();
        let mut status = Status::Green;

        let commits = github_service.get_latest_commits();

        if commits.iter().peekable().peek().is_some() {
            explanation.push_str("There have been commits this year");
        } else {
            status = Status::Red;
            explanation.push_str("There have been no commits this year");
        }

        CheckResult {
            question: None,
            status,
            explanation,
        }
    }

    fn check_latest_commits(&self, cargo_crate: &Crate) -> CheckResult {
        let source_url = cargo_crate.source_url.as_deref().unwrap().to_string();
        let github_service = GithubService::new(source_url);

        let mut status = Status::Green;

        let today = Utc::now();

        let last_commit = github_service.get_latest_commits().unwrap();
        let commit_date = last_commit.first().unwrap().commit.author.date;
        let date_difference = (today - commit_date).num_days();

        if date_difference < 365 {
            status = Status::Red;
        }

        let explanation = format!("The last commit was {} day(s) ago", date_difference);

        CheckResult {
            question: None,
            status,
            explanation,
        }
    }

    fn check_latest_release(&self, cargo_crate: &Crate) -> CheckResult {
        let source_url = cargo_crate.source_url.as_deref().unwrap().to_string();
        let github_service = GithubService::new(source_url);
        let mut status = Status::Green;
        let mut explanation = String::new();

        let latest_release = github_service.get_latest_release();
        match latest_release {
            Ok(release) => {
                let today = Utc::now();
                let date_difference = (today - release.created_at.unwrap()).num_days();
                if date_difference > 365 {
                    status = Status::Yellow;
                    explanation.push_str("The last release was over a year ago");
                } else {
                    explanation.push_str(&format!(
                        "The last release was {} day(s) ago",
                        date_difference
                    ));
                }
            }
            Err(_) => {
                status = Status::Red;
                explanation.push_str("There has no been release.")
            }
        };

        CheckResult {
            question: None,
            status,
            explanation,
        }
    }
}
