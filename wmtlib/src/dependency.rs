use crate::constants::MISSING_FIELD_PLACEHOLDER;
use crate::services::CratesService;
use cargo_toml::{DependencyDetail, DepsSet, Manifest};
use chrono::{DateTime, Utc};
use std::option::Option;

#[derive(Debug)]
pub struct DependencyVersion {
    pub local: Option<String>,
    pub remote: Option<String>,
}

#[derive(Debug)]
pub struct Dependency {
    pub name: String,
    pub source_url: Option<String>,
    pub description: Option<String>,
    pub documentation: Option<String>,
    pub homepage: Option<String>,
    pub version: Option<DependencyVersion>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub downloads: u64,
}

impl Dependency {
    pub fn from_source(github_url: String) -> Self {
        Dependency {
            name: "".to_string(),
            source_url: Option::from(github_url),
            description: None,
            documentation: None,
            homepage: None,
            version: None,
            created_at: None,
            updated_at: None,
            downloads: 0,
        }
    }

    pub fn from_manifest(manifest_file: String) -> Vec<Self> {
        let mut dependencies = Vec::new();
        let manifest_content = Self::extract_dependencies_from_manifest(manifest_file);

        for dep in manifest_content {
            let dependency_without_version = Self::dependency_without_version();

            let local_version = &dep
                .1
                .detail()
                .unwrap_or(&dependency_without_version)
                .version;
            let full_dependency = Dependency::from_name(dep.0, local_version.to_owned());
            dependencies.push(full_dependency)
        }

        dependencies
    }

    fn dependency_without_version() -> DependencyDetail {
        DependencyDetail {
            version: Some(MISSING_FIELD_PLACEHOLDER.to_string()),
            registry: None,
            registry_index: None,
            path: None,
            git: None,
            branch: None,
            tag: None,
            rev: None,
            features: vec![],
            optional: false,
            default_features: None,
            package: None,
        }
    }

    pub fn from_name(name: String, local_version: Option<String>) -> Self {
        let crate_client = CratesService::new();
        let crate_info = crate_client.get_crate(name.as_str()).unwrap().crate_data;

        Dependency {
            name,
            source_url: crate_info.repository,
            description: crate_info.description,
            documentation: crate_info.documentation,
            homepage: crate_info.homepage,
            version: Option::from(DependencyVersion {
                remote: Option::from(crate_info.max_version),
                local: local_version,
            }),
            created_at: Option::from(crate_info.created_at),
            updated_at: Option::from(crate_info.updated_at),
            downloads: crate_info.downloads,
        }
    }

    fn extract_dependencies_from_manifest(path: String) -> DepsSet {
        Manifest::from_path(path).unwrap().dependencies
    }
}
