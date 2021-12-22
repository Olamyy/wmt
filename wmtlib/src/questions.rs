use crate::constants::QUESTIONS_PATH;
use crate::result::CommandResult;
use crate::utils::log_if_verbose;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum CheckNames {
    ProductionReadiness,
    Documentation,
    Changelog,
    Tests,
    BugReportResponse,
    TestsRunAgainstLatestLanguageVersion,
    TestsRunAgainstLatestIntegrationVersion,
    ContinuousIntegrationConfiguration,
    ContinuousIntegrationPasses,
    Usage,
    LatestCommits,
    LatestRelease,
}

/// Represents a question
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Question {
    pub number: String,
    pub question: String,
    pub explanation: String,
    pub name: CheckNames,
}

impl Question {
    pub fn to_vec(&self) -> Vec<String> {
        return vec![
            self.number.to_owned(),
            self.question.to_owned(),
            self.explanation.to_owned(),
        ];
    }
}

pub struct Questions {
    pub verbose: bool,
}

/// Helper struct for serde_json deserialize
#[derive(Debug, Deserialize)]
pub struct DeserializableQuestions {
    pub questions: Vec<Question>,
}

pub fn read_questions_from_file(path: &str) -> Result<DeserializableQuestions, anyhow::Error> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let questions: DeserializableQuestions = serde_json::from_reader(reader)?;

    Ok(questions)
}

impl Questions {
    pub fn new(verbose: bool) -> Self {
        Questions { verbose }
    }
    /// Returns the available questions as numerical strings
    pub fn question_numbers() -> Vec<String> {
        (1..13).map(|x| x.to_string()).collect()
    }

    /// Wrapper for `read_questions_from_file`
    pub fn list(&self) -> DeserializableQuestions {
        log_if_verbose(self.verbose, "Getting available questions");
        read_questions_from_file(QUESTIONS_PATH).unwrap()
    }

    pub fn show_results(&self, json: bool, data: Vec<Question>) {
        let command_result = CommandResult {
            as_json: json,
            headers: vec![
                String::from("Number"),
                String::from("Question"),
                String::from("Explanation"),
            ],
        };
        command_result.display_question_result(data)
    }

    /// Gets a question from the json file.
    pub fn describe(&self, question_number: String) -> Vec<Question> {
        let questions = self.list();
        let question = questions.questions.into_iter();
        log_if_verbose(
            self.verbose,
            format!("Getting question {}", question_number).as_str(),
        );
        question
            .filter(|x| x.number == question_number)
            .collect::<Vec<Question>>()
    }
}
