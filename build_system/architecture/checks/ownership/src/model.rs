use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub enum Expression {
    Add(Vec<Expression>),
    Call(CallExpression),
    List(Vec<Expression>),
    Name(String),
    String(String),
    Unknown,
}

#[derive(Clone, Debug)]
pub struct CallExpression {
    pub name: String,
    pub positional: Vec<Expression>,
    pub attributes: BTreeMap<String, Expression>,
}

#[derive(Clone, Debug)]
pub struct Assignment {
    pub line: usize,
    pub expression: Expression,
}

#[derive(Clone, Debug)]
pub struct RuleCall {
    pub name: String,
    pub line: usize,
    pub attributes: BTreeMap<String, Expression>,
}

#[derive(Clone, Debug)]
pub struct Document {
    pub path: String,
    pub package: String,
    pub assignments: BTreeMap<String, Vec<Assignment>>,
    pub imports: BTreeMap<String, String>,
    pub rules: Vec<RuleCall>,
    pub rules_unresolved: bool,
    pub calls: Vec<CallSite>,
}

#[derive(Clone, Debug)]
pub struct CallSite {
    pub name: String,
    pub canonical: String,
    pub line: usize,
    pub top_level: bool,
    pub qualified: bool,
    pub definition: bool,
    pub depset_receiver: bool,
}

#[derive(Clone, Debug, Default)]
pub struct Evaluation {
    pub labels: Vec<String>,
    pub paths: Vec<String>,
    pub unresolved: bool,
}

impl Evaluation {
    pub fn extend(&mut self, other: Self) {
        self.labels.extend(other.labels);
        self.paths.extend(other.paths);
        self.unresolved |= other.unresolved;
    }

    pub fn normalize(&mut self) {
        self.labels.sort();
        self.labels.dedup();
        self.paths.sort();
        self.paths.dedup();
    }
}

#[derive(Clone, Debug)]
pub struct Target {
    pub id: String,
    pub package: String,
    pub path: String,
    pub line: usize,
    pub sources: Vec<String>,
    pub source_labels: Vec<String>,
    pub sources_unresolved: bool,
}

#[derive(Clone, Debug)]
pub struct FileGroup {
    pub label: String,
    pub name: String,
    pub package: String,
    pub document: String,
    pub line: usize,
    pub expression: Expression,
}
