use crate::board::{Character, UnsupportedChar};
use std::fmt;
use std::str::FromStr;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Model {
    Flagship,
    Note,
}

impl Model {
    const ALL: [Model; 2] = [Model::Flagship, Model::Note];

    pub fn rows(self) -> usize {
        match self {
            Model::Flagship => 6,
            Model::Note => 3,
        }
    }

    pub fn columns(self) -> usize {
        match self {
            Model::Flagship => 22,
            Model::Note => 15,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Model::Flagship => "flagship",
            Model::Note => "note",
        }
    }
}

impl fmt::Display for Model {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Model {
    type Err = UnknownModel;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Model::ALL
            .into_iter()
            .find(|model| model.as_str() == s)
            .ok_or_else(|| UnknownModel {
                value: s.to_string(),
            })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error(
    "Unknown Vestaboard model '{}', expected one of: flagship, note",
    value
)]
pub struct UnknownModel {
    pub value: String,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(try_from = "Vec<Vec<Character>>")]
pub struct Layout {
    model: Model,
    rows: Vec<Vec<Character>>,
}

impl Layout {
    pub fn new(rows: Vec<Vec<Character>>) -> Result<Self, InvalidSize> {
        let model = Model::ALL.into_iter().find(|model| {
            rows.len() == model.rows() && rows.iter().all(|row| row.len() == model.columns())
        });

        match model {
            Some(model) => Ok(Self { model, rows }),
            None => Err(InvalidSize {
                rows: rows.len(),
                columns: rows.iter().map(Vec::len).collect(),
            }),
        }
    }

    pub fn blank(model: Model) -> Self {
        Self {
            model,
            rows: vec![vec![Character::Blank; model.columns()]; model.rows()],
        }
    }

    pub fn from_text(model: Model, text: &str) -> Result<Self, TextError> {
        let lines = text.lines().collect::<Vec<_>>();

        if lines.len() > model.rows() {
            return Err(TextError::TooManyLines {
                lines: lines.len(),
                max: model.rows(),
            });
        }

        let mut layout = Self::blank(model);

        for (number, (row, line)) in layout.rows.iter_mut().zip(lines).enumerate() {
            let length = line.chars().count();

            if length > model.columns() {
                return Err(TextError::LineTooLong {
                    line: number + 1,
                    length,
                    max: model.columns(),
                });
            }

            for (cell, value) in row.iter_mut().zip(line.chars()) {
                *cell = Character::try_from(value)?;
            }
        }

        Ok(layout)
    }

    pub fn model(&self) -> Model {
        self.model
    }

    pub fn rows(&self) -> &[Vec<Character>] {
        &self.rows
    }

    pub fn into_rows(self) -> Vec<Vec<Character>> {
        self.rows
    }

    pub fn contains(&self, character: Character) -> bool {
        self.rows.iter().flatten().any(|cell| *cell == character)
    }
}

impl TryFrom<Vec<Vec<Character>>> for Layout {
    type Error = InvalidSize;

    fn try_from(rows: Vec<Vec<Character>>) -> Result<Self, Self::Error> {
        Self::new(rows)
    }
}

impl serde::Serialize for Layout {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.rows.serialize(serializer)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error(
    "Expected 6 rows of 22 characters (Flagship), or 3 rows of 15 characters (Note), but got '{}' rows with lengths '{:?}'",
    rows,
    columns
)]
pub struct InvalidSize {
    pub rows: usize,
    pub columns: Vec<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum TextError {
    #[error(
        "The text has '{}' lines, but the board only has '{}' rows",
        lines,
        max
    )]
    TooManyLines { lines: usize, max: usize },
    #[error(
        "Line '{}' has '{}' characters, but the board only has '{}' columns",
        line,
        length,
        max
    )]
    LineTooLong {
        line: usize,
        length: usize,
        max: usize,
    },
    #[error(transparent)]
    UnsupportedChar(#[from] UnsupportedChar),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid(rows: usize, columns: usize) -> Vec<Vec<Character>> {
        vec![vec![Character::Blank; columns]; rows]
    }

    #[yare::parameterized(
        flagship = { 6, 22, Model::Flagship },
        note = { 3, 15, Model::Note },
    )]
    fn infers_model_from_size(rows: usize, columns: usize, expected: Model) {
        assert_eq!(Layout::new(grid(rows, columns)).unwrap().model(), expected);
    }

    #[yare::parameterized(
        empty = { 0, 0 },
        flagship_too_few_rows = { 5, 22 },
        flagship_too_many_columns = { 6, 23 },
        note_too_few_columns = { 3, 14 },
        note_array = { 3, 30 },
        mixed = { 3, 22 },
    )]
    fn rejects_invalid_sizes(rows: usize, columns: usize) {
        assert_eq!(
            Layout::new(grid(rows, columns)),
            Err(InvalidSize {
                rows,
                columns: vec![columns; rows],
            })
        );
    }

    #[test]
    fn rejects_ragged_rows() {
        let mut rows = grid(3, 15);
        rows[1].pop();

        assert_eq!(Layout::new(rows).unwrap_err().columns, vec![15, 14, 15]);
    }

    #[test]
    fn from_text() {
        let layout = Layout::from_text(Model::Note, "Hi!\n\n°").unwrap();
        let mut expected = grid(3, 15);
        expected[0][0] = Character::H;
        expected[0][1] = Character::I;
        expected[0][2] = Character::ExclamationMark;
        expected[2][0] = Character::Degree;

        assert_eq!(layout.model(), Model::Note);
        assert_eq!(layout.rows(), expected);
    }

    #[test]
    fn from_empty_text() {
        assert_eq!(
            Layout::from_text(Model::Flagship, "").unwrap(),
            Layout::blank(Model::Flagship)
        );
    }

    #[test]
    fn from_text_with_full_rows() {
        let line = "X".repeat(15);
        let text = [line.as_str(); 3].join("\n");

        let layout = Layout::from_text(Model::Note, &text).unwrap();

        assert!(!layout.contains(Character::Blank));
    }

    #[yare::parameterized(
        too_many_lines = { "a\nb\nc\nd", TextError::TooManyLines { lines: 4, max: 3 } },
        line_too_long = { "a\nabcdefghijklmnop", TextError::LineTooLong { line: 2, length: 16, max: 15 } },
        unsupported_char = { "a_b", TextError::UnsupportedChar(UnsupportedChar { value: '_' }) },
    )]
    fn rejects_text(text: &str, expected: TextError) {
        assert_eq!(Layout::from_text(Model::Note, text), Err(expected));
    }

    #[test]
    fn serde_as_rows_of_codes() {
        let layout = Layout::from_text(Model::Note, "A").unwrap();
        let json = serde_json::to_value(&layout).unwrap();

        assert_eq!(json[0][0], 1);
        assert_eq!(json[0][1], 0);
        assert_eq!(serde_json::from_value::<Layout>(json).unwrap(), layout);
    }

    #[test]
    fn deserialize_rejects_invalid_size() {
        assert!(serde_json::from_str::<Layout>("[[0, 0], [0, 0]]").is_err());
    }

    #[yare::parameterized(
        flagship = { "flagship", Model::Flagship },
        note = { "note", Model::Note },
    )]
    fn parse_model(value: &str, expected: Model) {
        assert_eq!(value.parse::<Model>(), Ok(expected));
        assert_eq!(expected.to_string(), value);
    }

    #[test]
    fn unknown_model() {
        assert!("Flagship".parse::<Model>().is_err());
    }
}
