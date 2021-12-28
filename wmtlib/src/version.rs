#[derive(Debug)]
pub struct Version {
    pub id: String,
    pub minor: u64,
    pub major: u64,
    pub patch: u64,
}

impl Version {
    pub fn from_version_text(text: &str) -> Self {
        let semver: Vec<&str> = text.split('.').collect();
        Version {
            id: text.to_string(),
            major: semver.get(0).unwrap().parse::<u64>().unwrap(),
            minor: semver.get(1).unwrap().parse::<u64>().unwrap(),
            patch: semver.get(2).unwrap().parse::<u64>().unwrap(),
        }
    }

    pub fn at_least_one_major_release(&self) -> bool {
        self.major.gt(&0)
    }

    pub fn at_least_one_minor_release(&self) -> bool {
        self.minor.ge(&1)
    }
}
