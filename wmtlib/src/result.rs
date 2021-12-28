use crate::check::CheckResult;
use crate::Question;
use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::presets::UTF8_FULL;
use comfy_table::{Cells, ContentArrangement, Table};

#[derive(Debug)]
pub struct CommandResult {
    pub as_json: bool,
    pub headers: Vec<String>,
}

impl CommandResult {
    pub fn new(as_json: bool, headers: Vec<String>) -> Self {
        CommandResult { as_json, headers }
    }

    pub fn display_question_result(&self, data: Vec<Question>) {
        match self.as_json {
            true => {
                println!("{}", serde_json::to_string_pretty(&data).unwrap());
            }
            false => {
                let question_table = TableResult::from_questions(data);
                question_table.show()
            }
        }
    }

    pub fn display_checks_result(&self, data: Vec<Vec<CheckResult>>) {
        match self.as_json {
            true => {
                println!("{}", serde_json::to_string_pretty(&data).unwrap());
            }
            false => {
                let results_table = TableResult::from_checks(data);
                results_table.show()
            }
        }
    }
}

#[derive(Debug)]
pub struct TableResult {
    table: Table,
}

impl TableResult {
    pub fn new(headers: Vec<String>) -> TableResult {
        let mut table = Table::new();
        table
            .set_header(&headers)
            .load_preset(UTF8_FULL)
            .apply_modifier(UTF8_ROUND_CORNERS)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_table_width(150)
            .trim_fmt();

        TableResult { table }
    }

    pub fn from_questions(data: Vec<Question>) -> TableResult {
        let question_headers = vec![
            "Number".to_string(),
            "Question".to_string(),
            "Explanation".to_string(),
        ];
        let mut table = Self::new(question_headers);

        for entry in data {
            let cells: Cells = entry.to_vec().into();
            table.table.add_row(cells);
        }

        table
    }

    pub fn from_checks(data: Vec<Vec<CheckResult>>) -> TableResult {
        let headers = vec![
            String::from("Question"),
            String::from("Status"),
            String::from("Explanation"),
        ];

        let mut table = Self::new(headers);
        for entry in data {
            for result in entry {
                let cells: Cells = result.to_vec().into();
                table.table.add_row(cells);
            }
        }

        table
    }

    pub fn show(&self) {
        println!("{}", self.table)
    }
}
