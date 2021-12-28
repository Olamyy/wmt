use std::fmt::{Display, Formatter};
use std::num::ParseIntError;
use std::ops::Deref;

use select::document::Document;
use select::predicate::Class;

use crate::constants::RUST_DOC_URL;
use crate::github::{GithubService, RepoMetrics};
use crate::HTTPClient;

pub enum DocSource {
    GithubReadMe,
    RustDoc,
}

impl Display for DocSource {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match *self {
            DocSource::GithubReadMe => {
                write!(f, "Github")
            }
            DocSource::RustDoc => {
                write!(f, "Rust Doc")
            }
        }
    }
}

pub struct DocService {
    pub crate_name: String,
    pub doc_source: DocSource,
    pub doc_url: String,
    pub http_client: HTTPClient,
}

impl DocService {
    pub fn new(crate_name: &str, crate_documentation: &str) -> Self {
        match crate_documentation.contains("github.com") {
            true => DocService {
                crate_name: crate_name.to_string(),
                doc_source: DocSource::GithubReadMe,
                doc_url: crate_documentation.to_string(),
                http_client: HTTPClient::new(),
            },
            false => {
                let doc_url = format!("{}/{}", RUST_DOC_URL, crate_name);
                DocService {
                    crate_name: crate_name.to_string(),
                    doc_source: DocSource::RustDoc,
                    doc_url,
                    http_client: HTTPClient::new(),
                }
            }
        }
    }

    pub fn has_successful_build(&self) -> bool {
        let document = self.get_doc_page();
        document.select(Class("warning")).count() == 0
    }

    fn get_doc_page(&self) -> Document {
        let response = reqwest::blocking::get(&self.doc_url).unwrap();
        Document::from_read(response).unwrap()
    }

    pub fn get_rust_doc_coverage_score(&self) -> Result<u64, ParseIntError> {
        let document = self.get_doc_page();
        let mut explanation = String::new();
        document
            .select(Class("pure-menu-link"))
            .filter(|n| n.text().contains('%'))
            .for_each(|n| explanation.push_str(self.clean_doc_coverage_text(n.text()).as_str()));

        explanation.parse::<u64>()
    }

    fn clean_doc_coverage_text(&self, text: String) -> String {
        let mut result = String::new();
        text.split("      ")
            .filter(|n| !n.is_empty() && n.deref() != "\n" && n.contains('%'))
            .for_each(|n| result.push_str(n.trim()));

        result.replace("%", "")
    }

    pub fn check_doc_page_exists(&self) -> bool {
        match self.doc_source {
            DocSource::GithubReadMe => {
                let github_service = GithubService::new(self.doc_url.to_string());
                let repo_metric: RepoMetrics = github_service.get_repo_metrics().unwrap();
                repo_metric.files.get("readme").is_some()
            }
            DocSource::RustDoc => self
                .http_client
                .get(&self.doc_url)
                .unwrap()
                .status()
                .is_success(),
        }
    }
}
