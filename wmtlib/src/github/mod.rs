use std::collections::HashMap;
use std::env;

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use octocrab::models::issues::Issue;
use octocrab::models::repos::{Content, Release};
use octocrab::models::Repository;
use octocrab::models::workflows::{Run, WorkFlow};
use octocrab::Octocrab;
use octocrab::params::State;
use serde::Deserialize;

use crate::HTTPClient;

#[derive(Debug, Deserialize)]
pub struct RepoMetrics {
    pub health_percentage: u64,
    pub description: Option<String>,
    pub documentation: Option<String>,
    pub files: HashMap<String, Option<HashMap<String, Option<String>>>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub content_reports_enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct Commit {
    pub author: CommitAuthor,
}

#[derive(Debug, Deserialize)]
pub struct CommitAuthor {
    pub date: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct Commits {
    pub commit: Commit,
}

pub struct GithubService {
    pub url: String,
    pub repo: String,
    pub owner: String,
    pub http_client: HTTPClient,
    pub github_client: Octocrab,
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

        let builder = match env::var("GITHUB_TOKEN") {
            Ok(v) => octocrab::Octocrab::builder().personal_token(v).build(),
            Err(_) => octocrab::Octocrab::builder().build(),
        };

        GithubService {
            url,
            owner,
            repo,
            http_client: HTTPClient::new(),
            github_client: builder.unwrap(),
        }
    }

    pub fn get_repo_metrics(&self) -> reqwest::Result<RepoMetrics> {
        self.http_client.get_json(format!(
            "{}/community/profile",
            self.http_client
                .build_github_api_url(&self.owner, &self.repo)
        ))
    }

    #[tokio::main(flavor = "current_thread")]
    pub async fn get_repo(&self) -> Repository {
        return self
            .github_client
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
    pub async fn get_latest_release(&self) -> octocrab::Result<Release> {
        self.github_client
            .repos(&self.owner, &self.repo)
            .releases()
            .get_latest()
            .await
    }

    pub fn changelog_note_exists(&self) -> bool {
        reqwest::blocking::get(&self.build_file_url("CHANGELOG.md"))
            .unwrap()
            .status()
            .is_success()
    }

    pub fn release_changelog_exists(&self) -> Result<Option<String>> {
        let latest_release = self.get_latest_release();
        match latest_release {
            Ok(release) => Ok(release.body),
            Err(_) => Err(anyhow!("Could not get changelog")),
        }
    }

    #[tokio::main(flavor = "current_thread")]
    pub async fn get_repo_content(&self) -> Vec<Content> {
        self.github_client
            .repos(&self.owner, &self.repo)
            .get_content()
            .send()
            .await
            .unwrap()
            .items
    }

    pub fn get_test_files(&self) -> Vec<Content> {
        let repo_content = self.get_repo_content();
        let mut test_dir: Vec<Content> = Vec::new();
        for item in repo_content {
            if item.r#type == "dir" && item.path.contains("test") {
                test_dir.push(item)
            }
        }

        test_dir
    }

    #[tokio::main(flavor = "current_thread")]
    pub async fn get_bugs(&self, status: State) -> Vec<Issue> {
        self.github_client
            .issues(&self.owner, &self.repo)
            .list()
            .state(status)
            .labels(&[String::from("bug")])
            .send()
            .await
            .unwrap()
            .items
    }

    #[tokio::main(flavor = "current_thread")]
    pub async fn get_workflows(&self) -> Vec<WorkFlow> {
        self.github_client
            .workflows(&self.owner, &self.repo)
            .list()
            .send()
            .await
            .unwrap()
            .items
    }

    #[tokio::main(flavor = "current_thread")]
    pub async fn get_workflow_runs(&self, workflow_id: String) -> Vec<Run> {
        self.github_client
            .workflows(&self.owner, &self.repo)
            .list_runs(workflow_id)
            .status("failed")
            .branch("master")
            .exclude_pull_requests(true)
            .per_page(100)
            .page(1u8)
            .send()
            .await
            .unwrap()
            .items
    }

    pub fn get_latest_commits(&self) -> reqwest::Result<Vec<Commits>> {
        self.http_client.get_json(format!(
            "{}/commits?since=2021-01-00T00:00:00Z&per_page=1&page=1",
            self.http_client
                .build_github_api_url(&self.owner, &self.repo)
        ))
    }
}
