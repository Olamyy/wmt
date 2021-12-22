use crate::constants::{
    MAX_DOWNLOAD_FOR_MINOR_VERSION, MISSING_FIELD_PLACEHOLDER, QUESTION_EXPLANATION_SUFFIX,
};
use crate::dependency::Dependency;
use crate::questions::CheckNames;
use crate::services::{DocService, DocSource, GithubService};
use crate::utils::log_if_verbose;
use crate::version::Version;
use crate::{CommandResult, Question, Questions};
use serde::Serialize;
use std::cmp::Ordering;

/// Represents a dependency
#[derive(Debug)]
pub struct DependencyCheck {
    pub dependencies: Vec<Dependency>,
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
}

impl Status {
    pub fn value(&self) -> String {
        match *self {
            Status::Green => String::from("GREEN"),
            Status::Yellow => String::from("Yellow"),
            Status::Red => String::from("RED"),
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
}

impl DependencyCheck {
    #[tracing::instrument]
    pub fn new(deps: Vec<String>, verbose: bool) -> Self {
        log_if_verbose(verbose, "Checking dependency source");
        let mut dependencies_to_check = Vec::new();
        for dep in deps {
            match dep.ends_with(".toml") {
                true => {
                    log_if_verbose(verbose, "Found manifest path. Extracting");
                    let dependencies_from_manifest = Dependency::from_manifest(dep);
                    dependencies_to_check.extend(dependencies_from_manifest);
                }
                false => match dep.starts_with("https://") {
                    true => {
                        log_if_verbose(verbose, "Found source url. Querying");
                        let dependency = Dependency::from_source(dep);
                        dependencies_to_check.push(dependency);
                    }
                    false => {
                        log_if_verbose(verbose, "Found crate name. Extracting crate information");
                        let dependency =
                            Dependency::from_name(dep, Some(MISSING_FIELD_PLACEHOLDER.to_string()));
                        dependencies_to_check.push(dependency);
                    }
                },
            }
        }

        DependencyCheck {
            dependencies: dependencies_to_check,
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

        for question_to_check in selected_question {
            let question_results = self.check_question(&question_to_check);
            results.push(question_results);
        }

        results
    }

    fn check_question(&self, question: &Question) -> Vec<CheckResult> {
        let mut check_results = Vec::new();
        for dependency in &self.dependencies {
            let mut check_result = match question.name {
                CheckNames::ProductionReadiness => {
                    log_if_verbose(self.verbose, "Checking for production readiness");
                    self.check_production_readiness(dependency)
                }
                CheckNames::Documentation => {
                    log_if_verbose(self.verbose, "Checking for documentation");
                    self.check_documentation(dependency)
                }
                CheckNames::Changelog => {
                    log_if_verbose(self.verbose, "Checking for changelog");
                    self.check_changelog(dependency)
                }
                CheckNames::Tests => {
                    log_if_verbose(self.verbose, "Checking for production readiness");
                    self.check_production_readiness(dependency)
                }
                CheckNames::BugReportResponse => {
                    log_if_verbose(self.verbose, "Checking for production readiness");
                    self.check_production_readiness(dependency)
                }
                CheckNames::TestsRunAgainstLatestLanguageVersion => {
                    log_if_verbose(self.verbose, "Checking for production readiness");
                    self.check_production_readiness(dependency)
                }
                CheckNames::TestsRunAgainstLatestIntegrationVersion => {
                    log_if_verbose(self.verbose, "Checking for production readiness");
                    self.check_production_readiness(dependency)
                }
                CheckNames::ContinuousIntegrationConfiguration => {
                    log_if_verbose(self.verbose, "Checking for production readiness");
                    self.check_production_readiness(dependency)
                }
                CheckNames::ContinuousIntegrationPasses => {
                    log_if_verbose(self.verbose, "Checking for production readiness");
                    self.check_production_readiness(dependency)
                }
                CheckNames::Usage => {
                    log_if_verbose(self.verbose, "Checking for production readiness");
                    self.check_production_readiness(dependency)
                }
                CheckNames::LatestCommits => {
                    log_if_verbose(self.verbose, "Checking for production readiness");
                    self.check_production_readiness(dependency)
                }
                CheckNames::LatestRelease => {
                    log_if_verbose(self.verbose, "Checking for production readiness");
                    self.check_production_readiness(dependency)
                }
            };
            let actual_question = &question.question;
            check_result.question = Option::from(actual_question.to_string());
            check_results.push(check_result);
        }

        check_results
    }

    ///To check the production readiness, wmt checks the following :
    /// 1. Does the crate have at least one major release?
    /// 2. Does the crate have at least 2 minor releases and a significant number of downloads?
    fn check_production_readiness(&self, dependency: &Dependency) -> CheckResult {
        let dependency_version = &dependency.version.as_ref().unwrap();
        let remote_dependency_version = dependency_version.remote.as_ref().unwrap();
        let version = Version::from_version_text(remote_dependency_version);
        match version.at_least_one_major_release() {
            true => {
                log_if_verbose(self.verbose, "Passes AT_LEAST_ONE_MAJOR_RELEASE check");
                CheckResult {
                    question: None,
                    status: Status::Green,
                    explanation: format!(
                        "{} at least one major release.",
                        QUESTION_EXPLANATION_SUFFIX
                    ),
                }
            }
            false => match version.at_least_one_minor_release() {
                true => {
                    let downloads = &dependency.downloads;
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

    fn check_documentation(&self, dependency: &Dependency) -> CheckResult {
        let maybe_doc_link = &dependency
            .documentation
            .as_deref()
            .unwrap_or_else(|| dependency.source_url.as_deref().unwrap());
        let mut status = Status::Green;
        let mut explanation = String::new();
        let doc = DocService::new(dependency.name.as_str(), maybe_doc_link);

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
                                    format!("{}% of the crate is documented.", value).as_str(),
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
                                explanation.push_str("The crate is documented on doc.rs however there's no doc coverage information.")
                            }
                        }
                    } else {
                        log_if_verbose(self.verbose, "Crate has a failing build");
                        status = Status::Red;
                        explanation.push_str("The crate has a failing documentation build.")
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

    fn check_changelog(&self, dependency: &Dependency) -> CheckResult {
        let mut status = Status::Green;
        let mut explanation = String::new();

        let source_url = dependency.source_url.as_deref().unwrap().to_string();
        let github_service = GithubService::new(source_url);
        let release_changelog_exists = github_service.release_changelog_exists();

        match github_service.changelog_note_exists() {
            true => explanation.push_str("The crate release has a CHANGELOG.md note."),
            false => match release_changelog_exists {
                None => {
                    status = Status::Red;
                    explanation.push_str("The crate has no release changelog or CHANGELOG.md")
                }
                Some(_) => explanation.push_str("The crate has a release changelog"),
            },
        }

        CheckResult {
            question: None,
            status,
            explanation,
        }
    }
}
