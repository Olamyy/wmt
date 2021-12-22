use anyhow::Result;
use crates_io_api::{CrateResponse, Error, SyncClient};
use octocrab::models::repos::Release;
use octocrab::models::Repository;
use select::document::Document;
use select::predicate::Class;
use std::fmt::{Display, Formatter};
use std::num::ParseIntError;
use std::ops::Deref;

use crate::constants::{CRATES_API_RPS, CRATES_API_USER_AGENT, RUST_DOC_URL};

pub struct CratesService {
    client: SyncClient,
}

impl CratesService {
    pub fn new() -> Self {
        let client = SyncClient::new(
            CRATES_API_USER_AGENT,
            std::time::Duration::from_millis(CRATES_API_RPS),
        )
        .unwrap();
        CratesService { client }
    }

    pub fn get_crate(&self, crate_name: &str) -> Result<CrateResponse, Error> {
        Ok(self.client.get_crate(crate_name).unwrap())
    }
}

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
}

impl DocService {
    pub fn new(crate_name: &str, crate_documentation: &str) -> Self {
        match crate_documentation.contains("github.com") {
            true => DocService {
                crate_name: crate_name.to_string(),
                doc_source: DocSource::GithubReadMe,
                doc_url: crate_documentation.to_string(),
            },
            false => {
                let doc_url = format!("{}/{}", RUST_DOC_URL, crate_name);
                DocService {
                    crate_name: crate_name.to_string(),
                    doc_source: DocSource::RustDoc,
                    doc_url,
                }
            }
        }
    }

    pub fn has_successful_build(&self) -> bool {
        let document = self.get_doc_page();
        let warnings = document.select(Class("warning"));
        return warnings.peekable().peek().is_some();
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
                let readme_url = github_service.build_file_url("README.md");
                reqwest::blocking::get(&readme_url)
                    .unwrap()
                    .status()
                    .is_success()
            }
            DocSource::RustDoc => reqwest::blocking::get(&self.doc_url)
                .unwrap()
                .status()
                .is_success(),
        }
    }
}

#[derive(Debug)]
pub struct GithubService {
    pub url: String,
    pub repo: String,
    pub owner: String,
}

impl GithubService {
    pub fn new(url: String) -> Self {
        let split_url = url.split("github.com").collect::<Vec<&str>>();
        let url_parts = split_url.get(1).unwrap().to_string();
        let url_parts = url_parts
            .split('/')
            .filter(|n| !n.is_empty())
            .collect::<Vec<&str>>();
        let owner = url_parts.get(0).unwrap().to_string();
        let repo = url_parts.get(1).unwrap().to_string();

        GithubService { url, owner, repo }
    }

    #[tokio::main(flavor = "current_thread")]
    pub async fn get_repo(&self) -> Repository {
        return octocrab::instance()
            .repos(&self.owner, &self.repo)
            .get()
            .await
            .unwrap();
    }

    pub fn build_file_url(&self, file: &str) -> String {
        let default_branch = self.get_repo().default_branch.unwrap();
        format!("{}/blob/{}/{}", self.url, default_branch, file)
    }

    #[tokio::main(flavor = "current_thread")]
    pub async fn get_latest_release(&self) -> Release {
        octocrab::instance()
            .repos(&self.owner, &self.repo)
            .releases()
            .get_latest()
            .await
            .unwrap()
    }

    pub fn changelog_note_exists(&self) -> bool {
        reqwest::blocking::get(&self.build_file_url("CHANGELOG.md"))
            .unwrap()
            .status()
            .is_success()
    }

    pub fn release_changelog_exists(&self) -> Option<String> {
        let latest_release = self.get_latest_release();
        latest_release.body
    }
}
